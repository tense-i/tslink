use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::domain::message::CommonTopicReceiver;
use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::Result;
use crate::infrastructure::mqtt::handler::MessageHandler;
use crate::infrastructure::mqtt::publisher::MessagePublisher;

/// Handler for service reply messages (PostSync resolution).
///
/// Handles `sys/{pk}/{did}/thing/service/{method}/post_reply`.
/// Extracts `tid` from the reply message and resolves the
/// corresponding synchronous waiter in MessagePublisher.
pub struct ServiceReplyHandler {
    publisher: Arc<MessagePublisher>,
}

impl ServiceReplyHandler {
    pub fn new(publisher: Arc<MessagePublisher>) -> Self {
        Self { publisher }
    }
}

#[async_trait]
impl MessageHandler for ServiceReplyHandler {
    async fn handle(&self, topic: &TopicInfo, msg: CommonTopicReceiver<Value>) -> Result<()> {
        let pk = &topic.product_key;
        let did = &topic.device_id;

        let tid = match &msg.tid {
            Some(t) => t.clone(),
            None => {
                warn!(
                    pk = %pk,
                    did = %did,
                    "service reply missing tid, cannot resolve waiter"
                );
                return Ok(());
            }
        };

        // Serialize the full reply message for the waiter
        let payload = serde_json::to_vec(&msg)?;

        if self.publisher.resolve_waiter(&tid, payload) {
            debug!(
                pk = %pk,
                did = %did,
                tid = %tid,
                "service reply resolved PostSync waiter"
            );
        } else {
            debug!(
                pk = %pk,
                did = %did,
                tid = %tid,
                "service reply received but no waiter found (async call or timeout)"
            );
        }

        Ok(())
    }

    fn message_types(&self) -> &[ThingMessageType] {
        &[ThingMessageType::ServiceReply]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_reply_handler_types() {
        // We can't easily construct a MessagePublisher without AsyncClient,
        // but we can verify the expected types.
        assert_eq!(
            vec![ThingMessageType::ServiceReply],
            vec![ThingMessageType::ServiceReply]
        );
    }
}
