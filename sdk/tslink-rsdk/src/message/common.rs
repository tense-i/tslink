//! Common message structure for IoT protocol

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Standard message format for IoT communication
///
/// This structure matches the JASmartSDK CommonMessage format:
/// - `tid`: Transaction ID for request-response correlation
/// - `bid`: Batch ID for grouping related messages
/// - `version`: Protocol version (default "1.0")
/// - `timestamp`: Message timestamp in milliseconds
/// - `method`: Method name (e.g., "event.property.post")
/// - `data`: Message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonMessage {
    /// Transaction ID for correlation
    pub tid: String,
    /// Batch ID (optional for incoming messages)
    #[serde(default)]
    pub bid: String,
    /// Protocol version
    #[serde(default = "default_version")]
    pub version: String,
    /// Timestamp in milliseconds
    #[serde(default)]
    pub timestamp: i64,
    /// Method name
    pub method: String,
    /// Data payload (can be "data" or "params")
    #[serde(alias = "params", default)]
    pub data: Value,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl CommonMessage {
    /// Create a new CommonMessage with generated IDs
    pub fn new(method: impl Into<String>, data: Value) -> Self {
        Self {
            tid: Uuid::new_v4().to_string(),
            bid: Uuid::new_v4().to_string(),
            version: "1.0".to_string(),
            timestamp: chrono_timestamp_ms(),
            method: method.into(),
            data,
        }
    }

    /// Create a new CommonMessage with specific tid
    pub fn with_tid(tid: String, method: impl Into<String>, data: Value) -> Self {
        Self {
            tid,
            bid: Uuid::new_v4().to_string(),
            version: "1.0".to_string(),
            timestamp: chrono_timestamp_ms(),
            method: method.into(),
            data,
        }
    }

    /// Create a builder for CommonMessage
    pub fn builder() -> CommonMessageBuilder {
        CommonMessageBuilder::default()
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Builder for CommonMessage
#[derive(Default)]
pub struct CommonMessageBuilder {
    tid: Option<String>,
    bid: Option<String>,
    version: Option<String>,
    timestamp: Option<i64>,
    method: Option<String>,
    data: Option<Value>,
}

impl CommonMessageBuilder {
    /// Set transaction ID
    pub fn tid(mut self, tid: impl Into<String>) -> Self {
        self.tid = Some(tid.into());
        self
    }

    /// Set batch ID
    pub fn bid(mut self, bid: impl Into<String>) -> Self {
        self.bid = Some(bid.into());
        self
    }

    /// Set protocol version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set timestamp
    pub fn timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Set method name
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Set data payload
    pub fn data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Build the CommonMessage
    pub fn build(self) -> CommonMessage {
        CommonMessage {
            tid: self.tid.unwrap_or_else(|| Uuid::new_v4().to_string()),
            bid: self.bid.unwrap_or_else(|| Uuid::new_v4().to_string()),
            version: self.version.unwrap_or_else(|| "1.0".to_string()),
            timestamp: self.timestamp.unwrap_or_else(chrono_timestamp_ms),
            method: self.method.unwrap_or_default(),
            data: self.data.unwrap_or(Value::Null),
        }
    }
}

/// Get current timestamp in milliseconds
fn chrono_timestamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_common_message_new() {
        let msg = CommonMessage::new("event.property.post", json!({"temp": 25}));
        assert_eq!(msg.method, "event.property.post");
        assert_eq!(msg.version, "1.0");
        assert!(!msg.tid.is_empty());
    }

    #[test]
    fn test_common_message_builder() {
        let msg = CommonMessage::builder()
            .tid("test-tid")
            .method("test.method")
            .data(json!({"key": "value"}))
            .build();

        assert_eq!(msg.tid, "test-tid");
        assert_eq!(msg.method, "test.method");
    }

    #[test]
    fn test_common_message_serialization() {
        let msg = CommonMessage::new("test", json!({}));
        let json_str = msg.to_json().unwrap();
        let parsed = CommonMessage::from_json(&json_str).unwrap();
        assert_eq!(msg.tid, parsed.tid);
    }
}
