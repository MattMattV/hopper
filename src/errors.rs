use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub struct HopperError(pub anyhow::Error);

impl<E> From<E> for HopperError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for HopperError {
    fn into_response(self) -> Response {
        {
            tracing::error!(error = ?self.0, "internal server error");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub(crate) fn expand_error<S: Into<String>>(err: S) -> (String, String) {
    let err: String = err.into();
    let bare = err.split(' ').next().unwrap_or_default().to_string();
    let partial = err.split(':').next().unwrap_or_default().to_string();
    (bare, partial)
}
