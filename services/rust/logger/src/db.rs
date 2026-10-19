//! In-memory storage for transactions, keyed by `transaction_id`.

use std::sync::Arc;

use common::Transaction;
use dashmap::DashMap;
use uuid::Uuid;

/// Thread-safe store mapping `transaction_id -> Transaction`.
#[derive(Clone, Default)]
pub struct LogStore {
    inner: Arc<DashMap<Uuid, Transaction>>,
}

impl LogStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert (or overwrite) a transaction by its id.
    pub fn insert(&self, tx: Transaction) {
        self.inner.insert(tx.transaction_id, tx);
    }

    /// Return every stored transaction belonging to `user_id`.
    pub fn by_user(&self, user_id: &str) -> Vec<Transaction> {
        self.inner
            .iter()
            .filter(|entry| entry.value().user_id == user_id)
            .map(|entry| entry.value().clone())
            .collect()
    }
}
