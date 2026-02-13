use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Inbound MQTT message envelope.
///
/// Maps from Java: `CommonTopicReceiver<T>`.
/// Generic over the `data` payload type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonTopicReceiver<T = serde_json::Value> {
    /// Transaction ID — unique per request
    #[serde(default)]
    pub tid: Option<String>,
    /// Business ID — scoped to the long-lived session
    #[serde(default)]
    pub bid: Option<String>,
    /// Protocol version
    #[serde(default = "default_version")]
    pub version: String,
    /// Unix timestamp (milliseconds)
    #[serde(default)]
    pub timestamp: Option<i64>,
    /// RPC method name
    #[serde(default)]
    pub method: Option<String>,
    /// Product key (required for WebSocket relay)
    #[serde(default, rename = "productKey")]
    pub product_key: Option<String>,
    /// Device identifier
    #[serde(default, rename = "deviceId")]
    pub device_id: Option<String>,
    /// Payload data
    pub data: T,
    /// Response code
    #[serde(default)]
    pub code: Option<String>,
    /// Response message
    #[serde(default)]
    pub message: Option<String>,
}

/// Outbound MQTT response envelope.
///
/// Maps from Java: `CommonTopicResponse<T>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonTopicResponse<T = serde_json::Value> {
    /// Transaction ID — echoed from receiver
    #[serde(default)]
    pub tid: Option<String>,
    /// Business ID
    #[serde(default)]
    pub bid: Option<String>,
    /// RPC method name
    #[serde(default)]
    pub method: Option<String>,
    /// Payload data
    pub data: T,
    /// Unix timestamp (milliseconds)
    #[serde(default)]
    pub timestamp: Option<i64>,
    /// Protocol version
    #[serde(default = "default_version")]
    pub version: String,
    /// Response code
    #[serde(default)]
    pub code: Option<String>,
    /// Response message
    #[serde(default)]
    pub message: Option<String>,
    /// Product key
    #[serde(default, rename = "productKey")]
    pub product_key: Option<String>,
    /// Device identifier
    #[serde(default, rename = "deviceId")]
    pub device_id: Option<String>,
}

/// Service reply inner envelope.
///
/// Maps from Java: `ServiceReply<T>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceReply<T = serde_json::Value> {
    pub result: Option<i32>,
    pub info: Option<T>,
    pub output: Option<T>,
}

/// Response code constants.
pub struct ResponseCode;

impl ResponseCode {
    pub const SUCCESS: &'static str = "200";
    pub const REFUSE: &'static str = "1";
    pub const ERROR: &'static str = "0";
}

fn default_version() -> String {
    "1.0".to_string()
}

impl<T> CommonTopicReceiver<T> {
    /// Create a new receiver message with data.
    pub fn new(data: T) -> Self
    where
        T: Default,
    {
        Self {
            tid: Some(Uuid::new_v4().to_string()),
            bid: None,
            version: default_version(),
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            method: None,
            product_key: None,
            device_id: None,
            data,
            code: None,
            message: None,
        }
    }
}

impl<T> CommonTopicResponse<T> {
    /// Create a success reply from a receiver message.
    pub fn reply(receiver: &CommonTopicReceiver<impl std::fmt::Debug>, data: T) -> Self {
        Self {
            tid: receiver.tid.clone(),
            bid: receiver.bid.clone(),
            method: receiver.method.clone(),
            data,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            version: default_version(),
            code: Some(ResponseCode::SUCCESS.to_string()),
            message: Some("success".to_string()),
            product_key: receiver.product_key.clone(),
            device_id: receiver.device_id.clone(),
        }
    }

    /// Create an error reply.
    pub fn error(receiver: &CommonTopicReceiver<impl std::fmt::Debug>, data: T, msg: &str) -> Self {
        Self {
            tid: receiver.tid.clone(),
            bid: receiver.bid.clone(),
            method: receiver.method.clone(),
            data,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            version: default_version(),
            code: Some(ResponseCode::ERROR.to_string()),
            message: Some(msg.to_string()),
            product_key: receiver.product_key.clone(),
            device_id: receiver.device_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receiver_deserialize() {
        let json = r#"{
            "tid": "abc-123",
            "bid": "biz-001",
            "version": "1.0",
            "timestamp": 1700000000000,
            "method": "thing.event.property.post",
            "productKey": "pk001",
            "deviceId": "did001",
            "data": {"temperature": 25.5},
            "code": "200",
            "message": "success"
        }"#;

        let msg: CommonTopicReceiver<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(msg.tid.as_deref(), Some("abc-123"));
        assert_eq!(msg.product_key.as_deref(), Some("pk001"));
        assert_eq!(msg.device_id.as_deref(), Some("did001"));
        assert_eq!(msg.data["temperature"], 25.5);
    }

    #[test]
    fn test_response_reply() {
        let receiver = CommonTopicReceiver::<serde_json::Value> {
            tid: Some("tid-001".into()),
            bid: Some("bid-001".into()),
            version: "1.0".into(),
            timestamp: Some(1700000000000),
            method: Some("test.method".into()),
            product_key: Some("pk001".into()),
            device_id: Some("did001".into()),
            data: serde_json::json!({}),
            code: None,
            message: None,
        };

        let response = CommonTopicResponse::reply(&receiver, serde_json::json!({"ok": true}));
        assert_eq!(response.tid.as_deref(), Some("tid-001"));
        assert_eq!(response.code.as_deref(), Some("200"));
        assert!(response.timestamp.is_some());
    }

    #[test]
    fn test_response_serialize() {
        let response = CommonTopicResponse {
            tid: Some("tid-001".into()),
            bid: None,
            method: Some("test".into()),
            data: serde_json::json!({"value": 42}),
            timestamp: Some(1700000000000),
            version: "1.0".into(),
            code: Some("200".into()),
            message: Some("success".into()),
            product_key: Some("pk001".into()),
            device_id: Some("did001".into()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"tid\":\"tid-001\""));
        assert!(json.contains("\"productKey\":\"pk001\""));
    }

    #[test]
    fn test_service_reply_deserialize() {
        let json = r#"{"result": 0, "info": {"status": "ok"}, "output": null}"#;
        let reply: ServiceReply<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(reply.result, Some(0));
        assert!(reply.info.is_some());
    }
}
