use std::time::Duration;

#[cfg(feature = "embed")]
use std::convert::Infallible;

use axum::{http::HeaderValue, routing::get, Router};

#[cfg(feature = "embed")]
use axum::{body::Body, extract::Request, response::Response};

use axum_htmx::AutoVaryLayer;
use http::{
    header::{ACCEPT, ACCEPT_LANGUAGE},
    Method,
};
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

#[cfg(feature = "reload")]
use tower_http::services::ServeDir;

use crate::http::{context::WebContext, handle_index::handle_index, handle_spec::handle_spec};

pub fn build_router(web_context: WebContext) -> Router {
    #[cfg(feature = "reload")]
    let serve_dir = ServeDir::new("static");

    #[cfg(feature = "embed")]
    let serve_dir = tower::service_fn(|_request: Request| async {
        Ok::<_, Infallible>(Response::new(Body::empty()))
    });

    Router::new()
        .route("/", get(handle_index))
        .route("/spec", get(handle_spec))
        .nest_service("/static", serve_dir.clone())
        .fallback_service(serve_dir)
        .layer((
            TraceLayer::new_for_http(),
            TimeoutLayer::new(Duration::from_secs(10)),
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(web_context.external_base.parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET])
                .allow_headers([ACCEPT_LANGUAGE, ACCEPT]),
        )
        .layer(AutoVaryLayer)
        .with_state(web_context.clone())
}
