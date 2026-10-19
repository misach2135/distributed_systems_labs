//! HTTP client used by the facade to talk to the downstream services.
//!
//! Every remote call is timed with [`tokio::time::Instant`] and the elapsed
//! duration is accumulated into [`Metrics`], split by target service.

use std::collections::HashMap;

use common::{Ack, BalanceResponse, Transaction};
use reqwest::Client;
use tokio::time::Instant;

use crate::metrics::{Metrics, Target};

#[derive(Clone)]
pub struct Downstream {
    http: Client,
    logging_url: String,
    counter_url: String,
    metrics: Metrics,
}

impl Downstream {
    pub fn new(logging_url: String, counter_url: String, metrics: Metrics) -> Self {
        Self {
            http: Client::new(),
            logging_url,
            counter_url,
            metrics,
        }
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// `POST {logging}/log`
    pub async fn log(&self, tx: &Transaction) -> Result<Ack, reqwest::Error> {
        let url = format!("{}/log", self.logging_url);
        let start = Instant::now();
        let result = self
            .http
            .post(&url)
            .json(tx)
            .send()
            .await
            .and_then(|r| r.error_for_status());
        self.metrics
            .record(Target::Logging, start.elapsed().as_nanos() as u64);
        result?.json().await
    }

    /// `POST {counter}/process` -> updated balance
    pub async fn process(&self, tx: &Transaction) -> Result<BalanceResponse, reqwest::Error> {
        let url = format!("{}/process", self.counter_url);
        let start = Instant::now();
        let result = self
            .http
            .post(&url)
            .json(tx)
            .send()
            .await
            .and_then(|r| r.error_for_status());
        self.metrics
            .record(Target::Counter, start.elapsed().as_nanos() as u64);
        result?.json().await
    }

    /// `GET {logging}/transactions/{user_id}`
    pub async fn transactions(&self, user_id: &str) -> Result<Vec<Transaction>, reqwest::Error> {
        let url = format!("{}/transactions/{}", self.logging_url, user_id);
        let start = Instant::now();
        let result = self
            .http
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status());
        self.metrics
            .record(Target::Logging, start.elapsed().as_nanos() as u64);
        result?.json().await
    }

    /// `GET {counter}/balance/{user_id}`
    pub async fn balance(&self, user_id: &str) -> Result<BalanceResponse, reqwest::Error> {
        let url = format!("{}/balance/{}", self.counter_url, user_id);
        let start = Instant::now();
        let result = self
            .http
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status());
        self.metrics
            .record(Target::Counter, start.elapsed().as_nanos() as u64);
        result?.json().await
    }

    /// `GET {counter}/balances`
    pub async fn balances(&self) -> Result<HashMap<String, f64>, reqwest::Error> {
        let url = format!("{}/balances", self.counter_url);
        let start = Instant::now();
        let result = self
            .http
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status());
        self.metrics
            .record(Target::Counter, start.elapsed().as_nanos() as u64);
        result?.json().await
    }
}
