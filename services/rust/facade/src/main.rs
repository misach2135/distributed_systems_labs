mod client;
mod error;
mod http;
mod metrics;

use std::env;

use connectors::k8s::Discovery;
use krafka::Kafka;
use tokio::{
    net::TcpListener,
    select,
    signal::{
        ctrl_c,
        unix::{SignalKind, signal},
    },
};
use tracing::info;

use crate::client::Downstream;
use crate::metrics::Metrics;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Addresses of logger and counter pods come from Kubernetes.
    let discovery = Discovery::start().await?;

    let kafka_bootstrap =
        env::var("KAFKA_BOOTSTRAP").unwrap_or_else(|_| "localhost:9092".to_string());
    let kafka_topic = env::var("KAFKA_TOPIC").unwrap_or_else(|_| "transactions".to_string());
    let producer = Kafka::builder(kafka_bootstrap.as_str())
        .client_id("facade-service")
        .allow_auto_create_topics(true)
        .connect()
        .await?
        .producer()
        .build()
        .await?;

    let downstream = Downstream::new(discovery, producer, kafka_topic.clone(), Metrics::new());
    let app = http::router(downstream);

    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener = TcpListener::bind(&addr).await?;

    info!(%addr, %kafka_bootstrap, %kafka_topic, "facade-service listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("service stopped");

    Ok(())
}

async fn shutdown_signal() {
    let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    select! {
        _ = ctrl_c() => {}
        _ = terminate.recv() => {}
    }
    info!("shutdown signal received");
}
