mod db;
mod http;

use std::env;

use tokio::{net::TcpListener, select, signal::ctrl_c};
use tracing::info;

use crate::db::BalanceStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://bank:bank@localhost/bank".to_string());
    let store = BalanceStore::connect(&database_url).await?;
    let app = http::router(store);

    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener = TcpListener::bind(&addr).await?;

    let config_server_url =
        env::var("CONFIG_SERVER_URL").unwrap_or_else(|_| "http://localhost:8003".to_string());
    let service_url = env::var("SERVICE_URL").unwrap_or_else(|_| format!("http://{addr}"));
    connectors::config_server::register(&config_server_url, "counter", &service_url).await;

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
