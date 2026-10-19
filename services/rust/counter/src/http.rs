//! HTTP layer: router and handlers for the counter service.

use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use common::{BalanceResponse, Transaction};
use tracing::info;

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

/// `POST /process` — apply a transaction and return the updated balance.
async fn process(
    State(store): State<BalanceStore>,
    Json(tx): Json<Transaction>,
) -> Json<BalanceResponse> {
    let balance = store.apply(&tx.user_id, tx.amount);
    info!(
        user_id = %tx.user_id,
        amount = tx.amount,
        balance,
        "processed transaction"
    );
    Json(BalanceResponse {
        user_id: tx.user_id,
        balance,
    })
}

/// `GET /balance/{user_id}` — balance for a single account.
async fn balance(
    State(store): State<BalanceStore>,
    Path(user_id): Path<String>,
) -> Json<BalanceResponse> {
    let balance = store.get(&user_id);
    Json(BalanceResponse { user_id, balance })
}

/// `GET /balances` — every account balance.
async fn balances(State(store): State<BalanceStore>) -> Json<HashMap<String, f64>> {
    Json(store.all())
}
