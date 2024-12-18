use axum::extract::FromRef;
use axum_template::engine::Engine;
use moka::future::Cache;
use std::{ops::Deref, sync::Arc};
use unic_langid::LanguageIdentifier;

use crate::{
    cache::{ResolveAtUriResult, ResolveWebHostMetaResult},
    i18n::Locales,
};

#[cfg(feature = "reload")]
use minijinja_autoreload::AutoReloader;

#[cfg(feature = "reload")]
pub type AppEngine = Engine<AutoReloader>;

#[cfg(feature = "embed")]
use minijinja::Environment;

#[cfg(feature = "embed")]
pub type AppEngine = Engine<Environment<'static>>;

pub struct I18nContext {
    pub(crate) supported_languages: Vec<LanguageIdentifier>,
    pub(crate) locales: Locales,
}

pub struct InnerWebContext {
    pub(crate) external_base: String,
    pub(crate) engine: AppEngine,
    pub(crate) http_client: reqwest::Client,
    pub(crate) resolve_webfinger_cache: Cache<String, ResolveWebHostMetaResult>,
    pub(crate) resolve_aturi_cache: Cache<String, ResolveAtUriResult>,
    pub(crate) i18n_context: I18nContext,
}

#[derive(Clone, FromRef)]
pub struct WebContext(pub(crate) Arc<InnerWebContext>);

impl Deref for WebContext {
    type Target = InnerWebContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl WebContext {
    pub fn new(
        external_base: &str,
        engine: AppEngine,
        http_client: &reqwest::Client,
        resolve_webfinger_cache: Cache<String, ResolveWebHostMetaResult>,
        resolve_aturi_cache: Cache<String, ResolveAtUriResult>,
        i18n_context: I18nContext,
    ) -> Self {
        Self(Arc::new(InnerWebContext {
            external_base: external_base.to_string(),
            engine,
            http_client: http_client.clone(),
            resolve_webfinger_cache,
            resolve_aturi_cache,
            i18n_context,
        }))
    }
}

impl I18nContext {
    pub fn new(supported_languages: Vec<LanguageIdentifier>, locales: Locales) -> Self {
        Self {
            supported_languages,
            locales,
        }
    }
}
