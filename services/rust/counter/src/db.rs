//! In-memory balances, keyed by `user_id`.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;

/// Thread-safe store mapping `user_id -> balance`.
///
/// Uses [`DashMap`] so that updates to *different* accounts proceed in
/// parallel, while concurrent updates to the *same* account are serialized
/// by the per-entry lock held inside [`Self::apply`]. This keeps the
/// read-modify-write of a single balance atomic, which is what makes the
/// "10 clients hammering one account" scenario produce a correct total.
#[derive(Clone, Default)]
pub struct BalanceStore {
    inner: Arc<DashMap<String, f64>>,
}

impl BalanceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a credit/debit to `user_id` and return the new balance.
    pub fn apply(&self, user_id: &str, amount: f64) -> f64 {
        let mut entry = self.inner.entry(user_id.to_string()).or_insert(0.0);
        *entry += amount;
        *entry
    }

    /// Current balance for a single user (0.0 if unseen).
    pub fn get(&self, user_id: &str) -> f64 {
        self.inner.get(user_id).map(|v| *v).unwrap_or(0.0)
    }

    /// Snapshot of every account balance.
    pub fn all(&self) -> HashMap<String, f64> {
        self.inner
            .iter()
            .map(|e| (e.key().clone(), *e.value()))
            .collect()
    }
}
