//! Client side of the config-server: self-registration of a service instance
//! and lookup of the addresses of other services.

use std::time::Duration;

use tracing::{info, warn};

use common::Registration;

/// How many times to try registering before giving up.
const MAX_ATTEMPTS: u32 = 10;

/// Register this instance (`address`) of `service` in the config-server.
pub async fn register(
    config_server_url: &str,
    service: &str,
    address: &str,
) -> Result<(), reqwest::Error> {
    let http = reqwest::Client::new();
    let registration = Registration {
        service: service.to_string(),
        address: address.to_string(),
    };
    let mut attempt = 1;
    loop {
        let result = http
            .post(format!("{config_server_url}/register"))
            .json(&registration)
            .send()
            .await
            .and_then(|r| r.error_for_status());
        match result {
            Ok(_) => {
                info!(service, address, "registered in config-server");
                return Ok(());
            }
            Err(e) if attempt < MAX_ATTEMPTS => {
                warn!(error = %e, attempt, "config-server unavailable, retrying");
                attempt += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Addresses of all registered instances of `service` (empty if none).
pub async fn addresses(
    http: &reqwest::Client,
    config_server_url: &str,
    service: &str,
) -> Result<Vec<String>, reqwest::Error> {
    http.get(format!("{config_server_url}/services/{service}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}
