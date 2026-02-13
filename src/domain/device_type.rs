use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device type definition.
///
/// Maps to table `device_type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceType {
    pub id: Option<i64>,
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub gmt_create: Option<DateTime<Utc>>,
    #[serde(default)]
    pub gmt_modified: Option<DateTime<Utc>>,
}

impl DeviceType {
    pub fn new(code: String, name: String) -> Self {
        Self {
            id: None,
            code,
            name,
            description: None,
            gmt_create: None,
            gmt_modified: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_new() {
        let device_type = DeviceType::new("CAMERA".to_string(), "摄像头".to_string());
        assert_eq!(device_type.code, "CAMERA");
        assert_eq!(device_type.name, "摄像头");
    }
}
