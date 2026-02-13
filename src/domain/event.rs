use serde::{Deserialize, Serialize};

/// Domain events emitted by the system.
///
/// These events represent significant state changes in the IoT domain.
/// They can be used for internal pub/sub, audit logging, and Kafka forwarding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DomainEvent {
    /// Device came online
    DeviceOnline {
        product_key: String,
        device_id: String,
    },
    /// Device went offline
    DeviceOffline {
        product_key: String,
        device_id: String,
    },
    /// Device properties changed (shadow update)
    PropertyChanged {
        product_key: String,
        device_id: String,
        properties: serde_json::Value,
    },
    /// Service was invoked on a device
    ServiceInvoked {
        product_key: String,
        device_id: String,
        method: String,
    },
    /// Event received from a device
    EventReceived {
        product_key: String,
        device_id: String,
        identifier: String,
        level: String,
        data: serde_json::Value,
    },
    /// Device registered
    DeviceRegistered {
        product_key: String,
        device_id: String,
    },
    /// Link status changed
    LinkChanged {
        product_key: String,
        device_id: String,
        link_id: String,
        is_active: bool,
    },
}

impl DomainEvent {
    /// Get the product_key associated with this event.
    pub fn product_key(&self) -> &str {
        match self {
            DomainEvent::DeviceOnline { product_key, .. }
            | DomainEvent::DeviceOffline { product_key, .. }
            | DomainEvent::PropertyChanged { product_key, .. }
            | DomainEvent::ServiceInvoked { product_key, .. }
            | DomainEvent::EventReceived { product_key, .. }
            | DomainEvent::DeviceRegistered { product_key, .. }
            | DomainEvent::LinkChanged { product_key, .. } => product_key,
        }
    }

    /// Get the device_id associated with this event.
    pub fn device_id(&self) -> &str {
        match self {
            DomainEvent::DeviceOnline { device_id, .. }
            | DomainEvent::DeviceOffline { device_id, .. }
            | DomainEvent::PropertyChanged { device_id, .. }
            | DomainEvent::ServiceInvoked { device_id, .. }
            | DomainEvent::EventReceived { device_id, .. }
            | DomainEvent::DeviceRegistered { device_id, .. }
            | DomainEvent::LinkChanged { device_id, .. } => device_id,
        }
    }

    /// Get the event type name for logging/metrics.
    pub fn event_type(&self) -> &'static str {
        match self {
            DomainEvent::DeviceOnline { .. } => "device_online",
            DomainEvent::DeviceOffline { .. } => "device_offline",
            DomainEvent::PropertyChanged { .. } => "property_changed",
            DomainEvent::ServiceInvoked { .. } => "service_invoked",
            DomainEvent::EventReceived { .. } => "event_received",
            DomainEvent::DeviceRegistered { .. } => "device_registered",
            DomainEvent::LinkChanged { .. } => "link_changed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_event_accessors() {
        let event = DomainEvent::DeviceOnline {
            product_key: "pk001".to_string(),
            device_id: "did001".to_string(),
        };
        assert_eq!(event.product_key(), "pk001");
        assert_eq!(event.device_id(), "did001");
        assert_eq!(event.event_type(), "device_online");
    }

    #[test]
    fn test_domain_event_serde() {
        let event = DomainEvent::PropertyChanged {
            product_key: "pk001".to_string(),
            device_id: "did001".to_string(),
            properties: serde_json::json!({"temperature": 25.5}),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"PropertyChanged\""));
        assert!(json.contains("\"temperature\":25.5"));

        let deserialized: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.product_key(), "pk001");
    }

    #[test]
    fn test_event_received_serde() {
        let event = DomainEvent::EventReceived {
            product_key: "pk001".to_string(),
            device_id: "did001".to_string(),
            identifier: "fire_alarm".to_string(),
            level: "warning".to_string(),
            data: serde_json::json!({"value": true}),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type(), "event_received");
    }
}
