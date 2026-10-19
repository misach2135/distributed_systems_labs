mod client;
mod error;
mod http;
mod metrics;

use std::env;

use tokio::{net::TcpListener, select, signal::ctrl_c};
use tracing::info;

use crate::client::Downstream;
use crate::metrics::Metrics;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Comma-separated list of logging-service instances.
    let logging_urls: Vec<String> = env::var("LOGGING_URLS")
        .unwrap_or_else(|_| "http://localhost:8001".to_string())
        .split(',')
        .map(str::to_string)
        .collect();
    let counter_url =
        env::var("COUNTER_URL").unwrap_or_else(|_| "http://localhost:8002".to_string());

    let downstream = Downstream::new(logging_urls.clone(), counter_url.clone(), Metrics::new());
    let app = http::router(downstream);

    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener = TcpListener::bind(&addr).await?;
    info!(%addr, ?logging_urls, %counter_url, "facade-service listening");

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
