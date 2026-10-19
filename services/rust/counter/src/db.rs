//! Account balances in PostgreSQL, keyed by `user_id`.

use std::collections::HashMap;

use sqlx::PgPool;

#[derive(Clone)]
pub struct BalanceStore {
    pool: PgPool,
}

impl BalanceStore {
    /// Connect to Postgres and create the `accounts` table if it is missing.
    pub async fn connect(url: &str) -> sqlx::Result<Self> {
        let pool = PgPool::connect(url).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS accounts (
                user_id TEXT PRIMARY KEY,
                balance DOUBLE PRECISION NOT NULL
            )",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }

    /// Apply a credit/debit to `user_id` and return the new balance.
    ///
    /// A single atomic upsert: Postgres locks the row for the update, so
    /// concurrent transactions on the same account are never lost.
    pub async fn apply(&self, user_id: &str, amount: f64) -> sqlx::Result<f64> {
        sqlx::query_scalar(
            "INSERT INTO accounts (user_id, balance) VALUES ($1, $2)
             ON CONFLICT (user_id) DO UPDATE SET balance = accounts.balance + EXCLUDED.balance
             RETURNING balance",
        )
        .bind(user_id)
        .bind(amount)
        .fetch_one(&self.pool)
        .await
    }

    /// Current balance for a single user (0.0 if unseen).
    pub async fn get(&self, user_id: &str) -> sqlx::Result<f64> {
        let balance = sqlx::query_scalar("SELECT balance FROM accounts WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(balance.unwrap_or(0.0))
    }

    /// Every account balance.
    pub async fn all(&self) -> sqlx::Result<HashMap<String, f64>> {
        let rows: Vec<(String, f64)> = sqlx::query_as("SELECT user_id, balance FROM accounts")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().collect())
    }
}
