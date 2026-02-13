use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tracing::{debug, warn};

use crate::domain::message::CommonTopicReceiver;
use crate::domain::topic::{MessageType, ThingMessageType};
use crate::error::Result;
use crate::infrastructure::mqtt::handler::MessageHandler;
use crate::infrastructure::mqtt::topic_parser::{classify_thing_message, parse_topic};
use crate::telemetry::Metrics;

/// Routes incoming MQTT messages to the appropriate handler
/// based on parsed topic information and message type.
#[derive(Default)]
pub struct MessageRouter {
    handlers: HashMap<ThingMessageType, Arc<dyn MessageHandler>>,
    metrics: Option<Arc<Metrics>>,
}

impl MessageRouter {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach metrics for message counting.
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Register a handler. The handler declares which message types it handles.
    pub fn register(&mut self, handler: Arc<dyn MessageHandler>) {
        for msg_type in handler.message_types() {
            self.handlers.insert(msg_type.clone(), handler.clone());
        }
    }

    /// Route a raw MQTT message (topic string + payload bytes) to the
    /// appropriate handler.
    ///
    /// 1. Parse the topic string into TopicInfo.
    /// 2. Match the MessageType — only Thing messages are routed.
    /// 3. Classify the ThingMessageType.
    /// 4. Lookup the handler and dispatch.
    pub async fn route(&self, topic_str: &str, payload: &[u8]) -> Result<()> {
        // 1. Parse topic
        let topic = match parse_topic(topic_str) {
            Ok(t) => t,
            Err(e) => {
                warn!(topic = %topic_str, error = %e, "failed to parse topic, skipping");
                return Ok(());
            }
        };

        // 2. Only handle Thing messages in this router
        let msg_type = topic.message_type();
        match msg_type {
            Some(MessageType::Thing) => { /* continue */ }
            Some(MessageType::Platform) | Some(MessageType::App) => {
                debug!(
                    topic = %topic_str,
                    message_type = ?msg_type,
                    "non-Thing message, skipping router"
                );
                return Ok(());
            }
            None => {
                warn!(
                    topic = %topic_str,
                    "could not determine message type, skipping"
                );
                return Ok(());
            }
        }

        // 3. Classify into ThingMessageType
        let thing_type: ThingMessageType = match classify_thing_message(&topic) {
            Some(t) => t,
            None => {
                warn!(
                    topic = %topic_str,
                    category = %topic.category,
                    "unknown thing message type, no handler found"
                );
                return Ok(());
            }
        };

        // 4. Deserialize payload
        let msg: CommonTopicReceiver<Value> = match serde_json::from_slice(payload) {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    topic = %topic_str,
                    error = %e,
                    "failed to deserialize MQTT payload, skipping"
                );
                return Ok(());
            }
        };

        // 5. Record metrics
        if let Some(ref metrics) = self.metrics {
            metrics
                .mqtt_messages_total
                .with_label_values(&[&format!("{:?}", thing_type)])
                .inc();
        }

        // 6. Lookup handler and dispatch
        if let Some(handler) = self.handlers.get(&thing_type) {
            debug!(
                topic = %topic_str,
                message_type = ?thing_type,
                "routing message to handler"
            );
            handler.handle(&topic, msg).await?;
        } else {
            warn!(
                topic = %topic_str,
                message_type = ?thing_type,
                "no handler registered for message type"
            );
        }

        Ok(())
    }

    /// Number of registered handler slots.
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::mqtt::handler::NoopHandler;

    #[test]
    fn test_router_register() {
        let mut router = MessageRouter::new();
        let handler = Arc::new(NoopHandler::new(vec![
            ThingMessageType::EventProperty,
            ThingMessageType::Pong,
        ]));
        router.register(handler);
        assert_eq!(router.handler_count(), 2);
    }

    #[test]
    fn test_router_register_all() {
        let mut router = MessageRouter::new();
        let handler = Arc::new(NoopHandler::all());
        router.register(handler);
        assert_eq!(router.handler_count(), 14);
    }

    #[tokio::test]
    async fn test_route_property_post() {
        let mut router = MessageRouter::new();
        let handler = Arc::new(NoopHandler::all());
        router.register(handler);

        let topic = "sys/pk001/dev001/thing/event/property/post";
        let payload = br#"{"tid":"t1","method":"thing.event.property.post","version":"1.0","timestamp":1234567890}"#;
        let result = router.route(topic, payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_route_unknown_topic() {
        let router = MessageRouter::new();
        let topic = "unknown/topic/path";
        let payload = b"{}";
        // Should not error, just warn and skip
        let result = router.route(topic, payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_route_invalid_payload() {
        let mut router = MessageRouter::new();
        let handler = Arc::new(NoopHandler::all());
        router.register(handler);

        let topic = "sys/pk001/dev001/thing/event/property/post";
        let payload = b"not json";
        // Should not error, just warn and skip
        let result = router.route(topic, payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_route_platform_message_skipped() {
        let mut router = MessageRouter::new();
        let handler = Arc::new(NoopHandler::all());
        router.register(handler);

        let topic = "sys/pk001/dev001/platform/some/action";
        let payload = br#"{"tid":"t1"}"#;
        let result = router.route(topic, payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_route_service_reply() {
        let mut router = MessageRouter::new();
        let handler = Arc::new(NoopHandler::all());
        router.register(handler);

        let topic = "sys/pk001/dev001/thing/service/some_service/post_reply";
        let payload = br#"{"tid":"t1","method":"thing.service.some_service","version":"1.0","timestamp":1234567890,"code":"200","message":"success"}"#;
        let result = router.route(topic, payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_route_register() {
        let mut router = MessageRouter::new();
        let handler = Arc::new(NoopHandler::all());
        router.register(handler);

        let topic = "sys/pk001/dev001/thing/register/post";
        let payload =
            br#"{"tid":"t1","method":"thing.register","version":"1.0","timestamp":1234567890}"#;
        let result = router.route(topic, payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_route_ntp() {
        let mut router = MessageRouter::new();
        let handler = Arc::new(NoopHandler::all());
        router.register(handler);

        let topic = "sys/pk001/dev001/thing/ntp/post";
        let payload =
            br#"{"tid":"t1","method":"thing.ntp","version":"1.0","timestamp":1234567890}"#;
        let result = router.route(topic, payload).await;
        assert!(result.is_ok());
    }
}
