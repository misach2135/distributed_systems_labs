//! Accumulated latency tracking for downstream calls.
//!
//! The facade measures how long each remote call to `logging-service` and
//! `counter-service` takes and accumulates the totals here, so they can be
//! reported via `GET /metrics` (and reset via `POST /metrics/reset`).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use common::MetricsResponse;

#[derive(Clone, Default)]
pub struct Metrics {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    logging_nanos: AtomicU64,
    counter_nanos: AtomicU64,
    logging_calls: AtomicU64,
    counter_calls: AtomicU64,
}

/// Which downstream service a measurement belongs to.
#[derive(Clone, Copy)]
pub enum Target {
    Logging,
    Counter,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the duration of one downstream call.
    pub fn record(&self, target: Target, nanos: u64) {
        let (time, count) = match target {
            Target::Logging => (&self.inner.logging_nanos, &self.inner.logging_calls),
            Target::Counter => (&self.inner.counter_nanos, &self.inner.counter_calls),
        };
        time.fetch_add(nanos, Ordering::Relaxed);
        count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        self.inner.logging_nanos.store(0, Ordering::Relaxed);
        self.inner.counter_nanos.store(0, Ordering::Relaxed);
        self.inner.logging_calls.store(0, Ordering::Relaxed);
        self.inner.counter_calls.store(0, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsResponse {
        let logging_nanos = self.inner.logging_nanos.load(Ordering::Relaxed);
        let counter_nanos = self.inner.counter_nanos.load(Ordering::Relaxed);
        let logging_calls = self.inner.logging_calls.load(Ordering::Relaxed);
        let counter_calls = self.inner.counter_calls.load(Ordering::Relaxed);

        let to_ms = |nanos: u64| nanos as f64 / 1_000_000.0;
        let avg = |nanos: u64, calls: u64| {
            if calls == 0 {
                0.0
            } else {
                to_ms(nanos) / calls as f64
            }
        };

        MetricsResponse {
            logging_total_ms: to_ms(logging_nanos),
            counter_total_ms: to_ms(counter_nanos),
            logging_calls,
            counter_calls,
            logging_avg_ms: avg(logging_nanos, logging_calls),
            counter_avg_ms: avg(counter_nanos, counter_calls),
        }
    }
}
