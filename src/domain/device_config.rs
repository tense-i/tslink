use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device configuration data.
///
/// Maps to table `device_config`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub id: Option<i64>,
    pub product_key: String,
    pub device_id: String,
    #[serde(default)]
    pub version: Option<i64>,
    #[serde(default)]
    pub config_json: Option<serde_json::Value>,
    #[serde(default)]
    pub gmt_create: Option<DateTime<Utc>>,
    #[serde(default)]
    pub gmt_modified: Option<DateTime<Utc>>,
}

impl DeviceConfig {
    pub fn new(product_key: String, device_id: String) -> Self {
        Self {
            id: None,
            product_key,
            device_id,
            version: Some(1),
            config_json: None,
            gmt_create: None,
            gmt_modified: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_config_new() {
        let config = DeviceConfig::new("pk001".to_string(), "did001".to_string());
        assert_eq!(config.product_key, "pk001");
        assert_eq!(config.device_id, "did001");
        assert_eq!(config.version, Some(1));
    }
}
