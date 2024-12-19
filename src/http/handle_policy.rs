use anyhow::Result;
use axum::{extract::State, response::IntoResponse};
use axum_template::RenderHtml;
use minijinja::context as template_context;

use crate::{
    errors::HopperError,
    http::{context::WebContext, middleware_i18n::Language},
};

pub async fn handle_policy(
    State(web_context): State<WebContext>,
    Language(language): Language,
) -> Result<impl IntoResponse, HopperError> {
    let default_context = template_context! {
        language => language.to_string(),
        canonical_url => format!("https://{}/policy", web_context.external_base),
    };

    Ok(RenderHtml(
        "policy.en-us.html",
        web_context.engine.clone(),
        default_context,
    )
    .into_response())
}
