//! Service request/response types aligned with ja-IOT-SDK-cpp

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::enums::CommunicationChannel;

/// Get current timestamp in milliseconds
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ==================== Request Types ====================

/// Platform service request (mirrors JaIotPlatformServiceRequest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformServiceRequest {
    /// Communication channel
    #[serde(default)]
    pub channel: CommunicationChannel,
    /// Service identifier
    pub service_identifier: String,
    /// Parameter data (raw bytes, typically UTF-8 JSON)
    pub param_data: Vec<u8>,
    /// Product key of the target device
    #[serde(default)]
    pub product_key: String,
    /// Device ID of the target device
    #[serde(default)]
    pub device_id: String,
}

impl PlatformServiceRequest {
    /// Create a new PlatformServiceRequest
    pub fn new(service_identifier: impl Into<String>, param_data: Vec<u8>) -> Self {
        Self {
            channel: CommunicationChannel::default(),
            service_identifier: service_identifier.into(),
            param_data,
            product_key: String::new(),
            device_id: String::new(),
        }
    }

    /// Set the target device
    pub fn with_device(mut self, product_key: impl Into<String>, device_id: impl Into<String>) -> Self {
        self.product_key = product_key.into();
        self.device_id = device_id.into();
        self
    }

    /// Set the communication channel
    pub fn with_channel(mut self, channel: CommunicationChannel) -> Self {
        self.channel = channel;
        self
    }
}

/// Device service request (mirrors JaIotDeviceServiceRequest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceServiceRequest {
    /// Communication channel
    #[serde(default)]
    pub channel: CommunicationChannel,
    /// Service identifier
    pub service_identifier: String,
    /// Parameter data (raw bytes, typically UTF-8 JSON)
    pub param_data: Vec<u8>,
    /// Service timestamp in milliseconds
    #[serde(default)]
    pub service_timestamp_ms: i64,
}

impl DeviceServiceRequest {
    /// Create a new DeviceServiceRequest
    pub fn new(service_identifier: impl Into<String>, param_data: Vec<u8>) -> Self {
        Self {
            channel: CommunicationChannel::default(),
            service_identifier: service_identifier.into(),
            param_data,
            service_timestamp_ms: 0,
        }
    }

    /// Set the communication channel
    pub fn with_channel(mut self, channel: CommunicationChannel) -> Self {
        self.channel = channel;
        self
    }
}

// ==================== Response Types ====================

/// Platform service response (mirrors JaIotPlatformServiceResponse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformServiceResponse {
    /// Communication channel
    #[serde(default)]
    pub channel: CommunicationChannel,
    /// Service identifier
    pub service_identifier: String,
    /// Result code (0 = success)
    pub result: i32,
    /// Response parameter data
    pub param_data: Vec<u8>,
    /// Service timestamp in milliseconds
    #[serde(default)]
    pub service_timestamp_ms: i64,
}

impl PlatformServiceResponse {
    /// Create a success response
    pub fn success(service_identifier: impl Into<String>, param_data: Vec<u8>) -> Self {
        Self {
            channel: CommunicationChannel::default(),
            service_identifier: service_identifier.into(),
            result: 0,
            param_data,
            service_timestamp_ms: now_millis(),
        }
    }

    /// Create an error response
    pub fn error(service_identifier: impl Into<String>, result: i32) -> Self {
        Self {
            channel: CommunicationChannel::default(),
            service_identifier: service_identifier.into(),
            result,
            param_data: Vec::new(),
            service_timestamp_ms: now_millis(),
        }
    }
}

/// Device service response (mirrors JaIotDeviceServiceResponse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceServiceResponse {
    /// Communication channel
    #[serde(default)]
    pub channel: CommunicationChannel,
    /// Service identifier
    pub service_identifier: String,
    /// Result code (0 = success)
    pub result: i32,
    /// Response parameter data
    pub param_data: Vec<u8>,
    /// Service timestamp in milliseconds
    #[serde(default)]
    pub service_timestamp_ms: i64,
}

impl DeviceServiceResponse {
    /// Create a success response
    pub fn success(service_identifier: impl Into<String>, param_data: Vec<u8>) -> Self {
        Self {
            channel: CommunicationChannel::default(),
            service_identifier: service_identifier.into(),
            result: 0,
            param_data,
            service_timestamp_ms: now_millis(),
        }
    }

    /// Create an error response
    pub fn error(service_identifier: impl Into<String>, result: i32) -> Self {
        Self {
            channel: CommunicationChannel::default(),
            service_identifier: service_identifier.into(),
            result,
            param_data: Vec::new(),
            service_timestamp_ms: now_millis(),
        }
    }
}

// ==================== Callback Types ====================

/// Callback for replying to a service invocation
///
/// # Arguments
/// * `result` - Result code (0 = success)
/// * `data` - Response data as bytes
pub type ReplyCallback = Arc<dyn Fn(i32, Vec<u8>) + Send + Sync>;

/// Executor for handling incoming service invocations (mirrors ServiceExecutor)
///
/// # Arguments
/// * `request` - The incoming device service request
/// * `reply` - Callback to send the reply
pub type ServiceExecutor = Arc<dyn Fn(DeviceServiceRequest, ReplyCallback) + Send + Sync>;

/// Callback for platform service async response (mirrors PlatformResponseCallback)
pub type PlatformResponseCallback = Arc<dyn Fn(PlatformServiceResponse) + Send + Sync>;

/// Callback for device service async response (mirrors ServiceResponseCallback)
pub type ServiceResponseCallback = Arc<dyn Fn(DeviceServiceResponse) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_service_request_new() {
        let req = PlatformServiceRequest::new("test_service", b"hello".to_vec())
            .with_device("pk1", "did1")
            .with_channel(CommunicationChannel::Remote);

        assert_eq!(req.service_identifier, "test_service");
        assert_eq!(req.param_data, b"hello");
        assert_eq!(req.product_key, "pk1");
        assert_eq!(req.device_id, "did1");
        assert_eq!(req.channel, CommunicationChannel::Remote);
    }

    #[test]
    fn test_device_service_request_new() {
        let req = DeviceServiceRequest::new("dev_svc", b"data".to_vec());
        assert_eq!(req.service_identifier, "dev_svc");
        assert_eq!(req.param_data, b"data");
        assert_eq!(req.service_timestamp_ms, 0);
    }

    #[test]
    fn test_platform_service_response_success() {
        let resp = PlatformServiceResponse::success("svc1", b"ok".to_vec());
        assert_eq!(resp.result, 0);
        assert_eq!(resp.service_identifier, "svc1");
        assert_eq!(resp.param_data, b"ok");
    }

    #[test]
    fn test_device_service_response_error() {
        let resp = DeviceServiceResponse::error("svc2", -1);
        assert_eq!(resp.result, -1);
        assert!(resp.param_data.is_empty());
    }

    #[test]
    fn test_reply_callback() {
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        let cb: ReplyCallback = Arc::new(move |code, _data| {
            assert_eq!(code, 0);
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        cb(0, b"ok".to_vec());
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
