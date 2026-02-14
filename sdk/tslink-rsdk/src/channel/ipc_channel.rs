//! IPC Channel implementation using iceoryx2 for zero-copy inter-process communication
//!
//! Architecture:
//! - A single dedicated **iceoryx2 thread** owns one persistent node and handles both
//!   publishing (cached publishers per topic) and subscribing (polling all subscribers).
//! - Commands are sent to this thread via `std::sync::mpsc` (publish, subscribe, stop).
//! - Received messages are dispatched to the `MessageReceiveCallback` via tokio tasks.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use iceoryx2::prelude::*;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::{MessageChannel, MessageReceiveCallback};
use crate::error::{Error, Result};

/// Discovery topic for device awareness
const DEVICE_DISCOVERY_TOPIC: &str = "sys/device/discovery";

/// Default maximum payload size for IPC messages (12MB, supports 4K YUV frames)
const DEFAULT_MAX_PAYLOAD_SIZE: usize = 12 * 1024 * 1024;

/// Default subscriber buffer size (number of samples the subscriber can buffer)
const DEFAULT_SUBSCRIBER_BUFFER_SIZE: usize = 256;

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
    /// Maximum payload size in bytes (default: 12MB)
    pub max_payload_size: usize,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            product_key: String::new(),
            device_id: String::new(),
            discovery_interval_secs: 5,
            device_cache_expire_secs: 30,
            service_prefix: "tslink".to_string(),
            max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
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

    /// Set maximum payload size in bytes
    pub fn with_max_payload_size(mut self, size: usize) -> Self {
        self.max_payload_size = size;
        self
    }
}

/// IPC message payload for zero-copy transfer (kept for backward compatibility / tests)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct IpcPayload {
    /// Message data buffer (variable length via iceoryx2 slice API)
    pub data: Vec<u8>,
}

impl IpcPayload {
    /// Create a new payload from a string message
    pub fn from_str(message: &str) -> Result<Self> {
        Ok(Self {
            data: message.as_bytes().to_vec(),
        })
    }

    /// Convert payload to string
    pub fn to_string(&self) -> Result<String> {
        String::from_utf8(self.data.clone())
            .map_err(|e| Error::Internal(format!("Invalid UTF-8: {}", e)))
    }
}

/// Command sent to the unified iceoryx2 thread
enum IpcCommand {
    /// Publish a message to a topic (data is raw bytes)
    Publish {
        service_name: String,
        data: Vec<u8>,
        result_tx: std::sync::mpsc::Sender<Result<()>>,
    },
    /// Subscribe to a topic
    Subscribe {
        topic: String,
        service_name: String,
        result_tx: std::sync::mpsc::Sender<Result<()>>,
    },
    /// Stop the thread
    Stop,
}

/// IPC Channel for zero-copy inter-process communication
pub struct IpcChannel {
    config: IpcConfig,
    callback: Arc<RwLock<Option<Arc<dyn MessageReceiveCallback>>>>,
    is_running: Arc<AtomicBool>,
    /// Channel to send commands to the unified iceoryx2 thread
    cmd_tx: std::sync::mpsc::Sender<IpcCommand>,
    /// Handle for the iceoryx2 thread
    iox_thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
    /// Service prefix for topic→service_name mapping
    service_prefix: String,
    /// Maximum payload size
    max_payload_size: usize,
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

        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<IpcCommand>();

        let callback: Arc<RwLock<Option<Arc<dyn MessageReceiveCallback>>>> =
            Arc::new(RwLock::new(None));
        let callback_clone = Arc::clone(&callback);
        let is_running = Arc::new(AtomicBool::new(false));
        let is_running_clone = Arc::clone(&is_running);
        let max_payload_size = config.max_payload_size;

        // Spawn the unified iceoryx2 thread
        let iox_thread = std::thread::Builder::new()
            .name("ipc-iceoryx2".into())
            .spawn(move || {
                Self::iox_thread_main(cmd_rx, callback_clone, is_running_clone, max_payload_size);
            })
            .map_err(|e| Error::Internal(format!("Failed to spawn iceoryx2 thread: {}", e)))?;

        let service_prefix = config.service_prefix.clone();

        Ok(Self {
            config,
            callback,
            is_running,
            cmd_tx,
            iox_thread: std::sync::Mutex::new(Some(iox_thread)),
            service_prefix,
            max_payload_size,
            node_name,
        })
    }

    /// Unified iceoryx2 thread — owns one node, handles pub+sub
    fn iox_thread_main(
        cmd_rx: std::sync::mpsc::Receiver<IpcCommand>,
        callback: Arc<RwLock<Option<Arc<dyn MessageReceiveCallback>>>>,
        is_running: Arc<AtomicBool>,
        max_payload_size: usize,
    ) {
        // Create the single persistent node
        let node = match NodeBuilder::new().create::<ipc::Service>() {
            Ok(n) => n,
            Err(e) => {
                error!("IPC thread: failed to create iceoryx2 node: {:?}", e);
                // Drain commands and error
                for cmd in cmd_rx.iter() {
                    match cmd {
                        IpcCommand::Publish { result_tx, .. } => {
                            let _ = result_tx.send(Err(Error::Channel(
                                "iceoryx2 node creation failed".into(),
                            )));
                        }
                        IpcCommand::Subscribe { result_tx, .. } => {
                            let _ = result_tx.send(Err(Error::Channel(
                                "iceoryx2 node creation failed".into(),
                            )));
                        }
                        IpcCommand::Stop => break,
                    }
                }
                return;
            }
        };

        debug!("IPC iceoryx2 thread started (single node)");

        // Publisher cache: service_name -> publish closure (takes &[u8] data)
        let mut pub_cache: HashMap<String, Box<dyn Fn(&[u8]) -> Result<()>>> = HashMap::new();

        // Subscriber list: topic -> (service_name, subscriber_receive_fn)
        struct SubEntry {
            topic: String,
            recv_fn: Box<dyn Fn() -> Vec<(String, String)>>,
        }
        let mut subscribers: Vec<SubEntry> = Vec::new();

        // Macro to process a command inline (avoids borrow conflicts with closures)
        macro_rules! process_cmd {
            ($cmd:expr, $node:expr, $pub_cache:expr, $subscribers:expr) => {
                match $cmd {
                    IpcCommand::Publish {
                        service_name,
                        data,
                        result_tx,
                    } => {
                        let result = if let Some(publish_fn) = $pub_cache.get(&service_name) {
                            publish_fn(&data)
                        } else {
                            match Self::create_publisher_fn(&$node, &service_name, max_payload_size) {
                                Ok(publish_fn) => {
                                    let result = publish_fn(&data);
                                    $pub_cache.insert(service_name, Box::new(publish_fn));
                                    result
                                }
                                Err(e) => Err(e),
                            }
                        };
                        let _ = result_tx.send(result);
                        true
                    }
                    IpcCommand::Subscribe {
                        topic,
                        service_name,
                        result_tx,
                    } => {
                        match Self::create_subscriber_fn(&$node, &service_name) {
                            Ok(recv_fn) => {
                                info!("IPC subscribed to topic: {}", topic);
                                $subscribers.push(SubEntry {
                                    topic,
                                    recv_fn: Box::new(recv_fn),
                                });
                                let _ = result_tx.send(Ok(()));
                            }
                            Err(e) => {
                                error!("Failed to subscribe to {}: {:?}", topic, e);
                                let _ = result_tx.send(Err(e));
                            }
                        }
                        true
                    }
                    IpcCommand::Stop => {
                        debug!("IPC iceoryx2 thread received Stop command");
                        false
                    }
                }
            };
        }

        loop {
            // 1. If no subscribers yet, block on commands (no need to poll)
            if subscribers.is_empty() {
                match cmd_rx.recv() {
                    Ok(cmd) => {
                        if !process_cmd!(cmd, node, pub_cache, subscribers) {
                            return;
                        }
                    }
                    Err(_) => {
                        debug!("IPC command channel disconnected, stopping");
                        return;
                    }
                }
                continue;
            }

            // 2. Process all pending commands (non-blocking)
            loop {
                match cmd_rx.try_recv() {
                    Ok(cmd) => {
                        if !process_cmd!(cmd, node, pub_cache, subscribers) {
                            return;
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        debug!("IPC command channel disconnected, stopping");
                        return;
                    }
                }
            }

            // 3. Poll all subscribers for messages
            let mut got_any = false;
            for sub in &subscribers {
                let messages = (sub.recv_fn)();
                for (topic, message) in messages {
                    got_any = true;
                    debug!("IPC received from {}: {} bytes", topic, message.len());

                    let cb = callback.clone();
                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        handle.spawn(async move {
                            let guard = cb.read().await;
                            if let Some(ref cb) = *guard {
                                cb.receive(&topic, &message);
                            }
                        });
                    } else {
                        let cb_guard = callback.blocking_read();
                        if let Some(ref cb) = *cb_guard {
                            cb.receive(&topic, &message);
                        }
                    }
                }
            }

            // 4. Check if channel stopped
            if !is_running.load(Ordering::SeqCst) {
                while let Ok(cmd) = cmd_rx.try_recv() {
                    if !process_cmd!(cmd, node, pub_cache, subscribers) {
                        return;
                    }
                }
                return;
            }

            // 5. Brief yield if no messages to avoid 100% CPU
            if !got_any {
                std::thread::sleep(std::time::Duration::from_micros(50));
            }
        }
    }

    /// Create a publisher closure for a service name (uses iceoryx2 slice API)
    fn create_publisher_fn(
        node: &Node<ipc::Service>,
        service_name: &str,
        max_payload_size: usize,
    ) -> Result<impl Fn(&[u8]) -> Result<()>> {
        let service = node
            .service_builder(
                &service_name
                    .try_into()
                    .map_err(|e| Error::Configuration(format!("Invalid service name: {:?}", e)))?,
            )
            .publish_subscribe::<[u8]>()
            .enable_safe_overflow(true)
            .subscriber_max_buffer_size(DEFAULT_SUBSCRIBER_BUFFER_SIZE)
            .open_or_create()
            .map_err(|e| Error::Channel(format!("Failed to create service: {:?}", e)))?;

        let publisher = service
            .publisher_builder()
            .initial_max_slice_len(max_payload_size)
            .create()
            .map_err(|e| Error::Channel(format!("Failed to create publisher: {:?}", e)))?;

        debug!(
            "IPC publisher created for: {} (max_slice_len={})",
            service_name, max_payload_size
        );

        Ok(move |data: &[u8]| -> Result<()> {
            let mut sample = publisher
                .loan_slice_uninit(data.len())
                .map_err(|e| Error::Internal(format!("Failed to loan slice: {:?}", e)))?;
            let slice = sample.payload_mut();
            for (dst, src) in slice.iter_mut().zip(data.iter()) {
                dst.write(*src);
            }
            unsafe { sample.assume_init() }
                .send()
                .map_err(|e| Error::Internal(format!("Failed to send sample: {:?}", e)))?;
            Ok(())
        })
    }

    /// Create a subscriber closure for a service name (uses iceoryx2 slice API)
    /// Returns a closure that drains all available messages and returns (topic, message) pairs
    fn create_subscriber_fn(
        node: &Node<ipc::Service>,
        service_name: &str,
    ) -> Result<impl Fn() -> Vec<(String, String)>> {
        let service = node
            .service_builder(
                &service_name
                    .try_into()
                    .map_err(|e| Error::Configuration(format!("Invalid service name: {:?}", e)))?,
            )
            .publish_subscribe::<[u8]>()
            .enable_safe_overflow(true)
            .subscriber_max_buffer_size(DEFAULT_SUBSCRIBER_BUFFER_SIZE)
            .open_or_create()
            .map_err(|e| Error::Channel(format!("Failed to create service: {:?}", e)))?;

        let subscriber = service
            .subscriber_builder()
            .buffer_size(DEFAULT_SUBSCRIBER_BUFFER_SIZE)
            .create()
            .map_err(|e| Error::Channel(format!("Failed to create subscriber: {:?}", e)))?;

        let topic = service_name.to_string();

        Ok(move || -> Vec<(String, String)> {
            let mut messages = Vec::new();
            while let Ok(Some(sample)) = subscriber.receive() {
                let payload: &[u8] = &sample;
                match String::from_utf8(payload.to_vec()) {
                    Ok(msg) => messages.push((topic.clone(), msg)),
                    Err(e) => warn!("Failed to decode IPC message: {}", e),
                }
            }
            messages
        })
    }

    /// Get the full service name for a topic
    fn service_name(&self, topic: &str) -> String {
        format!("{}/{}", self.service_prefix, topic)
    }

    /// Send a command to the iceoryx2 thread and wait for result
    fn send_publish(&self, topic: &str, message: &str) -> Result<()> {
        let data = message.as_bytes();
        if data.len() > self.max_payload_size {
            return Err(Error::Configuration(format!(
                "Message too large: {} bytes (max: {})",
                data.len(),
                self.max_payload_size
            )));
        }

        let service_name = self.service_name(topic);
        let (result_tx, result_rx) = std::sync::mpsc::channel();

        self.cmd_tx
            .send(IpcCommand::Publish {
                service_name,
                data: data.to_vec(),
                result_tx,
            })
            .map_err(|_| Error::Channel("iceoryx2 thread not running".into()))?;

        result_rx
            .recv()
            .map_err(|_| Error::Channel("iceoryx2 thread disconnected".into()))?
    }

    /// Send a subscribe command to the iceoryx2 thread
    fn send_subscribe(&self, topic: &str) -> Result<()> {
        let service_name = self.service_name(topic);

        let (result_tx, result_rx) = std::sync::mpsc::channel();

        self.cmd_tx
            .send(IpcCommand::Subscribe {
                topic: topic.to_string(),
                service_name,
                result_tx,
            })
            .map_err(|_| Error::Channel("iceoryx2 thread not running".into()))?;

        result_rx
            .recv()
            .map_err(|_| Error::Channel("iceoryx2 thread disconnected".into()))?
    }
}

#[async_trait]
impl MessageChannel for IpcChannel {
    async fn send(&self, topic: &str, message: &str) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Err(Error::Channel("IPC channel not started".to_string()));
        }
        self.send_publish(topic, message)
    }

    async fn add_topic(&self, topic: &str) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Err(Error::Channel("IPC channel not started".to_string()));
        }
        self.send_subscribe(topic)
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
        self.send_subscribe(DEVICE_DISCOVERY_TOPIC)?;

        info!("IPC channel started successfully");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Stopping IPC channel: {}", self.config.device_id);

        self.is_running.store(false, Ordering::SeqCst);

        // Send stop command to the iceoryx2 thread
        let _ = self.cmd_tx.send(IpcCommand::Stop);

        // Wait for the thread to finish
        if let Ok(mut guard) = self.iox_thread.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
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
    pub async fn set_callback(&self, callback: Arc<dyn MessageReceiveCallback>) {
        let mut guard = self.callback.write().await;
        *guard = Some(callback);
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
        assert_eq!(payload.data, b"hello");
    }

    #[test]
    fn test_ipc_payload_to_string() {
        let payload = IpcPayload {
            data: b"world".to_vec(),
        };
        let s = payload.to_string().unwrap();
        assert_eq!(s, "world");
    }

    #[test]
    fn test_ipc_payload_large() {
        let large_msg = "x".repeat(1_000_000);
        let payload = IpcPayload::from_str(&large_msg).unwrap();
        assert_eq!(payload.data.len(), 1_000_000);
    }

    #[test]
    fn test_ipc_channel_creation() {
        let config = IpcConfig::new("test_pk", "test_did");
        let channel = IpcChannel::new(config);
        assert!(channel.is_ok());
    }
}
