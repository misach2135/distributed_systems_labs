mod db;
mod http;

use std::env;

use tokio::{net::TcpListener, select, signal::ctrl_c};
use tracing::info;

use crate::db::BalanceStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let store = BalanceStore::new();
    let app = http::router(store);

    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener = TcpListener::bind(&addr).await?;
    info!(%addr, "counter-service listening");

    select! {
        res = axum::serve(listener, app) => {
            res?;
            info!("service stopped");
        }
        _ = ctrl_c() => {
            info!("shutdown signal received");
        }
    }

    Ok(())
}
