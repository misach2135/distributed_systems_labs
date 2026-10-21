//! Service discovery through the Kubernetes API.
use futures::StreamExt;
use k8s_openapi::api::discovery::v1::EndpointSlice;
use kube::runtime::reflector::{self, Store};
use kube::runtime::{WatchStreamExt, watcher};
use kube::{Api, Client};
use tracing::{info, warn};

const SERVICE_NAME_LABEL: &str = "kubernetes.io/service-name";

#[derive(Clone)]
pub struct Discovery {
    store: Store<EndpointSlice>,
}

impl Discovery {
    /// Connect to the API server and start watching EndpointSlices.
    /// Returns once the initial list has been loaded.
    pub async fn start() -> anyhow::Result<Self> {
        let client = Client::try_default().await?;
        let api: Api<EndpointSlice> = Api::default_namespaced(client);

        let (store, writer) = reflector::store();
        let updates = reflector::reflector(writer, watcher(api, watcher::Config::default()))
            .default_backoff()
            .touched_objects();
        tokio::spawn(updates.for_each(|update| async move {
            match update {
                Ok(slice) => info!(
                    service = service_name(&slice).unwrap_or("-"),
                    addresses = ?ready_addresses(&slice),
                    "endpoints updated"
                ),
                Err(e) => warn!(error = %e, "endpointslice watch failed, retrying"),
            }
        }));

        store.wait_until_ready().await?;
        Ok(Self { store })
    }

    /// `http://ip:port` of every ready pod behind `service` (empty if none).
    pub fn addresses(&self, service: &str) -> Vec<String> {
        self.store
            .state()
            .iter()
            .filter(|slice| service_name(slice) == Some(service))
            .flat_map(|slice| ready_addresses(slice))
            .collect()
    }
}

fn service_name(slice: &EndpointSlice) -> Option<&str> {
    slice
        .metadata
        .labels
        .as_ref()?
        .get(SERVICE_NAME_LABEL)
        .map(String::as_str)
}

/// Addresses of the endpoints that pass their readiness probe.
fn ready_addresses(slice: &EndpointSlice) -> Vec<String> {
    let Some(port) = slice.ports.iter().flatten().find_map(|p| p.port) else {
        return Vec::new();
    };
    slice
        .endpoints
        .iter()
        .flatten()
        // An unset `ready` means ready (per the EndpointSlice API).
        .filter(|e| e.conditions.as_ref().and_then(|c| c.ready) != Some(false))
        .flat_map(|e| e.addresses.iter())
        .map(|ip| format!("http://{ip}:{port}"))
        .collect()
}
