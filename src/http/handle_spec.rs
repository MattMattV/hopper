use anyhow::Result;
use axum::{extract::State, response::IntoResponse};
use axum_template::RenderHtml;
use minijinja::context as template_context;

use crate::{
    errors::HopperError,
    http::{context::WebContext, middleware_i18n::Language},
};

pub async fn handle_spec(
    State(web_context): State<WebContext>,
    Language(language): Language,
) -> Result<impl IntoResponse, HopperError> {
    let default_context = template_context! {
        language => language.to_string(),
        canonical_url => format!("https://{}/spec", web_context.external_base),
    };

    let render_template = format!("spec.{}.html", language.to_string().to_lowercase());

    Ok(RenderHtml(
        &render_template,
        web_context.engine.clone(),
        default_context,
    )
    .into_response())
}
