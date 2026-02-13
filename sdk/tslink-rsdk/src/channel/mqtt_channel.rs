//! MQTT channel implementation

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use super::{MessageChannel, MessageReceiveCallback};
use crate::error::{Error, Result};

/// MQTT channel configuration
#[derive(Debug, Clone)]
pub struct MqttConfig {
    /// MQTT broker endpoint (e.g., "mqtt://broker:1883")
    pub endpoint: String,
    /// Product key
    pub product_key: String,
    /// Device ID
    pub device_id: String,
    /// Username for authentication
    pub username: String,
    /// Password for authentication
    pub password: String,
    /// Keep alive interval in seconds
    pub keep_alive_secs: u64,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Max inflight messages
    pub max_inflight: u16,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            product_key: String::new(),
            device_id: String::new(),
            username: String::new(),
            password: String::new(),
            keep_alive_secs: 20,
            connection_timeout_secs: 10,
            max_inflight: 1000,
        }
    }
}

/// MQTT channel for IoT communication
pub struct MqttChannel {
    config: MqttConfig,
    client: RwLock<Option<AsyncClient>>,
    callback: Arc<dyn MessageReceiveCallback>,
    topics: RwLock<Vec<String>>,
    is_connected: Arc<AtomicBool>,
    shutdown_tx: RwLock<Option<mpsc::Sender<()>>>,
}

impl std::fmt::Debug for MqttChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttChannel")
            .field("config", &self.config)
            .field("is_connected", &self.is_connected.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl MqttChannel {
    /// Create a new MQTT channel
    pub fn new(config: MqttConfig, callback: Arc<dyn MessageReceiveCallback>) -> Self {
        let default_topics = Self::default_topics(&config.product_key, &config.device_id);

        Self {
            config,
            client: RwLock::new(None),
            callback,
            topics: RwLock::new(default_topics),
            is_connected: Arc::new(AtomicBool::new(false)),
            shutdown_tx: RwLock::new(None),
        }
    }

    /// Get default subscription topics for a device
    fn default_topics(product_key: &str, device_id: &str) -> Vec<String> {
        vec![
            format!("sys/{}/{}/thing/properties/set", product_key, device_id),
            format!("sys/{}/{}/thing/service/+/post", product_key, device_id),
            format!(
                "sys/{}/{}/thing/service/property/set",
                product_key, device_id
            ),
            format!(
                "sys/{}/{}/platform/service/+/post_reply",
                product_key, device_id
            ),
            format!(
                "sys/{}/{}/thing/event/+/info_reply",
                product_key, device_id
            ),
            format!(
                "sys/{}/{}/thing/event/+/warning_reply",
                product_key, device_id
            ),
            format!(
                "sys/{}/{}/thing/event/+/error_reply",
                product_key, device_id
            ),
        ]
    }

    /// Create MQTT options from config
    fn create_mqtt_options(&self) -> MqttOptions {
        let client_id = format!("DEVICE:{}", self.config.device_id);

        // Parse endpoint to extract host and port
        let endpoint = self.config.endpoint.replace("mqtt://", "");
        let parts: Vec<&str> = endpoint.split(':').collect();
        let host = parts.first().unwrap_or(&"localhost");
        let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(1883);

        let mut options = MqttOptions::new(client_id, *host, port);
        options.set_credentials(&self.config.username, &self.config.password);
        options.set_keep_alive(Duration::from_secs(self.config.keep_alive_secs));
        options.set_inflight(self.config.max_inflight);
        options.set_clean_session(true);

        options
    }

    /// Subscribe to all configured topics
    async fn subscribe_topics(&self, client: &AsyncClient) -> Result<()> {
        let topics = self.topics.read().await.clone();
        for topic in topics {
            debug!("Subscribing to topic: {}", topic);
            client
                .subscribe(&topic, QoS::AtMostOnce)
                .await
                .map_err(|e| Error::MqttSubscribe(e.to_string()))?;
        }
        Ok(())
    }

    /// Handle incoming MQTT events
    async fn handle_events(
        mut eventloop: EventLoop,
        callback: Arc<dyn MessageReceiveCallback>,
        is_connected: Arc<AtomicBool>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                event = eventloop.poll() => {
                    match event {
                        Ok(Event::Incoming(Packet::Publish(publish))) => {
                            let topic = publish.topic.clone();
                            let payload = String::from_utf8_lossy(&publish.payload).to_string();
                            debug!("Received message on topic: {}, payload: {}", topic, payload);
                            callback.receive(&topic, &payload);
                        }
                        Ok(Event::Incoming(Packet::ConnAck(_))) => {
                            info!("MQTT connected");
                            is_connected.store(true, Ordering::SeqCst);
                        }
                        Ok(Event::Incoming(Packet::Disconnect)) => {
                            warn!("MQTT disconnected");
                            is_connected.store(false, Ordering::SeqCst);
                        }
                        Err(e) => {
                            error!("MQTT error: {:?}", e);
                            is_connected.store(false, Ordering::SeqCst);
                            // Wait before reconnecting
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                        _ => {}
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("MQTT channel shutting down");
                    break;
                }
            }
        }
    }
}

#[async_trait]
impl MessageChannel for MqttChannel {
    async fn send(&self, topic: &str, data: &str) -> Result<()> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| Error::NotStarted)?;

        debug!("Publishing to topic: {}, data: {}", topic, data);

        client
            .publish(topic, QoS::AtMostOnce, false, data.as_bytes())
            .await
            .map_err(|e| Error::MqttPublish(e.to_string()))?;

        Ok(())
    }

    async fn start(&self) -> Result<()> {
        if self.client.read().await.is_some() {
            return Err(Error::AlreadyStarted);
        }

        let options = self.create_mqtt_options();
        let (client, eventloop) = AsyncClient::new(options, 100);

        // Subscribe to topics
        self.subscribe_topics(&client).await?;

        // Store client
        *self.client.write().await = Some(client);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        *self.shutdown_tx.write().await = Some(shutdown_tx);

        // Start event loop in background
        let callback = self.callback.clone();
        let is_connected = self.is_connected.clone();

        tokio::spawn(async move {
            Self::handle_events(eventloop, callback, is_connected, shutdown_rx).await;
        });

        info!("MQTT channel started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        // Send shutdown signal
        let tx = self.shutdown_tx.write().await.take();
        if let Some(tx) = tx {
            let _ = tx.send(()).await;
        }

        // Clear client
        *self.client.write().await = None;
        self.is_connected.store(false, Ordering::SeqCst);

        info!("MQTT channel stopped");
        Ok(())
    }

    async fn add_topic(&self, topic: &str) -> Result<()> {
        self.topics.write().await.push(topic.to_string());

        // If already connected, subscribe immediately
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            client
                .subscribe(topic, QoS::AtMostOnce)
                .await
                .map_err(|e| Error::MqttSubscribe(e.to_string()))?;
        }

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }
}
