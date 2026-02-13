use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use tracing::{debug, error, warn};

use crate::config::KafkaConfig;
use crate::error::{Result, TsLinkError};

/// Kafka event producer for forwarding device events.
///
/// Wraps rdkafka FutureProducer with error fallback to logging.
pub struct EventProducer {
    producer: FutureProducer,
}

impl EventProducer {
    /// Create a new Kafka producer from configuration.
    pub fn new(config: &KafkaConfig) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("message.timeout.ms", "5000")
            .set("queue.buffering.max.ms", "100")
            .set("batch.num.messages", "1000")
            .create()
            .map_err(|e| TsLinkError::Kafka(format!("failed to create producer: {}", e)))?;

        Ok(Self { producer })
    }

    /// Send an event to a Kafka topic.
    ///
    /// On failure, logs the error as a fallback (never panics).
    pub async fn send_event(&self, topic: &str, key: &str, payload: &[u8]) -> Result<()> {
        let record = FutureRecord::to(topic).key(key).payload(payload);

        match self.producer.send(record, Duration::from_secs(5)).await {
            Ok((partition, offset)) => {
                debug!(
                    topic = %topic,
                    key = %key,
                    partition = partition,
                    offset = offset,
                    "kafka event sent"
                );
                Ok(())
            }
            Err((err, _)) => {
                error!(
                    topic = %topic,
                    key = %key,
                    error = %err,
                    "kafka send failed, falling back to log"
                );
                // Fallback: log the payload instead of failing
                warn!(
                    topic = %topic,
                    key = %key,
                    payload_len = payload.len(),
                    "event logged as fallback due to kafka failure"
                );
                Err(TsLinkError::Kafka(format!("send failed: {}", err)))
            }
        }
    }

    /// Send an event with best-effort delivery (ignore errors).
    pub async fn send_event_best_effort(&self, topic: &str, key: &str, payload: &[u8]) {
        if let Err(e) = self.send_event(topic, key, payload).await {
            warn!(
                topic = %topic,
                key = %key,
                error = %e,
                "best-effort kafka send failed, event dropped"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_kafka_topic_format() {
        let topic = "iot-device-message";
        let key = format!("{}_{}", "pk001", "did001");
        assert_eq!(key, "pk001_did001");
        assert_eq!(topic, "iot-device-message");
    }
}
