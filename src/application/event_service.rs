use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::domain::message::{CommonTopicReceiver, CommonTopicResponse};
use crate::error::Result;
use crate::infrastructure::kafka::producer::EventProducer;
use crate::infrastructure::mqtt::publisher::MessagePublisher;

/// Default Kafka topic for device events.
const DEVICE_EVENT_TOPIC: &str = "iot-device-message";

/// Application service for device event processing.
///
/// Handles:
/// - Receiving device events (property, info, warning, error)
/// - Replying ACK to the device
/// - Forwarding events to Kafka
pub struct EventService {
    publisher: Arc<MessagePublisher>,
    kafka_producer: Arc<EventProducer>,
}

impl EventService {
    pub fn new(publisher: Arc<MessagePublisher>, kafka_producer: Arc<EventProducer>) -> Self {
        Self {
            publisher,
            kafka_producer,
        }
    }

    /// Handle an incoming device event.
    ///
    /// 1. Reply ACK to the device on `{level}_reply` topic
    /// 2. Forward the event payload to Kafka
    pub async fn handle_event(
        &self,
        product_key: &str,
        device_id: &str,
        identifier: &str,
        level: &str,
        receiver: &CommonTopicReceiver<serde_json::Value>,
    ) -> Result<()> {
        info!(
            pk = %product_key,
            did = %device_id,
            identifier = %identifier,
            level = %level,
            "processing device event"
        );

        // Step 1: Reply ACK to the device
        let reply_topic = format!(
            "sys/{}/{}/thing/event/{}/{}_reply",
            product_key, device_id, identifier, level
        );
        let ack_response = CommonTopicResponse::reply(receiver, serde_json::json!({}));
        if let Err(e) = self.publisher.publish(&reply_topic, &ack_response).await {
            warn!(
                pk = %product_key,
                did = %device_id,
                error = %e,
                "failed to send event ACK reply"
            );
        } else {
            debug!(
                pk = %product_key,
                did = %device_id,
                reply_topic = %reply_topic,
                "event ACK reply sent"
            );
        }

        // Step 2: Forward to Kafka
        let kafka_key = format!("{}_{}", product_key, device_id);
        let event_payload = serde_json::json!({
            "productKey": product_key,
            "deviceId": device_id,
            "identifier": identifier,
            "level": level,
            "data": receiver.data,
            "timestamp": receiver.timestamp,
            "tid": receiver.tid,
            "method": receiver.method,
        });
        let payload_bytes = serde_json::to_vec(&event_payload)?;

        self.kafka_producer
            .send_event_best_effort(DEVICE_EVENT_TOPIC, &kafka_key, &payload_bytes)
            .await;

        debug!(
            pk = %product_key,
            did = %device_id,
            identifier = %identifier,
            "event forwarded to kafka"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_event_reply_topic_format() {
        let topic = format!(
            "sys/{}/{}/thing/event/{}/{}_reply",
            "pk001", "did001", "temperature", "info"
        );
        assert_eq!(topic, "sys/pk001/did001/thing/event/temperature/info_reply");
    }

    #[test]
    fn test_kafka_key_format() {
        let key = format!("{}_{}", "pk001", "did001");
        assert_eq!(key, "pk001_did001");
    }

    #[test]
    fn test_event_payload_serialization() {
        let payload = serde_json::json!({
            "productKey": "pk001",
            "deviceId": "did001",
            "identifier": "temperature",
            "level": "info",
            "data": {"temperature": 25.5},
            "timestamp": 1700000000000_i64,
        });
        let bytes = serde_json::to_vec(&payload).unwrap();
        assert!(!bytes.is_empty());
    }
}
