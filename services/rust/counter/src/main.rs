mod consumer;
mod db;
mod http;

use std::env;

use tokio::{
    net::TcpListener,
    select,
    signal::{
        ctrl_c,
        unix::{SignalKind, signal},
    },
};
use tracing::info;

use crate::db::BalanceStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://bank:bank@localhost/bank".to_string());
    let store = BalanceStore::connect(&database_url).await?;
    let app = http::router(store.clone());

    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener = TcpListener::bind(&addr).await?;

    info!(%addr, "counter-service listening");

    let kafka_bootstrap =
        env::var("KAFKA_BOOTSTRAP").unwrap_or_else(|_| "localhost:9092".to_string());
    let kafka_topic = env::var("KAFKA_TOPIC").unwrap_or_else(|_| "transactions".to_string());

    // Both stop on Ctrl+C / SIGTERM; the consumer then leaves its group, so a
    // restarted counter gets the partitions at once instead of waiting for
    // the old member's session to time out.
    tokio::try_join!(
        async {
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .map_err(anyhow::Error::from)
        },
        consumer::run(store, &kafka_bootstrap, &kafka_topic),
    )?;
    info!("service stopped");

    Ok(())
}

/// Resolves on Ctrl+C or SIGTERM (`docker stop`).
pub async fn shutdown_signal() {
    let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    select! {
        _ = ctrl_c() => {}
        _ = terminate.recv() => {}
    }
    info!("shutdown signal received");
}
