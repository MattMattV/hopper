use anyhow::Result;
use async_trait::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use std::str::FromStr;
use unic_langid::LanguageIdentifier;

use crate::{http::context::WebContext, i18n::errors::I18nError};

pub(crate) const COOKIE_LANG: &str = "lang";

#[derive(Clone)]
struct AcceptedLanguage {
    value: String,
    quality: f32,
}

impl Eq for AcceptedLanguage {}

impl PartialEq for AcceptedLanguage {
    fn eq(&self, other: &Self) -> bool {
        self.quality == other.quality && self.value.eq(&other.value)
    }
}

impl PartialOrd for AcceptedLanguage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AcceptedLanguage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.quality > other.quality {
            std::cmp::Ordering::Greater
        } else if self.quality < other.quality {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

impl FromStr for AcceptedLanguage {
    type Err = I18nError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut value = s.trim().split(';');
        let (value, quality) = (value.next(), value.next());

        let Some(value) = value else {
            return Err(I18nError::InvalidLanguage());
        };

        if value.is_empty() {
            return Err(I18nError::InvalidLanguage());
        }

        let quality = if let Some(quality) = quality.and_then(|q| q.strip_prefix("q=")) {
            quality.parse::<f32>().unwrap_or(0.0)
        } else {
            1.0
        };

        Ok(AcceptedLanguage {
            value: value.to_string(),
            quality,
        })
    }
}

#[derive(Clone)]
pub(crate) struct Language(pub(crate) LanguageIdentifier);

#[async_trait]
impl<S> FromRequestParts<S> for Language
where
    WebContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, context: &S) -> Result<Self, Self::Rejection> {
        let web_context = WebContext::from_ref(context);

        let cookie_jar = CookieJar::from_headers(&parts.headers);

        if let Some(lang_cookie) = cookie_jar.get(COOKIE_LANG) {
            for value_part in lang_cookie.value().split(',') {
                if let Ok(value) = value_part.parse::<LanguageIdentifier>() {
                    for lang in &web_context.i18n_context.supported_languages {
                        if lang.matches(&value, true, false) {
                            return Ok(Self(lang.clone()));
                        }
                    }
                }
            }
        }

        let accept_languages = &mut parts
            .headers
            .get("accept-language")
            .and_then(|header| header.to_str().ok())
            .map(|header| {
                header
                    .split(',')
                    .filter_map(|lang| lang.parse::<AcceptedLanguage>().ok())
                    .collect::<Vec<AcceptedLanguage>>()
            })
            .unwrap_or_default();

        accept_languages.sort();

        for accept_language in accept_languages {
            if let Ok(value) = accept_language.value.parse::<LanguageIdentifier>() {
                for lang in &web_context.i18n_context.supported_languages {
                    if lang.matches(&value, true, false) {
                        return Ok(Self(lang.clone()));
                    }
                }
            }
        }

        Ok(Self(
            web_context.i18n_context.supported_languages[0].clone(),
        ))
    }
}
