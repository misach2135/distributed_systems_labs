//! HTTP client used by the facade to talk to the downstream services.
//!
//! Every remote call is timed and the elapsed duration is accumulated into
//! [`Metrics`], split by target service.
//!
//! There are several logging-service instances: each call goes to a random
//! one, and if it fails, to the next one, until one of them answers.

use std::collections::HashMap;
use std::time::Duration;

use common::{Ack, BalanceResponse, Transaction};
use rand::seq::SliceRandom;
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use tokio::time::Instant;
use tracing::warn;

use crate::metrics::{Metrics, Target};

/// Upper bound for a single downstream request. Generous on purpose: right
/// after a Hazelcast member crashes, a write may wait a few seconds until the
/// cluster reassigns that member's partitions.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct Downstream {
    http: Client,
    logging_urls: Vec<String>,
    counter_url: String,
    metrics: Metrics,
}

/// Send a request and decode a JSON response, treating non-2xx as an error.
async fn fetch<T: DeserializeOwned>(request: RequestBuilder) -> Result<T, reqwest::Error> {
    request.send().await?.error_for_status()?.json().await
}

impl Downstream {
    pub fn new(logging_urls: Vec<String>, counter_url: String, metrics: Metrics) -> Self {
        Self {
            http: Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("valid reqwest client config"),
            logging_urls,
            counter_url,
            metrics,
        }
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Await `call` and add its duration to the metrics of `target`.
    async fn timed<T>(&self, target: Target, call: impl Future<Output = T>) -> T {
        let start = Instant::now();
        let result = call.await;
        self.metrics
            .record(target, start.elapsed().as_nanos() as u64);
        result
    }

    /// Try the logging instances in random order; return the first success
    /// (or the last error if all of them fail).
    async fn logging<T: DeserializeOwned>(
        &self,
        request: impl Fn(&str) -> RequestBuilder,
    ) -> Result<T, reqwest::Error> {
        let mut urls = self.logging_urls.clone();
        urls.shuffle(&mut rand::rng());

        self.timed(Target::Logging, async {
            let mut last_error = None;
            for url in &urls {
                match fetch(request(url)).await {
                    Ok(value) => return Ok(value),
                    Err(e) => {
                        warn!(%url, error = %e, "logging-service instance failed, trying the next one");
                        last_error = Some(e);
                    }
                }
            }
            Err(last_error.expect("at least one logging-service url"))
        })
        .await
    }

    /// `POST {logging}/log`
    pub async fn log(&self, tx: &Transaction) -> Result<Ack, reqwest::Error> {
        self.logging(|base| self.http.post(format!("{base}/log")).json(tx))
            .await
    }

    /// `GET {logging}/transactions/{user_id}`
    pub async fn transactions(&self, user_id: &str) -> Result<Vec<Transaction>, reqwest::Error> {
        self.logging(|base| self.http.get(format!("{base}/transactions/{user_id}")))
            .await
    }

    /// `POST {counter}/process` -> updated balance
    pub async fn process(&self, tx: &Transaction) -> Result<BalanceResponse, reqwest::Error> {
        let request = self
            .http
            .post(format!("{}/process", self.counter_url))
            .json(tx);
        self.timed(Target::Counter, fetch(request)).await
    }

    /// `GET {counter}/balance/{user_id}`
    pub async fn balance(&self, user_id: &str) -> Result<BalanceResponse, reqwest::Error> {
        let request = self
            .http
            .get(format!("{}/balance/{user_id}", self.counter_url));
        self.timed(Target::Counter, fetch(request)).await
    }

    /// `GET {counter}/balances`
    pub async fn balances(&self) -> Result<HashMap<String, f64>, reqwest::Error> {
        let request = self.http.get(format!("{}/balances", self.counter_url));
        self.timed(Target::Counter, fetch(request)).await
    }
}
