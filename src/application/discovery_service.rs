use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::domain::device::Device;
use crate::error::Result;
use crate::infrastructure::database::device_repo::DeviceRepository;

/// Device discovery info returned to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryDevice {
    pub product_key: String,
    pub device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

impl From<&Device> for DiscoveryDevice {
    fn from(d: &Device) -> Self {
        Self {
            product_key: d.product_key.clone(),
            device_id: d.device_id.clone(),
            device_name: d.device_name.clone(),
            status: d.device_status.to_string(),
            parent_id: d.parent_id.clone(),
        }
    }
}

/// Application service for device discovery and topology.
pub struct DiscoveryService {
    device_repo: Arc<DeviceRepository>,
    cache: Arc<DashMap<String, Vec<DiscoveryDevice>>>,
    cache_ttl: Duration,
    last_refresh: Arc<RwLock<std::time::Instant>>,
}

impl DiscoveryService {
    pub fn new(device_repo: Arc<DeviceRepository>) -> Self {
        Self {
            device_repo,
            cache: Arc::new(DashMap::new()),
            cache_ttl: Duration::from_secs(60),
            last_refresh: Arc::new(RwLock::new(std::time::Instant::now())),
        }
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Get devices for a product key, using cache if available.
    pub async fn get_devices(&self, product_key: &str) -> Result<Vec<DiscoveryDevice>> {
        if let Some(cached) = self.cache.get(product_key) {
            let last = *self.last_refresh.read().await;
            if last.elapsed() < self.cache_ttl {
                debug!(product_key = %product_key, "returning cached devices");
                return Ok(cached.clone());
            }
        }

        self.refresh_cache_for_product(product_key).await
    }

    /// Get all sub-devices for a gateway device.
    pub async fn get_sub_devices(
        &self,
        product_key: &str,
        gateway_device_id: &str,
    ) -> Result<Vec<DiscoveryDevice>> {
        let devices = self.get_devices(product_key).await?;
        let sub_devices: Vec<DiscoveryDevice> = devices
            .into_iter()
            .filter(|d| d.parent_id.as_deref() == Some(gateway_device_id))
            .collect();
        Ok(sub_devices)
    }

    /// Force refresh cache for a specific product.
    pub async fn refresh_cache_for_product(
        &self,
        product_key: &str,
    ) -> Result<Vec<DiscoveryDevice>> {
        debug!(product_key = %product_key, "refreshing device cache");

        let devices = self.device_repo.find_by_product_key(product_key).await?;
        let discovery_devices: Vec<DiscoveryDevice> =
            devices.iter().map(DiscoveryDevice::from).collect();

        self.cache
            .insert(product_key.to_string(), discovery_devices.clone());
        *self.last_refresh.write().await = std::time::Instant::now();

        info!(
            product_key = %product_key,
            count = discovery_devices.len(),
            "device cache refreshed"
        );
        Ok(discovery_devices)
    }

    /// Refresh all cached product keys (called periodically).
    pub async fn refresh_all_caches(&self) -> Result<usize> {
        let keys: Vec<String> = self.cache.iter().map(|r| r.key().clone()).collect();
        let mut total = 0;

        for key in keys {
            match self.refresh_cache_for_product(&key).await {
                Ok(devices) => total += devices.len(),
                Err(e) => {
                    warn!(product_key = %key, error = %e, "failed to refresh cache");
                }
            }
        }

        Ok(total)
    }

    /// Build discovery response payload.
    pub fn build_response(&self, devices: Vec<DiscoveryDevice>) -> Value {
        json!({
            "devices": devices
        })
    }
}
