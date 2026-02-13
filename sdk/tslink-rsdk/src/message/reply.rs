//! Reply message structure for IoT protocol

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Reply message from platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyMessage {
    /// Transaction ID for correlation
    pub tid: String,
    /// Batch ID
    pub bid: String,
    /// Response code (0 = success)
    pub code: i32,
    /// Response message
    pub message: String,
    /// Response data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ReplyMessage {
    /// Create a success reply
    pub fn success(tid: impl Into<String>, bid: impl Into<String>) -> Self {
        Self {
            tid: tid.into(),
            bid: bid.into(),
            code: 0,
            message: "success".to_string(),
            data: None,
        }
    }

    /// Create a success reply with data
    pub fn success_with_data(
        tid: impl Into<String>,
        bid: impl Into<String>,
        data: Value,
    ) -> Self {
        Self {
            tid: tid.into(),
            bid: bid.into(),
            code: 0,
            message: "success".to_string(),
            data: Some(data),
        }
    }

    /// Create an error reply
    pub fn error(
        tid: impl Into<String>,
        bid: impl Into<String>,
        code: i32,
        message: impl Into<String>,
    ) -> Self {
        Self {
            tid: tid.into(),
            bid: bid.into(),
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Check if the reply indicates success
    pub fn is_success(&self) -> bool {
        self.code == 0
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_reply_message_success() {
        let reply = ReplyMessage::success("tid-1", "bid-1");
        assert!(reply.is_success());
        assert_eq!(reply.code, 0);
    }

    #[test]
    fn test_reply_message_error() {
        let reply = ReplyMessage::error("tid-1", "bid-1", 500, "Internal error");
        assert!(!reply.is_success());
        assert_eq!(reply.code, 500);
    }

    #[test]
    fn test_reply_message_serialization() {
        let reply = ReplyMessage::success_with_data("tid-1", "bid-1", json!({"result": true}));
        let json_str = reply.to_json().unwrap();
        let parsed = ReplyMessage::from_json(&json_str).unwrap();
        assert_eq!(reply.tid, parsed.tid);
        assert!(parsed.data.is_some());
    }
}
