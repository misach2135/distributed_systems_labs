//! HTTP layer: router and handlers for the counter service.

use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use common::{BalanceResponse, Transaction};
use tracing::{error, info};

use crate::db::BalanceStore;

pub fn router(store: BalanceStore) -> Router {
    Router::new()
        .route("/process", post(process))
        .route("/balance/{user_id}", get(balance))
        .route("/balances", get(balances))
        .route("/health", get(health))
        .with_state(store)
}

async fn health() -> &'static str {
    "ok"
}

fn db_error(e: sqlx::Error) -> StatusCode {
    error!(error = %e, "database error");
    StatusCode::INTERNAL_SERVER_ERROR
}

/// `POST /process` — apply a transaction and return the updated balance.
async fn process(
    State(store): State<BalanceStore>,
    Json(tx): Json<Transaction>,
) -> Result<Json<BalanceResponse>, StatusCode> {
    let balance = store.apply(&tx.user_id, tx.amount).await.map_err(db_error)?;
    info!(
        user_id = %tx.user_id,
        amount = tx.amount,
        balance,
        "processed transaction"
    );
    Ok(Json(BalanceResponse {
        user_id: tx.user_id,
        balance,
    }))
}

/// `GET /balance/{user_id}` — balance for a single account.
async fn balance(
    State(store): State<BalanceStore>,
    Path(user_id): Path<String>,
) -> Result<Json<BalanceResponse>, StatusCode> {
    let balance = store.get(&user_id).await.map_err(db_error)?;
    Ok(Json(BalanceResponse { user_id, balance }))
}

/// `GET /balances` — every account balance.
async fn balances(
    State(store): State<BalanceStore>,
) -> Result<Json<HashMap<String, f64>>, StatusCode> {
    Ok(Json(store.all().await.map_err(db_error)?))
}
