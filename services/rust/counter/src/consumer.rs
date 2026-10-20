//! Kafka consumer: applies transactions from the `transactions` topic.

use std::time::Duration;

use common::Transaction;
use krafka::Kafka;
use krafka::consumer::AutoOffsetReset;
use tracing::{error, info, warn};

use crate::db::BalanceStore;

pub async fn run(store: BalanceStore, bootstrap: &str, topic: &str) -> anyhow::Result<()> {
    let kafka = Kafka::builder(bootstrap)
        .client_id("counter-service")
        .allow_auto_create_topics(true)
        .connect()
        .await?;
    let consumer = kafka
        .consumer("counter-service")
        .auto_offset_reset(AutoOffsetReset::Earliest)
        .enable_auto_commit(false)
        .build()
        .await?;
    consumer.subscribe([topic]).await?;
    info!(%bootstrap, %topic, "consuming transactions");

    let shutdown = crate::shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        let polled = tokio::select! {
            _ = &mut shutdown => break,
            polled = consumer.poll(Duration::from_secs(1)) => polled,
        };
        let records = match polled {
            Ok(records) => records,
            Err(e) => {
                warn!(error = %e, "kafka poll failed, retrying");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        if records.is_empty() {
            continue;
        }

        for record in &records {
            let Some(tx) = record
                .value
                .as_deref()
                .and_then(|value| serde_json::from_slice::<Transaction>(value).ok())
            else {
                warn!(offset = record.offset, "skipping malformed message");
                continue;
            };
            apply(&store, &tx, record.offset).await;
        }

        if let Err(e) = consumer.commit().await {
            warn!(error = %e, "offset commit failed");
        }
    }

    consumer.close().await?;
    info!("left the consumer group");
    Ok(())
}

async fn apply(store: &BalanceStore, tx: &Transaction, offset: i64) {
    loop {
        match store
            .apply_once(tx.transaction_id, &tx.user_id, tx.amount)
            .await
        {
            Ok(Some(balance)) => {
                info!(offset, transaction_id = %tx.transaction_id, user_id = %tx.user_id, amount = tx.amount, balance, "applied transaction");
                return;
            }
            Ok(None) => {
                info!(offset, transaction_id = %tx.transaction_id, "skipped duplicate transaction");
                return;
            }
            Err(e) => {
                error!(error = %e, "database error, retrying");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
