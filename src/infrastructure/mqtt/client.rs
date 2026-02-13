use dashmap::DashMap;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

use crate::config::MqttConfig;
use crate::error::{Result, TsLinkError};

/// Incoming MQTT message — raw topic + payload bytes.
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub topic: String,
    pub payload: Vec<u8>,
}

/// MQTT client wrapping rumqttc AsyncClient + EventLoop.
///
/// Provides connect / subscribe / publish / spawn_event_loop methods.
pub struct MqttClient {
    client: AsyncClient,
    eventloop: Option<EventLoop>,
    config: MqttConfig,
}

impl MqttClient {
    /// Create a new MQTT client from configuration.
    pub fn new(config: &MqttConfig) -> Self {
        let mut opts = MqttOptions::new(&config.client_id, &config.host, config.port);

        opts.set_keep_alive(Duration::from_secs(config.keep_alive_secs));
        opts.set_clean_session(config.clean_session);
        opts.set_inflight(config.inflight);

        // Set credentials if provided
        if !config.username.is_empty() {
            opts.set_credentials(&config.username, &config.password);
        }

        let (client, eventloop) = AsyncClient::new(opts, config.max_packet_size);

        Self {
            client,
            eventloop: Some(eventloop),
            config: config.clone(),
        }
    }

    /// Subscribe to all configured topic filters.
    pub async fn subscribe_all(&self) -> Result<()> {
        for topic in &self.config.subscribe_topics {
            self.client
                .subscribe(topic, QoS::AtLeastOnce)
                .await
                .map_err(|e| TsLinkError::Mqtt(format!("subscribe failed for {}: {}", topic, e)))?;
            info!(topic = %topic, "subscribed to topic");
        }
        Ok(())
    }

    /// Subscribe to a single topic.
    pub async fn subscribe(&self, topic: &str, qos: QoS) -> Result<()> {
        self.client
            .subscribe(topic, qos)
            .await
            .map_err(|e| TsLinkError::Mqtt(format!("subscribe failed for {}: {}", topic, e)))?;
        Ok(())
    }

    /// Publish a message to a topic.
    pub async fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<()> {
        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .await?;
        Ok(())
    }

    /// Publish a retained message.
    pub async fn publish_retained(&self, topic: &str, payload: Vec<u8>) -> Result<()> {
        self.client
            .publish(topic, QoS::AtLeastOnce, true, payload)
            .await?;
        Ok(())
    }

    /// Get a clone of the underlying AsyncClient for publishing from other contexts.
    pub fn client_handle(&self) -> AsyncClient {
        self.client.clone()
    }

    /// Spawn the event loop processor.
    ///
    /// Returns a receiver channel for incoming publish messages.
    /// The event loop runs in a background tokio task with reconnect logic.
    ///
    /// If `sync_waiters` is provided, incoming messages are checked for a
    /// matching `tid` field in the JSON payload. If a sync waiter exists for
    /// that `tid`, the waiter is resolved directly (PostSync pattern) and the
    /// message is NOT forwarded to the channel.
    pub fn spawn_event_loop(
        &mut self,
        sync_waiters: Option<Arc<DashMap<String, oneshot::Sender<Vec<u8>>>>>,
    ) -> Result<mpsc::Receiver<IncomingMessage>> {
        let mut eventloop = self
            .eventloop
            .take()
            .ok_or_else(|| TsLinkError::Mqtt("event loop already taken".to_string()))?;

        let (tx, rx) = mpsc::channel(4096);

        tokio::spawn(async move {
            info!("MQTT event loop started");
            loop {
                match eventloop.poll().await {
                    Ok(event) => {
                        if let Event::Incoming(Packet::Publish(publish)) = event {
                            let payload = publish.payload.to_vec();

                            // Try PostSync resolution: check if payload has a `tid` that
                            // matches a pending synchronous waiter.
                            if let Some(ref waiters) = sync_waiters {
                                if let Some(tid) = extract_tid(&payload) {
                                    if let Some((_, sender)) = waiters.remove(&tid) {
                                        debug!(tid = %tid, "resolved sync waiter");
                                        let _ = sender.send(payload);
                                        continue;
                                    }
                                }
                            }

                            // Normal message — forward to channel
                            let msg = IncomingMessage {
                                topic: publish.topic.clone(),
                                payload,
                            };

                            if tx.send(msg).await.is_err() {
                                error!("message channel closed, stopping event loop");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "MQTT connection error, will retry...");
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                }
            }
            info!("MQTT event loop stopped");
        });

        Ok(rx)
    }
}

/// Extract the `tid` field from a JSON payload (best-effort, non-fatal).
fn extract_tid(payload: &[u8]) -> Option<String> {
    // Fast path: try to parse as a JSON object and extract `tid`
    let v: serde_json::Value = serde_json::from_slice(payload).ok()?;
    v.get("tid")?.as_str().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MqttConfig;

    fn test_mqtt_config() -> MqttConfig {
        MqttConfig {
            host: "127.0.0.1".to_string(),
            port: 1883,
            client_id: "test-client".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            keep_alive_secs: 30,
            clean_session: true,
            max_packet_size: 65536,
            inflight: 100,
            subscribe_topics: vec!["sys/+/+/#".to_string()],
        }
    }

    #[test]
    fn test_mqtt_client_creation() {
        let config = test_mqtt_config();
        let client = MqttClient::new(&config);
        assert!(client.eventloop.is_some());
    }

    #[tokio::test]
    async fn test_spawn_event_loop() {
        let config = test_mqtt_config();
        let mut client = MqttClient::new(&config);
        let rx = client.spawn_event_loop(None);
        assert!(rx.is_ok());
        // Event loop is now taken
        assert!(client.eventloop.is_none());
    }

    #[test]
    fn test_extract_tid() {
        let payload = br#"{"tid":"abc-123","method":"thing.event.property.post"}"#;
        assert_eq!(extract_tid(payload), Some("abc-123".to_string()));
    }

    #[test]
    fn test_extract_tid_missing() {
        let payload = br#"{"method":"thing.event.property.post"}"#;
        assert_eq!(extract_tid(payload), None);
    }

    #[test]
    fn test_extract_tid_invalid_json() {
        let payload = b"not json";
        assert_eq!(extract_tid(payload), None);
    }
}
