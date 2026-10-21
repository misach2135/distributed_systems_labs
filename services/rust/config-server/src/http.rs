//! HTTP layer: router and handlers for the config-server.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use common::Registration;
use tracing::info;

use crate::registry::Registry;

pub fn router(registry: Registry) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/services/{name}", get(addresses))
        .route("/health", get(health))
        .with_state(registry)
}

async fn health() -> &'static str {
    "ok"
}

/// `POST /register` — a service instance announces its address.
async fn register(State(registry): State<Registry>, Json(reg): Json<Registration>) -> StatusCode {
    info!(service = %reg.service, address = %reg.address, "registered");
    registry.register(reg.service, reg.address);
    StatusCode::NO_CONTENT
}

/// `GET /services/{name}` — addresses of all instances of a service.
async fn addresses(
    State(registry): State<Registry>,
    Path(name): Path<String>,
) -> Json<Vec<String>> {
    Json(registry.addresses(&name))
}
