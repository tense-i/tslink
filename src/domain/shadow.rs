use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device shadow — cached device properties in Redis.
///
/// Redis key format: `IOT_DEVICE_PROPERTIES_KEY_{product_key}_{device_id}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceShadow {
    pub product_key: String,
    pub device_id: String,
    /// Current property values (JSON object)
    pub properties: serde_json::Value,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Shadow service configuration — stored in DB table `iot_device_shadow`.
///
/// When a device comes online, shadow services are replayed by
/// sending the stored payload via the configured method.
///
/// Maps from Java: `IotDeviceShadow` + `IotDeviceShadowProperties`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowServiceConfig {
    pub product_key: String,
    /// Service method to invoke on device online
    pub method: String,
}

/// Shadow service property record — per-device shadow method payload.
///
/// Maps from Java: `IotDeviceShadowProperties`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowServiceProperty {
    pub product_key: String,
    pub device_id: String,
    /// Service method name
    pub method: String,
    /// Stored payload (JSON string)
    pub payload: String,
}

impl DeviceShadow {
    /// Create a new empty shadow.
    pub fn new(product_key: String, device_id: String) -> Self {
        Self {
            product_key,
            device_id,
            properties: serde_json::json!({}),
            updated_at: Utc::now(),
        }
    }

    /// Merge new properties into the shadow.
    /// New values override existing ones; keys not in `new_props` are retained.
    pub fn merge_properties(&mut self, new_props: &serde_json::Value) {
        if let (Some(existing), Some(incoming)) =
            (self.properties.as_object_mut(), new_props.as_object())
        {
            for (key, value) in incoming {
                existing.insert(key.clone(), value.clone());
            }
        }
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_new() {
        let shadow = DeviceShadow::new("pk001".to_string(), "did001".to_string());
        assert_eq!(shadow.product_key, "pk001");
        assert_eq!(shadow.properties, serde_json::json!({}));
    }

    #[test]
    fn test_shadow_merge_properties() {
        let mut shadow = DeviceShadow::new("pk001".to_string(), "did001".to_string());

        shadow.merge_properties(&serde_json::json!({"temperature": 25.5}));
        assert_eq!(shadow.properties["temperature"], 25.5);

        shadow.merge_properties(&serde_json::json!({"humidity": 60, "temperature": 26.0}));
        assert_eq!(shadow.properties["temperature"], 26.0);
        assert_eq!(shadow.properties["humidity"], 60);
    }

    #[test]
    fn test_shadow_serde() {
        let shadow = DeviceShadow::new("pk001".to_string(), "did001".to_string());
        let json = serde_json::to_string(&shadow).unwrap();
        let deserialized: DeviceShadow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.product_key, "pk001");
    }

    #[test]
    fn test_shadow_service_config() {
        let config = ShadowServiceConfig {
            product_key: "pk001".to_string(),
            method: "set_brightness".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("set_brightness"));
    }
}
