use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device status enumeration.
///
/// Maps from Java: `DeviceStatus` enum
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceStatus {
    /// 设备在线
    Online,
    /// 设备离线
    Offline,
    /// 设备故障
    Fault,
    /// 未激活
    #[default]
    NotActive,
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceStatus::Online => write!(f, "ONLINE"),
            DeviceStatus::Offline => write!(f, "OFFLINE"),
            DeviceStatus::Fault => write!(f, "FAULT"),
            DeviceStatus::NotActive => write!(f, "NOT_ACTIVE"),
        }
    }
}

/// Product node type enumeration.
///
/// Maps from Java: `ProductType` enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductType {
    /// 直连设备
    DirectDevice,
    /// 网关设备
    Gateway,
    /// 子设备
    SubDevice,
    /// 虚拟设备
    UnrealDevice,
}

/// IoT Device entity (v3).
///
/// Maps from Java table `iot_device` / `IotDevice.java`.
/// Contains only IoT-relevant fields — no business logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Option<i64>,
    pub product_id: Option<i64>,
    pub product_key: String,
    #[serde(default)]
    pub product_version: Option<String>,
    #[serde(default)]
    pub parent_product_key: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub device_id: String,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub device_secret: Option<String>,
    #[serde(default)]
    pub device_status: DeviceStatus,
    #[serde(default)]
    pub gmt_last_online: Option<DateTime<Utc>>,
    #[serde(default)]
    pub register_time: Option<DateTime<Utc>>,
    #[serde(default)]
    pub device_extend: Option<String>,
    #[serde(default)]
    pub org_code: Option<String>,
}

impl Device {
    /// Create a new Device with minimal required fields.
    pub fn new(product_key: String, device_id: String) -> Self {
        Self {
            id: None,
            product_id: None,
            product_key,
            product_version: None,
            parent_product_key: None,
            parent_id: None,
            device_id,
            device_name: None,
            device_secret: None,
            device_status: DeviceStatus::NotActive,
            gmt_last_online: None,
            register_time: None,
            device_extend: None,
            org_code: None,
        }
    }

    /// Check if this device is currently online.
    pub fn is_online(&self) -> bool {
        self.device_status == DeviceStatus::Online
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_new() {
        let device = Device::new("pk001".to_string(), "did001".to_string());
        assert_eq!(device.product_key, "pk001");
        assert_eq!(device.device_id, "did001");
        assert_eq!(device.device_status, DeviceStatus::NotActive);
        assert!(!device.is_online());
    }

    #[test]
    fn test_device_status_display() {
        assert_eq!(DeviceStatus::Online.to_string(), "ONLINE");
        assert_eq!(DeviceStatus::Offline.to_string(), "OFFLINE");
        assert_eq!(DeviceStatus::NotActive.to_string(), "NOT_ACTIVE");
    }

    #[test]
    fn test_device_status_serde() {
        let status = DeviceStatus::Online;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"ONLINE\"");

        let deserialized: DeviceStatus = serde_json::from_str("\"OFFLINE\"").unwrap();
        assert_eq!(deserialized, DeviceStatus::Offline);
    }

    #[test]
    fn test_device_serde_roundtrip() {
        let device = Device::new("pk001".to_string(), "did001".to_string());
        let json = serde_json::to_string(&device).unwrap();
        let deserialized: Device = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.product_key, "pk001");
        assert_eq!(deserialized.device_id, "did001");
    }
}
