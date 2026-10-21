//! Error type for the facade HTTP layer.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum FacadeError {
    #[error("invalid transaction message: {0}")]
    BadRequest(String),

    #[error("downstream service error: {0}")]
    Downstream(#[from] reqwest::Error),

    #[error("no registered instances of {0}")]
    NoInstances(String),

    #[error("message queue error: {0}")]
    Queue(#[from] krafka::KrafkaError),
}

impl IntoResponse for FacadeError {
    fn into_response(self) -> Response {
        let status = match &self {
            FacadeError::BadRequest(_) => StatusCode::BAD_REQUEST,
            FacadeError::Downstream(_) => StatusCode::BAD_GATEWAY,
            FacadeError::NoInstances(_) | FacadeError::Queue(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}
