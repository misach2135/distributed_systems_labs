//! HTTP layer: router and handlers for the logging service.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use common::{Ack, Transaction};
use tracing::info;

use crate::db::LogStore;

pub fn router(store: LogStore) -> Router {
    Router::new()
        .route("/log", post(log_transaction))
        .route("/transactions/{user_id}", get(transactions_by_user))
        .route("/health", get(health))
        .with_state(store)
}

async fn health() -> &'static str {
    "ok"
}

/// `POST /log` — persist a transaction in memory.
async fn log_transaction(
    State(store): State<LogStore>,
    Json(tx): Json<Transaction>,
) -> Json<Ack> {
    info!(
        transaction_id = %tx.transaction_id,
        user_id = %tx.user_id,
        amount = tx.amount,
        timestamp = tx.timestamp,
        "logged transaction"
    );
    store.insert(tx);
    Json(Ack::ok())
}

/// `GET /transactions/{user_id}` — all transactions for a user.
async fn transactions_by_user(
    State(store): State<LogStore>,
    Path(user_id): Path<String>,
) -> Json<Vec<Transaction>> {
    let txs = store.by_user(&user_id);
    info!(%user_id, count = txs.len(), "returning transactions");
    Json(txs)
}
