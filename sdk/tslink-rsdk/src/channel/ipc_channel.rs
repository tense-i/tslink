//! IPC Channel implementation using iceoryx2 for zero-copy inter-process communication

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use iceoryx2::prelude::*;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use super::{MessageChannel, MessageReceiveCallback};
use crate::error::{Error, Result};

/// Discovery topic for device awareness
const DEVICE_DISCOVERY_TOPIC: &str = "sys/device/discovery";

/// Maximum payload size for IPC messages (64KB)
const MAX_PAYLOAD_SIZE: usize = 65536;

/// IPC channel configuration
#[derive(Debug, Clone)]
pub struct IpcConfig {
    /// Product key
    pub product_key: String,
    /// Device ID
    pub device_id: String,
    /// Device discovery broadcast interval in seconds
    pub discovery_interval_secs: u64,
    /// Device cache expiration time in seconds
    pub device_cache_expire_secs: u64,
    /// Service name prefix
    pub service_prefix: String,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            product_key: String::new(),
            device_id: String::new(),
            discovery_interval_secs: 5,
            device_cache_expire_secs: 30,
            service_prefix: "tslink".to_string(),
        }
    }
}

impl IpcConfig {
    /// Create a new IPC configuration
    pub fn new(product_key: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self {
            product_key: product_key.into(),
            device_id: device_id.into(),
            ..Default::default()
        }
    }

    /// Set discovery interval
    pub fn with_discovery_interval(mut self, secs: u64) -> Self {
        self.discovery_interval_secs = secs;
        self
    }

    /// Set device cache expiration time
    pub fn with_cache_expire(mut self, secs: u64) -> Self {
        self.device_cache_expire_secs = secs;
        self
    }

    /// Set service prefix
    pub fn with_service_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.service_prefix = prefix.into();
        self
    }
}

/// IPC message payload for zero-copy transfer
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IpcPayload {
    /// Actual message length
    pub len: u32,
    /// Message data buffer
    pub data: [u8; MAX_PAYLOAD_SIZE],
}

impl Default for IpcPayload {
    fn default() -> Self {
        Self {
            len: 0,
            data: [0u8; MAX_PAYLOAD_SIZE],
        }
    }
}

impl IpcPayload {
    /// Create a new payload from a string message
    pub fn from_str(message: &str) -> Result<Self> {
        let bytes = message.as_bytes();
        if bytes.len() > MAX_PAYLOAD_SIZE {
            return Err(Error::Configuration(format!(
                "Message too large: {} bytes (max: {})",
                bytes.len(),
                MAX_PAYLOAD_SIZE
            )));
        }

        let mut payload = Self::default();
        payload.len = bytes.len() as u32;
        payload.data[..bytes.len()].copy_from_slice(bytes);
        Ok(payload)
    }

    /// Convert payload to string
    pub fn to_string(&self) -> Result<String> {
        let len = self.len as usize;
        if len > MAX_PAYLOAD_SIZE {
            return Err(Error::Internal("Invalid payload length".to_string()));
        }
        String::from_utf8(self.data[..len].to_vec())
            .map_err(|e| Error::Internal(format!("Invalid UTF-8: {}", e)))
    }
}

/// Subscriber entry with callback
struct SubscriberEntry {
    _service_name: String,
    handle: Option<JoinHandle<()>>,
}

/// IPC Channel for zero-copy inter-process communication
pub struct IpcChannel {
    config: IpcConfig,
    callback: Arc<RwLock<Option<Arc<dyn MessageReceiveCallback>>>>,
    is_running: Arc<AtomicBool>,
    subscribers: Arc<RwLock<HashMap<String, SubscriberEntry>>>,
    node_name: String,
}

impl std::fmt::Debug for IpcChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpcChannel")
            .field("config", &self.config)
            .field("is_running", &self.is_running.load(Ordering::SeqCst))
            .field("node_name", &self.node_name)
            .finish()
    }
}

impl IpcChannel {
    /// Create a new IPC channel
    pub fn new(config: IpcConfig) -> Result<Self> {
        let node_name = format!(
            "{}/{}",
            config.product_key, config.device_id
        );

        Ok(Self {
            config,
            callback: Arc::new(RwLock::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            node_name,
        })
    }

    /// Get the full service name for a topic
    fn service_name(&self, topic: &str) -> String {
        format!("{}/{}", self.config.service_prefix, topic)
    }

    /// Publish a message to a topic
    async fn publish_internal(&self, topic: &str, message: &str) -> Result<()> {
        let service_name = self.service_name(topic);
        let payload = IpcPayload::from_str(message)?;

        // Create node and service for publishing
        let node = NodeBuilder::new()
            .create::<ipc::Service>()
            .map_err(|e| Error::Channel(format!("Failed to create IPC node: {:?}", e)))?;

        let service = node
            .service_builder(&service_name.as_str().try_into().map_err(|e| {
                Error::Configuration(format!("Invalid service name '{}': {:?}", service_name, e))
            })?)
            .publish_subscribe::<IpcPayload>()
            .open_or_create()
            .map_err(|e| Error::Channel(format!("Failed to create service: {:?}", e)))?;

        let publisher = service
            .publisher_builder()
            .create()
            .map_err(|e| Error::Channel(format!("Failed to create publisher: {:?}", e)))?;

        // Send message using zero-copy API
        let sample = publisher
            .loan_uninit()
            .map_err(|e| Error::Internal(format!("Failed to loan sample: {:?}", e)))?;

        let sample = sample.write_payload(payload);
        sample
            .send()
            .map_err(|e| Error::Internal(format!("Failed to send sample: {:?}", e)))?;

        debug!("IPC published to {}: {} bytes", topic, message.len());
        Ok(())
    }

    /// Subscribe to a topic and start receiving messages
    async fn subscribe_internal(&self, topic: &str) -> Result<()> {
        let service_name = self.service_name(topic);
        let service_name_clone = service_name.clone();
        let topic_owned = topic.to_string();
        let callback = Arc::clone(&self.callback);
        let is_running = Arc::clone(&self.is_running);

        // Spawn a task to receive messages
        let handle = tokio::task::spawn_blocking(move || {
            let service_name = service_name_clone;
            let node = match NodeBuilder::new().create::<ipc::Service>() {
                Ok(n) => n,
                Err(e) => {
                    error!("Failed to create IPC node for subscriber: {:?}", e);
                    return;
                }
            };

            let service = match node
                .service_builder(&service_name.as_str().try_into().unwrap())
                .publish_subscribe::<IpcPayload>()
                .open_or_create()
            {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to create service for subscriber: {:?}", e);
                    return;
                }
            };

            let subscriber = match service.subscriber_builder().create() {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to create subscriber: {:?}", e);
                    return;
                }
            };

            info!("IPC subscribed to topic: {}", topic_owned);

            while is_running.load(Ordering::SeqCst) {
                // Use node.wait for timing
                if node.wait(std::time::Duration::from_millis(100)).is_err() {
                    break;
                }

                // Receive all available messages
                while let Ok(Some(sample)) = subscriber.receive() {
                    match sample.to_string() {
                        Ok(message) => {
                            debug!("IPC received from {}: {} bytes", topic_owned, message.len());
                            
                            // Call the callback
                            let rt = tokio::runtime::Handle::current();
                            let cb = callback.clone();
                            let topic = topic_owned.clone();
                            rt.spawn(async move {
                                let guard = cb.read().await;
                                if let Some(ref cb) = *guard {
                                    cb.receive(&topic, &message);
                                }
                            });
                        }
                        Err(e) => {
                            warn!("Failed to decode IPC message: {}", e);
                        }
                    }
                }
            }

            info!("IPC subscriber stopped for topic: {}", topic_owned);
        });

        // Store the subscriber entry
        let mut subs = self.subscribers.write().await;
        subs.insert(
            topic.to_string(),
            SubscriberEntry {
                _service_name: service_name,
                handle: Some(handle),
            },
        );

        Ok(())
    }
}

#[async_trait]
impl MessageChannel for IpcChannel {
    async fn send(&self, topic: &str, message: &str) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Err(Error::Channel("IPC channel not started".to_string()));
        }
        self.publish_internal(topic, message).await
    }

    async fn add_topic(&self, topic: &str) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Err(Error::Channel("IPC channel not started".to_string()));
        }
        self.subscribe_internal(topic).await
    }

    async fn start(&self) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!(
            "Starting IPC channel: {} (node: {})",
            self.config.device_id, self.node_name
        );

        self.is_running.store(true, Ordering::SeqCst);
        
        // Subscribe to device discovery topic
        self.subscribe_internal(DEVICE_DISCOVERY_TOPIC).await?;

        info!("IPC channel started successfully");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Stopping IPC channel: {}", self.config.device_id);

        self.is_running.store(false, Ordering::SeqCst);

        // Wait for all subscriber tasks to finish
        let mut subs = self.subscribers.write().await;
        for (topic, entry) in subs.drain() {
            if let Some(handle) = entry.handle {
                debug!("Waiting for subscriber to stop: {}", topic);
                let _ = handle.await;
            }
        }

        info!("IPC channel stopped");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

impl IpcChannel {
    /// Set the message callback
    pub fn set_callback(&self, callback: Arc<dyn MessageReceiveCallback>) {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let mut guard = self.callback.write().await;
            *guard = Some(callback);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_config_default() {
        let config = IpcConfig::default();
        assert_eq!(config.discovery_interval_secs, 5);
        assert_eq!(config.device_cache_expire_secs, 30);
        assert_eq!(config.service_prefix, "tslink");
    }

    #[test]
    fn test_ipc_config_builder() {
        let config = IpcConfig::new("pk", "did")
            .with_discovery_interval(10)
            .with_cache_expire(60)
            .with_service_prefix("custom");

        assert_eq!(config.product_key, "pk");
        assert_eq!(config.device_id, "did");
        assert_eq!(config.discovery_interval_secs, 10);
        assert_eq!(config.device_cache_expire_secs, 60);
        assert_eq!(config.service_prefix, "custom");
    }

    #[test]
    fn test_ipc_payload_from_str() {
        let payload = IpcPayload::from_str("hello").unwrap();
        assert_eq!(payload.len, 5);
        assert_eq!(&payload.data[..5], b"hello");
    }

    #[test]
    fn test_ipc_payload_to_string() {
        let mut payload = IpcPayload::default();
        payload.len = 5;
        payload.data[..5].copy_from_slice(b"world");
        
        let s = payload.to_string().unwrap();
        assert_eq!(s, "world");
    }

    #[test]
    fn test_ipc_payload_too_large() {
        let large_msg = "x".repeat(MAX_PAYLOAD_SIZE + 1);
        let result = IpcPayload::from_str(&large_msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_ipc_channel_creation() {
        let config = IpcConfig::new("test_pk", "test_did");
        let channel = IpcChannel::new(config);
        assert!(channel.is_ok());
    }
}
