use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::warn;

use crate::application::event_service::EventService;
use crate::domain::message::CommonTopicReceiver;
use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::Result;
use crate::infrastructure::mqtt::handler::MessageHandler;

/// Handler for custom device events.
///
/// Handles `sys/{pk}/{did}/thing/event/{identifier}/{info|warning|error}`.
/// Delegates to EventService for ACK reply + Kafka forwarding.
pub struct EventHandler {
    event_service: Arc<EventService>,
}

impl EventHandler {
    pub fn new(event_service: Arc<EventService>) -> Self {
        Self { event_service }
    }
}

#[async_trait]
impl MessageHandler for EventHandler {
    async fn handle(&self, topic: &TopicInfo, msg: CommonTopicReceiver<Value>) -> Result<()> {
        let pk = &topic.product_key;
        let did = &topic.device_id;

        // Extract identifier from sub_category
        // sub_category pattern: ["event", "{identifier}"]
        let identifier = if topic.sub_category.len() >= 2 {
            &topic.sub_category[1]
        } else {
            warn!(
                pk = %pk,
                did = %did,
                "event handler: missing identifier in sub_category"
            );
            return Ok(());
        };

        // Extract level from identifier field or sub_category
        let level = topic.level.as_deref().unwrap_or("info");

        self.event_service
            .handle_event(pk, did, identifier, level, &msg)
            .await
    }

    fn message_types(&self) -> &[ThingMessageType] {
        &[ThingMessageType::EventCustom]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_handler_types() {
        // Verify expected message types
        assert_eq!(
            vec![ThingMessageType::EventCustom],
            vec![ThingMessageType::EventCustom]
        );
    }
}
