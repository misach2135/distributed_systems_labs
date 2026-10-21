//! HTTP client used by the facade to talk to the downstream services.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use common::{Ack, BalanceResponse, Transaction};
use connectors::k8s::Discovery;
use krafka::Record;
use krafka::producer::Producer;
use rand::seq::SliceRandom;
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use tokio::time::Instant;
use tracing::warn;

use crate::error::FacadeError;
use crate::metrics::{Metrics, Target};

/// Upper bound for a single downstream request. Generous on purpose: right
/// after a Hazelcast member crashes, a write may wait a few seconds until the
/// cluster reassigns that member's partitions.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct Downstream {
    http: Client,
    discovery: Discovery,
    producer: Arc<Producer>,
    topic: String,
    metrics: Metrics,
}

/// Send a request and decode a JSON response, treating non-2xx as an error.
async fn fetch<T: DeserializeOwned>(request: RequestBuilder) -> Result<T, reqwest::Error> {
    request.send().await?.error_for_status()?.json().await
}

impl Downstream {
    pub fn new(discovery: Discovery, producer: Producer, topic: String, metrics: Metrics) -> Self {
        Self {
            http: Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("valid reqwest client config"),
            discovery,
            producer: Arc::new(producer),
            topic,
            metrics,
        }
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Look up the ready pods of `service` and try them in random order;
    /// return the first success (or the last error if all fail).
    async fn call<T: DeserializeOwned>(
        &self,
        service: &str,
        target: Target,
        request: impl Fn(&str) -> RequestBuilder,
    ) -> Result<T, FacadeError> {
        let mut urls = self.discovery.addresses(service);
        urls.shuffle(&mut rand::rng());

        let start = Instant::now();
        let mut result = Err(FacadeError::NoInstances(service.to_string()));
        for url in &urls {
            match fetch(request(url)).await {
                Ok(value) => {
                    result = Ok(value);
                    break;
                }
                Err(e) => {
                    warn!(service, %url, error = %e, "instance failed, trying the next one");
                    result = Err(e.into());
                }
            }
        }
        self.metrics
            .record(target, start.elapsed().as_nanos() as u64);
        result
    }

    /// `POST {logger}/log`
    pub async fn log(&self, tx: &Transaction) -> Result<Ack, FacadeError> {
        self.call("logger", Target::Logging, |base| {
            self.http.post(format!("{base}/log")).json(tx)
        })
        .await
    }

    /// `GET {logger}/transactions/{user_id}`
    pub async fn transactions(&self, user_id: &str) -> Result<Vec<Transaction>, FacadeError> {
        self.call("logger", Target::Logging, |base| {
            self.http.get(format!("{base}/transactions/{user_id}"))
        })
        .await
    }

    pub async fn publish(&self, tx: &Transaction) -> Result<(), FacadeError> {
        let value = serde_json::to_vec(tx).expect("transaction serializes to JSON");
        let record = Record::new(self.topic.as_str(), value).key(tx.user_id.clone());

        let start = Instant::now();
        let result = self.producer.send(record).await;
        self.metrics
            .record(Target::Counter, start.elapsed().as_nanos() as u64);
        result.map(|_metadata| ()).map_err(FacadeError::from)
    }

    /// `GET {counter}/balance/{user_id}`
    pub async fn balance(&self, user_id: &str) -> Result<BalanceResponse, FacadeError> {
        self.call("counter", Target::Counter, |base| {
            self.http.get(format!("{base}/balance/{user_id}"))
        })
        .await
    }

    /// `GET {counter}/balances`
    pub async fn balances(&self) -> Result<HashMap<String, f64>, FacadeError> {
        self.call("counter", Target::Counter, |base| {
            self.http.get(format!("{base}/balances"))
        })
        .await
    }
}
