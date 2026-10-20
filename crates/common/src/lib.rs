//! Shared communication protocol for the banking microservices.
//!
//! Defines the data structures exchanged over HTTP between the
//! `facade-service`, `logging-service` and `counter-service`, plus the
//! custom client-facing transaction format `{user_Id, +/- amount}`.

use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// A transaction as it travels through the system once the facade has
/// enriched the client message with an id and a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub transaction_id: Uuid,
    pub user_id: String,
    pub amount: f64,
    /// Unix timestamp in milliseconds, also usable as a unique-ish id.
    pub timestamp: i64,
}

// Note: the facade sends a full [`Transaction`] both to `counter-service`
// (`POST /process`) and to `logging-service` (`POST /log`), so there is no
// separate request struct — `Transaction` is reused directly.

/// Response returned by `counter-service` after applying a transaction
/// or when querying a single balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub user_id: String,
    pub balance: f64,
}

/// Response returned by the facade for `POST /transaction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub transaction_id: Uuid,
    pub balance: f64,
}

/// Response returned by the facade for `GET /user/{user_id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub balance: f64,
    pub transactions: Vec<Transaction>,
}

/// Accumulated latency (in milliseconds) spent calling the downstream
/// services, returned by the facade `GET /metrics` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub logging_total_ms: f64,
    pub counter_total_ms: f64,
    pub logging_calls: u64,
    pub counter_calls: u64,
    pub logging_avg_ms: f64,
    pub counter_avg_ms: f64,
}

/// Body of `POST /register` on the config-server: an instance of `service`
/// is reachable at `address` (e.g. `http://logger-1:8000`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub service: String,
    pub address: String,
}

/// Generic acknowledgement returned by `logging-service`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ack {
    pub status: String,
}

impl Ack {
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
        }
    }
}

/// Error produced while parsing the custom `{user_Id, +/- amount}` format.
#[derive(Debug, Error)]
pub enum ParseTransactionError {
    #[error("message does not match the expected format `{{user_Id, +/- amount}}`")]
    BadFormat,
    #[error("invalid amount: {0}")]
    BadAmount(#[from] std::num::ParseFloatError),
}

/// A transaction message as submitted by a client, before the facade
/// assigns it an id and timestamp.
///
/// Wire format (custom, non-JSON): `{user_Id, +/- amount}`,
/// e.g. `{user1, +100}` or `{ user1 , -42.5 }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClientTransaction {
    pub user_id: String,
    pub amount: f64,
}

fn client_tx_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*\{\s*(?P<user>[^,{}]+?)\s*,\s*(?P<amount>[+-]?\d+(?:\.\d+)?)\s*\}\s*$")
            .expect("static transaction regex is valid")
    })
}

impl FromStr for ClientTransaction {
    type Err = ParseTransactionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let caps = client_tx_regex()
            .captures(s)
            .ok_or(ParseTransactionError::BadFormat)?;

        let user_id = caps["user"].to_string();
        let amount: f64 = caps["amount"].parse()?;

        Ok(ClientTransaction { user_id, amount })
    }
}

impl fmt::Display for ClientTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Always render an explicit sign to mirror the `+/- amount` spec.
        write!(f, "{{{}, {:+}}}", self.user_id, self.amount)
    }
}

impl ClientTransaction {
    /// Promote a client message into a full [`Transaction`] by assigning a
    /// fresh id and the current timestamp.
    pub fn into_transaction(self, transaction_id: Uuid, timestamp: i64) -> Transaction {
        Transaction {
            transaction_id,
            user_id: self.user_id,
            amount: self.amount,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_positive() {
        let tx: ClientTransaction = "{user1, +100}".parse().unwrap();
        assert_eq!(tx.user_id, "user1");
        assert_eq!(tx.amount, 100.0);
    }

    #[test]
    fn parses_negative_and_whitespace() {
        let tx: ClientTransaction = "{  user_42 , -42.5 }".parse().unwrap();
        assert_eq!(tx.user_id, "user_42");
        assert_eq!(tx.amount, -42.5);
    }

    #[test]
    fn parses_unsigned() {
        let tx: ClientTransaction = "{acc, 10}".parse().unwrap();
        assert_eq!(tx.amount, 10.0);
    }

    #[test]
    fn rejects_garbage() {
        assert!("not a transaction".parse::<ClientTransaction>().is_err());
        assert!("{user1}".parse::<ClientTransaction>().is_err());
        assert!("{user1, abc}".parse::<ClientTransaction>().is_err());
    }

    #[test]
    fn round_trips_through_display() {
        let tx = ClientTransaction {
            user_id: "user1".into(),
            amount: 100.0,
        };
        let rendered = tx.to_string();
        assert_eq!(rendered, "{user1, +100}");
        assert_eq!(rendered.parse::<ClientTransaction>().unwrap(), tx);
    }
}
