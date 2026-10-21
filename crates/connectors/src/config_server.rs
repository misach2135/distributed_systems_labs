//! Client side of the config-server: self-registration of a service instance
//! and lookup of the addresses of other services.

use std::time::Duration;

use tracing::{info, warn};

use common::Registration;

/// Register this instance (`address`) of `service` in the config-server.
///
/// Retries every second until the config-server answers, so services can
/// start in any order.
pub async fn register(config_server_url: &str, service: &str, address: &str) {
    let http = reqwest::Client::new();
    let registration = Registration {
        service: service.to_string(),
        address: address.to_string(),
    };
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
                return;
            }
            Err(e) => {
                warn!(error = %e, "config-server unavailable, retrying");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
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
