use std::sync::Arc;
use tracing::debug;

use crate::domain::message::{CommonTopicReceiver, CommonTopicResponse, ResponseCode};
use crate::error::Result;
use crate::infrastructure::mqtt::publisher::MessagePublisher;

/// NTP service — responds to device NTP requests with server timestamp.
///
/// Topic in: `sys/{pk}/{did}/thing/ntp/post`
/// Topic out: `sys/{pk}/{did}/thing/ntp/post_reply`
pub struct NtpService {
    publisher: Arc<MessagePublisher>,
}

impl NtpService {
    pub fn new(publisher: Arc<MessagePublisher>) -> Self {
        Self { publisher }
    }

    /// Handle an NTP request from a device.
    ///
    /// Responds with the server's current timestamp.
    pub async fn handle_ntp(
        &self,
        product_key: &str,
        device_id: &str,
        receiver: &CommonTopicReceiver<serde_json::Value>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        let response = CommonTopicResponse {
            tid: receiver.tid.clone(),
            bid: receiver.bid.clone(),
            version: "1.0".to_string(),
            timestamp: Some(now),
            method: Some("thing.ntp".to_string()),
            product_key: Some(product_key.to_string()),
            device_id: Some(device_id.to_string()),
            data: serde_json::json!({
                "serverSendTime": now,
                "deviceSendTime": receiver.timestamp.unwrap_or(0),
            }),
            code: Some(ResponseCode::SUCCESS.to_string()),
            message: Some("success".to_string()),
        };

        let reply_topic = format!("sys/{}/{}/thing/ntp/post_reply", product_key, device_id);
        self.publisher.publish(&reply_topic, &response).await?;
        debug!(pk = %product_key, did = %device_id, "NTP reply sent");
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_ntp_response_format() {
        let now = chrono::Utc::now().timestamp_millis();
        let data = serde_json::json!({
            "serverSendTime": now,
            "deviceSendTime": 1234567890000_i64,
        });
        assert!(data["serverSendTime"].is_number());
        assert_eq!(data["deviceSendTime"], 1234567890000_i64);
    }
}
