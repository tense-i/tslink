use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::application::device_service::DeviceService;
use crate::domain::message::CommonTopicReceiver;
use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::Result;
use crate::infrastructure::mqtt::handler::MessageHandler;
use crate::infrastructure::mqtt::publisher::MessagePublisher;

/// Handler for device registration messages.
///
/// Handles:
/// - `sys/{pk}/{did}/thing/register/post` — static registration
/// - `sys/{pk}/{did}/thing/dynamic_register/post` — dynamic registration
pub struct RegisterHandler {
    device_service: Arc<DeviceService>,
    publisher: Arc<MessagePublisher>,
}

impl RegisterHandler {
    pub fn new(device_service: Arc<DeviceService>, publisher: Arc<MessagePublisher>) -> Self {
        Self {
            device_service,
            publisher,
        }
    }
}

#[async_trait]
impl MessageHandler for RegisterHandler {
    async fn handle(&self, topic: &TopicInfo, msg: CommonTopicReceiver<Value>) -> Result<()> {
        let pk = &topic.product_key;
        let did = &topic.device_id;

        // Determine if static or dynamic register based on sub_category
        let is_dynamic = topic.sub_category.iter().any(|s| s == "dynamic_register");

        let response = if is_dynamic {
            let device_name = msg.data.get("deviceName").and_then(|v| v.as_str());
            let product_secret = msg
                .data
                .get("productSecret")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            self.device_service
                .handle_dynamic_register(pk, did, device_name, product_secret)
                .await?
        } else {
            let secret = msg
                .data
                .get("deviceSecret")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let product_secret = msg
                .data
                .get("productSecret")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            self.device_service
                .handle_register(pk, did, secret, product_secret)
                .await?
        };

        // Publish reply
        let register_type = if is_dynamic {
            "dynamic_register"
        } else {
            "register"
        };
        let reply_topic = format!("sys/{}/{}/thing/{}/post_reply", pk, did, register_type);
        self.publisher.publish(&reply_topic, &response).await?;
        debug!(pk = %pk, did = %did, register_type = %register_type, "register reply sent");

        Ok(())
    }

    fn message_types(&self) -> &[ThingMessageType] {
        &[
            ThingMessageType::Register,
            ThingMessageType::DynamicRegister,
        ]
    }
}
