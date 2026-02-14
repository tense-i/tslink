//! Device discovery implementation for local IPC-based discovery

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::error::Result;

/// Discovery topic for device announcements
pub const DISCOVERY_TOPIC: &str = "sys/device/discovery";

/// Device status callback type
pub type DeviceStatusCallback = Arc<dyn Fn(&DeviceInfo, bool) + Send + Sync>;

/// Configuration for device discovery
#[derive(Debug, Clone)]
pub struct DeviceDiscoveryConfig {
    /// Product key of this device
    pub product_key: String,
    /// Device ID of this device
    pub device_id: String,
    /// Interval between discovery broadcasts (seconds)
    pub broadcast_interval_secs: u64,
    /// Time after which a device is considered offline (seconds)
    pub device_timeout_secs: u64,
}

impl Default for DeviceDiscoveryConfig {
    fn default() -> Self {
        Self {
            product_key: String::new(),
            device_id: String::new(),
            broadcast_interval_secs: 5,
            device_timeout_secs: 15,
        }
    }
}

/// Information about a discovered device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Product key of the device
    pub product_key: String,
    /// Device ID
    pub device_id: String,
    /// Device name (optional)
    pub device_name: Option<String>,
    /// Timestamp of last seen (Unix epoch millis)
    pub last_seen_ms: u64,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl DeviceInfo {
    /// Create a new DeviceInfo
    pub fn new(product_key: &str, device_id: &str) -> Self {
        Self {
            product_key: product_key.to_string(),
            device_id: device_id.to_string(),
            device_name: None,
            last_seen_ms: Self::current_time_ms(),
            metadata: HashMap::new(),
        }
    }

    /// Get current time in milliseconds
    fn current_time_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Update last seen timestamp
    pub fn touch(&mut self) {
        self.last_seen_ms = Self::current_time_ms();
    }

    /// Get unique device key
    pub fn key(&self) -> String {
        format!("{}:{}", self.product_key, self.device_id)
    }
}

/// Device discovery service for local IPC-based discovery
pub struct DeviceDiscovery {
    config: DeviceDiscoveryConfig,
    devices: Arc<RwLock<HashMap<String, DeviceEntry>>>,
    status_callback: Arc<RwLock<Option<DeviceStatusCallback>>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    broadcast_handle: RwLock<Option<JoinHandle<()>>>,
    cleanup_handle: RwLock<Option<JoinHandle<()>>>,
}

struct DeviceEntry {
    info: DeviceInfo,
    last_seen: Instant,
}

impl DeviceDiscovery {
    /// Create a new DeviceDiscovery service
    pub fn new(config: DeviceDiscoveryConfig) -> Self {
        Self {
            config,
            devices: Arc::new(RwLock::new(HashMap::new())),
            status_callback: Arc::new(RwLock::new(None)),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            broadcast_handle: RwLock::new(None),
            cleanup_handle: RwLock::new(None),
        }
    }

    /// Set callback for device status changes
    pub async fn set_status_callback(&self, callback: DeviceStatusCallback) {
        *self.status_callback.write().await = Some(callback);
    }

    /// Get list of online devices
    pub async fn get_devices(&self) -> Vec<DeviceInfo> {
        self.devices
            .read()
            .await
            .values()
            .map(|e| e.info.clone())
            .collect()
    }

    /// Get a specific device by key
    pub async fn get_device(&self, product_key: &str, device_id: &str) -> Option<DeviceInfo> {
        let key = format!("{}:{}", product_key, device_id);
        self.devices.read().await.get(&key).map(|e| e.info.clone())
    }

    /// Check if a device is online
    pub async fn is_device_online(&self, product_key: &str, device_id: &str) -> bool {
        let key = format!("{}:{}", product_key, device_id);
        self.devices.read().await.contains_key(&key)
    }

    /// Handle incoming discovery message
    pub async fn handle_discovery_message(&self, payload: &str) {
        match serde_json::from_str::<DeviceInfo>(payload) {
            Ok(mut info) => {
                info.touch();
                let key = info.key();
                let is_new = {
                    let mut devices = self.devices.write().await;
                    let is_new = !devices.contains_key(&key);
                    devices.insert(
                        key.clone(),
                        DeviceEntry {
                            info: info.clone(),
                            last_seen: Instant::now(),
                        },
                    );
                    is_new
                };

                if is_new {
                    info!(device_key = %key, "New device discovered");
                    if let Some(callback) = self.status_callback.read().await.as_ref() {
                        callback(&info, true);
                    }
                } else {
                    debug!(device_key = %key, "Device heartbeat received");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to parse discovery message");
            }
        }
    }

    /// Create broadcast message for this device
    pub fn create_broadcast_message(&self) -> String {
        let info = DeviceInfo::new(&self.config.product_key, &self.config.device_id);
        serde_json::to_string(&info).unwrap_or_default()
    }

    /// Start the discovery service (cleanup task only, broadcast handled externally)
    pub async fn start(&self) -> Result<()> {
        use std::sync::atomic::Ordering;

        if self.is_running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        info!("Starting device discovery service");

        // Start cleanup task
        let devices = self.devices.clone();
        let timeout = Duration::from_secs(self.config.device_timeout_secs);
        let status_callback = self.status_callback.clone();
        let is_running = self.is_running.clone();

        let cleanup_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;

                let mut expired = Vec::new();
                {
                    let devices = devices.read().await;
                    for (key, entry) in devices.iter() {
                        if entry.last_seen.elapsed() > timeout {
                            expired.push((key.clone(), entry.info.clone()));
                        }
                    }
                }

                if !expired.is_empty() {
                    let mut devices = devices.write().await;
                    for (key, info) in expired {
                        if devices.remove(&key).is_some() {
                            info!(device_key = %key, "Device offline (timeout)");
                            if let Some(callback) = status_callback.read().await.as_ref() {
                                callback(&info, false);
                            }
                        }
                    }
                }
            }
        });

        *self.cleanup_handle.write().await = Some(cleanup_handle);

        Ok(())
    }

    /// Stop the discovery service
    pub async fn stop(&self) -> Result<()> {
        use std::sync::atomic::Ordering;

        if !self.is_running.swap(false, Ordering::SeqCst) {
            return Ok(());
        }

        info!("Stopping device discovery service");

        if let Some(handle) = self.broadcast_handle.write().await.take() {
            handle.abort();
        }

        if let Some(handle) = self.cleanup_handle.write().await.take() {
            handle.abort();
        }

        Ok(())
    }

    /// Check if the discovery service is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_new() {
        let info = DeviceInfo::new("pk1", "dev1");
        assert_eq!(info.product_key, "pk1");
        assert_eq!(info.device_id, "dev1");
        assert_eq!(info.key(), "pk1:dev1");
    }

    #[test]
    fn test_device_discovery_config_default() {
        let config = DeviceDiscoveryConfig::default();
        assert_eq!(config.broadcast_interval_secs, 5);
        assert_eq!(config.device_timeout_secs, 15);
    }

    #[tokio::test]
    async fn test_device_discovery_handle_message() {
        let config = DeviceDiscoveryConfig {
            product_key: "test_pk".to_string(),
            device_id: "test_dev".to_string(),
            ..Default::default()
        };
        let discovery = DeviceDiscovery::new(config);

        let info = DeviceInfo::new("remote_pk", "remote_dev");
        let payload = serde_json::to_string(&info).unwrap();

        discovery.handle_discovery_message(&payload).await;

        let devices = discovery.get_devices().await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].product_key, "remote_pk");
    }
}
