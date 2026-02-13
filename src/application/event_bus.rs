use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::debug;

/// Default broadcast channel capacity.
const DEFAULT_CAPACITY: usize = 4096;

/// Device event types pushed via WebSocket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeviceEvent {
    /// Device came online.
    Online {
        product_key: String,
        device_id: String,
        timestamp: i64,
    },
    /// Device went offline.
    Offline {
        product_key: String,
        device_id: String,
        timestamp: i64,
    },
    /// Device properties updated.
    PropertyUpdate {
        product_key: String,
        device_id: String,
        properties: Value,
        timestamp: i64,
    },
    /// Device event reported.
    EventReport {
        product_key: String,
        device_id: String,
        identifier: String,
        params: Value,
        timestamp: i64,
    },
}

impl DeviceEvent {
    /// Get the product_key of this event.
    pub fn product_key(&self) -> &str {
        match self {
            DeviceEvent::Online { product_key, .. }
            | DeviceEvent::Offline { product_key, .. }
            | DeviceEvent::PropertyUpdate { product_key, .. }
            | DeviceEvent::EventReport { product_key, .. } => product_key,
        }
    }

    /// Get the device_id of this event.
    pub fn device_id(&self) -> &str {
        match self {
            DeviceEvent::Online { device_id, .. }
            | DeviceEvent::Offline { device_id, .. }
            | DeviceEvent::PropertyUpdate { device_id, .. }
            | DeviceEvent::EventReport { device_id, .. } => device_id,
        }
    }

    fn now() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    pub fn online(product_key: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self::Online {
            product_key: product_key.into(),
            device_id: device_id.into(),
            timestamp: Self::now(),
        }
    }

    pub fn offline(product_key: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self::Offline {
            product_key: product_key.into(),
            device_id: device_id.into(),
            timestamp: Self::now(),
        }
    }

    pub fn property_update(
        product_key: impl Into<String>,
        device_id: impl Into<String>,
        properties: Value,
    ) -> Self {
        Self::PropertyUpdate {
            product_key: product_key.into(),
            device_id: device_id.into(),
            properties,
            timestamp: Self::now(),
        }
    }

    pub fn event_report(
        product_key: impl Into<String>,
        device_id: impl Into<String>,
        identifier: impl Into<String>,
        params: Value,
    ) -> Self {
        Self::EventReport {
            product_key: product_key.into(),
            device_id: device_id.into(),
            identifier: identifier.into(),
            params,
            timestamp: Self::now(),
        }
    }
}

/// In-process event bus for broadcasting device events to WebSocket clients.
///
/// Uses `tokio::broadcast` — multiple subscribers, single publisher pattern.
/// Slow consumers that fall behind will miss messages (lagged).
#[derive(Clone)]
pub struct DeviceEventBus {
    sender: Arc<broadcast::Sender<DeviceEvent>>,
}

impl DeviceEventBus {
    /// Create a new event bus with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new event bus with specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender: Arc::new(sender),
        }
    }

    /// Publish an event to all subscribers.
    ///
    /// Returns the number of active receivers, or 0 if none.
    pub fn publish(&self, event: DeviceEvent) -> usize {
        debug!(
            event_type = %serde_json::to_string(&event).unwrap_or_default(),
            "event bus publish"
        );
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribe to the event bus. Returns a receiver that yields DeviceEvent.
    pub fn subscribe(&self) -> broadcast::Receiver<DeviceEvent> {
        self.sender.subscribe()
    }

    /// Get the current number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for DeviceEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = DeviceEventBus::new();
        let mut rx = bus.subscribe();

        let event = DeviceEvent::online("pk001", "did001");
        let receivers = bus.publish(event);
        assert_eq!(receivers, 1);

        let received = rx.recv().await.unwrap();
        assert_eq!(received.product_key(), "pk001");
        assert_eq!(received.device_id(), "did001");
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = DeviceEventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 2);

        bus.publish(DeviceEvent::offline("pk001", "did001"));

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert_eq!(e1.product_key(), "pk001");
        assert_eq!(e2.product_key(), "pk001");
    }

    #[test]
    fn test_event_serialization() {
        let event = DeviceEvent::property_update("pk001", "did001", serde_json::json!({"temp": 25}));
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"property_update\""));
        assert!(json.contains("\"product_key\":\"pk001\""));
        assert!(json.contains("\"temp\":25"));
    }

    #[test]
    fn test_no_subscribers_publish() {
        let bus = DeviceEventBus::new();
        let receivers = bus.publish(DeviceEvent::online("pk001", "did001"));
        assert_eq!(receivers, 0);
    }
}
