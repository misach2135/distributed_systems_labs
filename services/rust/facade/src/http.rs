//! HTTP layer: router and handlers for the facade service.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use common::{ClientTransaction, MetricsResponse, TransactionResponse, UserResponse};
use tracing::{info, warn};
use uuid::Uuid;

use crate::client::Downstream;
use crate::error::FacadeError;

pub fn router(downstream: Downstream) -> Router {
    Router::new()
        .route("/transaction", post(transaction))
        .route("/user/{user_id}", get(user))
        .route("/accounts", get(accounts))
        .route("/metrics", get(metrics))
        .route("/metrics/reset", post(reset_metrics))
        .route("/health", get(health))
        .with_state(downstream)
}

async fn health() -> &'static str {
    "ok"
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `POST /transaction` — client sends a `{user_Id, +/- amount}` message.
///
/// The facade assigns a `transaction_id` + timestamp, then fans out
/// concurrently to logging-service (`/log`) and counter-service
/// (`/process`), and returns `{transaction_id, balance}`.
async fn transaction(
    State(downstream): State<Downstream>,
    body: String,
) -> Result<Json<TransactionResponse>, FacadeError> {
    let client_tx: ClientTransaction = body
        .parse()
        .map_err(|e: common::ParseTransactionError| FacadeError::BadRequest(e.to_string()))?;

    let transaction_id = Uuid::new_v4();
    let tx = client_tx.into_transaction(transaction_id, now_millis());

    // Fire both downstream calls concurrently and wait for both.
    let (log_res, process_res) = tokio::join!(downstream.log(&tx), downstream.process(&tx));
    log_res?;
    let balance = process_res?.balance;

    info!(%transaction_id, user_id = %tx.user_id, amount = tx.amount, balance, "transaction completed");

    Ok(Json(TransactionResponse {
        transaction_id,
        balance,
    }))
}

/// `GET /user/{user_id}` — balance (counter) + transactions (logging).
async fn user(
    State(downstream): State<Downstream>,
    Path(user_id): Path<String>,
) -> Result<Json<UserResponse>, FacadeError> {
    let (tx_res, balance_res) = tokio::join!(
        downstream.transactions(&user_id),
        downstream.balance(&user_id)
    );
    let transactions = tx_res?;
    let balance = balance_res?.balance;

    Ok(Json(UserResponse {
        balance,
        transactions,
    }))
}

/// `GET /accounts` — all balances from counter-service.
async fn accounts(
    State(downstream): State<Downstream>,
) -> Result<Json<HashMap<String, f64>>, FacadeError> {
    let balances = downstream.balances().await?;
    Ok(Json(balances))
}

/// `GET /metrics` — accumulated latency for logging vs counter calls.
async fn metrics(State(downstream): State<Downstream>) -> Json<MetricsResponse> {
    Json(downstream.metrics().snapshot())
}

/// `POST /metrics/reset` — clear accumulated latency counters.
async fn reset_metrics(State(downstream): State<Downstream>) -> &'static str {
    downstream.metrics().reset();
    warn!("metrics reset");
    "reset"
}
