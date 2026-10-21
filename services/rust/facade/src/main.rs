mod client;
mod error;
mod http;
mod metrics;

use std::env;

use krafka::Kafka;
use tokio::{net::TcpListener, select, signal::ctrl_c};
use tracing::info;

use crate::client::Downstream;
use crate::metrics::Metrics;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config_server_url =
        env::var("CONFIG_SERVER_URL").unwrap_or_else(|_| "http://localhost:8003".to_string());

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

    let downstream = Downstream::new(
        config_server_url.clone(),
        producer,
        kafka_topic.clone(),
        Metrics::new(),
    );
    let app = http::router(downstream);

    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener = TcpListener::bind(&addr).await?;

    let service_url = env::var("SERVICE_URL").unwrap_or_else(|_| format!("http://{addr}"));
    connectors::config_server::register(&config_server_url, "facade", &service_url).await?;

    info!(%addr, %config_server_url, %kafka_bootstrap, %kafka_topic, "facade-service listening");

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
