use async_trait::async_trait;
use serde_json::Value;

use crate::domain::message::CommonTopicReceiver;
use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::Result;

/// Trait for MQTT message handlers.
///
/// Each handler declares which `ThingMessageType` variants it handles,
/// and provides an async `handle` method that processes the message.
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming MQTT message.
    ///
    /// `topic` contains parsed topic components;
    /// `msg` is the deserialized JSON payload.
    async fn handle(&self, topic: &TopicInfo, msg: CommonTopicReceiver<Value>) -> Result<()>;

    /// The message types this handler is interested in.
    fn message_types(&self) -> &[ThingMessageType];
}

/// A no-op handler used as a placeholder during bootstrap.
pub struct NoopHandler {
    types: Vec<ThingMessageType>,
}

impl NoopHandler {
    pub fn new(types: Vec<ThingMessageType>) -> Self {
        Self { types }
    }

    /// Create a NoopHandler that handles all ThingMessageType variants.
    pub fn all() -> Self {
        Self {
            types: vec![
                ThingMessageType::EventProperty,
                ThingMessageType::EventCustom,
                ThingMessageType::PropertyState,
                ThingMessageType::ServiceReply,
                ThingMessageType::PropertySetReply,
                ThingMessageType::Register,
                ThingMessageType::DynamicRegister,
                ThingMessageType::Pong,
                ThingMessageType::Ntp,
                ThingMessageType::UpdateTopo,
                ThingMessageType::DeviceModel,
                ThingMessageType::DeviceRequest,
                ThingMessageType::Config,
                ThingMessageType::Discovery,
            ],
        }
    }
}

#[async_trait]
impl MessageHandler for NoopHandler {
    async fn handle(&self, topic: &TopicInfo, _msg: CommonTopicReceiver<Value>) -> Result<()> {
        tracing::debug!(
            product_key = %topic.product_key,
            device_id = %topic.device_id,
            category = %topic.category,
            "NoopHandler: message received but not processed"
        );
        Ok(())
    }

    fn message_types(&self) -> &[ThingMessageType] {
        &self.types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_handler_all() {
        let handler = NoopHandler::all();
        assert_eq!(handler.message_types().len(), 14);
    }

    #[test]
    fn test_noop_handler_specific() {
        let handler = NoopHandler::new(vec![
            ThingMessageType::EventProperty,
            ThingMessageType::Pong,
        ]);
        assert_eq!(handler.message_types().len(), 2);
    }
}
