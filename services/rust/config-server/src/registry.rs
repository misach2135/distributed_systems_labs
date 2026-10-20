//! In-memory service registry: service name -> addresses of its instances.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct Registry {
    inner: Arc<RwLock<HashMap<String, BTreeSet<String>>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, service: String, address: String) {
        self.inner
            .write()
            .expect("registry lock poisoned")
            .entry(service)
            .or_default()
            .insert(address);
    }

    pub fn addresses(&self, service: &str) -> Vec<String> {
        self.inner
            .read()
            .expect("registry lock poisoned")
            .get(service)
            .map(|addresses| addresses.iter().cloned().collect())
            .unwrap_or_default()
    }
}
