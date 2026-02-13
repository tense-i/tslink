use dashmap::DashMap;
use rumqttc::{AsyncClient, QoS};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::domain::message::CommonTopicResponse;
use crate::error::{Result, TsLinkError};

/// Message publisher with synchronous request-reply support (PostSync).
///
/// For synchronous service calls, registers a waiter keyed by `tid`.
/// When the eventloop receives a reply with matching `tid`, the waiter is woken up.
pub struct MessagePublisher {
    client: AsyncClient,
    /// Pending synchronous request waiters: tid -> oneshot sender
    waiters: Arc<DashMap<String, oneshot::Sender<Vec<u8>>>>,
}

impl MessagePublisher {
    /// Create a new publisher from an MQTT client handle.
    pub fn new(client: AsyncClient) -> Self {
        Self {
            client,
            waiters: Arc::new(DashMap::new()),
        }
    }

    /// Publish a response message to a topic (fire-and-forget).
    pub async fn publish<T: serde::Serialize>(
        &self,
        topic: &str,
        response: &CommonTopicResponse<T>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(response)?;
        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .await?;
        debug!(topic = %topic, "published message");
        Ok(())
    }

    /// Publish raw bytes to a topic.
    pub async fn publish_raw(&self, topic: &str, payload: Vec<u8>) -> Result<()> {
        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .await?;
        Ok(())
    }

    /// Publish and wait for a reply with matching `tid` (PostSync pattern).
    ///
    /// Returns the raw reply payload bytes, or times out.
    pub async fn publish_with_reply<T: serde::Serialize>(
        &self,
        topic: &str,
        response: &CommonTopicResponse<T>,
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let tid = response
            .tid
            .as_ref()
            .ok_or_else(|| TsLinkError::Mqtt("tid required for sync request".to_string()))?
            .clone();

        // Register waiter
        let (tx, rx) = oneshot::channel();
        self.waiters.insert(tid.clone(), tx);

        // Publish the request
        let payload = serde_json::to_vec(response)?;
        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .await?;
        debug!(topic = %topic, tid = %tid, "published sync request, waiting for reply");

        // Wait for reply or timeout
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(reply)) => {
                debug!(tid = %tid, "received sync reply");
                Ok(reply)
            }
            Ok(Err(_)) => {
                warn!(tid = %tid, "sync reply channel dropped");
                self.waiters.remove(&tid);
                Err(TsLinkError::Mqtt("reply channel dropped".to_string()))
            }
            Err(_) => {
                self.waiters.remove(&tid);
                Err(TsLinkError::Timeout {
                    operation: format!("sync reply for tid={}", tid),
                    duration_ms: timeout.as_millis() as u64,
                })
            }
        }
    }

    /// Resolve a pending synchronous waiter by tid.
    ///
    /// Called by the event loop when a reply message is received.
    pub fn resolve_waiter(&self, tid: &str, payload: Vec<u8>) -> bool {
        if let Some((_, tx)) = self.waiters.remove(tid) {
            if tx.send(payload).is_err() {
                warn!(tid = %tid, "waiter receiver dropped");
                return false;
            }
            return true;
        }
        false
    }

    /// Get count of pending waiters (for metrics/debugging).
    pub fn pending_count(&self) -> usize {
        self.waiters.len()
    }

    /// Get a reference to the waiters map for event loop integration.
    pub fn waiters(&self) -> Arc<DashMap<String, oneshot::Sender<Vec<u8>>>> {
        self.waiters.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rumqttc::MqttOptions;

    fn create_test_publisher() -> MessagePublisher {
        let opts = MqttOptions::new("test", "127.0.0.1", 1883);
        let (client, _eventloop) = AsyncClient::new(opts, 65536);
        MessagePublisher::new(client)
    }

    #[test]
    fn test_publisher_creation() {
        let publisher = create_test_publisher();
        assert_eq!(publisher.pending_count(), 0);
    }

    #[test]
    fn test_resolve_waiter_no_pending() {
        let publisher = create_test_publisher();
        assert!(!publisher.resolve_waiter("nonexistent", vec![1, 2, 3]));
    }

    #[test]
    fn test_resolve_waiter_success() {
        let publisher = create_test_publisher();
        let (tx, mut rx) = oneshot::channel();
        publisher.waiters.insert("tid-001".to_string(), tx);

        assert_eq!(publisher.pending_count(), 1);
        assert!(publisher.resolve_waiter("tid-001", vec![1, 2, 3]));
        assert_eq!(publisher.pending_count(), 0);

        // Verify the received data
        let data = rx.try_recv().unwrap();
        assert_eq!(data, vec![1, 2, 3]);
    }
}
