use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, warn};

use crate::application::handlers::discovery_handler::DiscoveryHandler;
use crate::domain::message::{CommonTopicReceiver, CommonTopicResponse};
use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::Result;
use crate::infrastructure::mqtt::handler::MessageHandler;
use crate::infrastructure::mqtt::publisher::MessagePublisher;

/// MQTT handler for device discovery messages.
pub struct DiscoveryMqttHandler {
    discovery_handler: Arc<DiscoveryHandler>,
    publisher: Arc<MessagePublisher>,
}

impl DiscoveryMqttHandler {
    pub fn new(discovery_handler: Arc<DiscoveryHandler>, publisher: Arc<MessagePublisher>) -> Self {
        Self {
            discovery_handler,
            publisher,
        }
    }
}

#[async_trait]
impl MessageHandler for DiscoveryMqttHandler {
    async fn handle(&self, topic: &TopicInfo, msg: CommonTopicReceiver<Value>) -> Result<()> {
        let sub_path = topic.sub_category.join("/");
        let identifier = topic.identifier.as_deref().unwrap_or("");

        debug!(
            product_key = %topic.product_key,
            device_id = %topic.device_id,
            sub_path = %sub_path,
            identifier = %identifier,
            "DiscoveryMqttHandler processing message"
        );

        let result = match (sub_path.as_str(), identifier) {
            ("discovery", "list") | ("discovery/list", "post") => {
                self.discovery_handler.handle_list(topic).await
            }
            ("discovery", "sub_devices") | ("discovery/sub_devices", "post") => {
                self.discovery_handler.handle_sub_devices(topic).await
            }
            ("discovery", "refresh") | ("discovery/refresh", "post") => {
                self.discovery_handler.handle_refresh(topic).await
            }
            _ => {
                warn!(
                    product_key = %topic.product_key,
                    device_id = %topic.device_id,
                    sub_path = %sub_path,
                    identifier = %identifier,
                    "Unknown discovery operation"
                );
                return Ok(());
            }
        };

        match result {
            Ok(data) => {
                let reply_topic = format!(
                    "sys/{}/{}/thing/discovery/{}_reply",
                    topic.product_key,
                    topic.device_id,
                    identifier.trim_end_matches("_reply")
                );
                let response = CommonTopicResponse::reply(&msg, data);
                self.publisher.publish(&reply_topic, &response).await?;
            }
            Err(e) => {
                warn!(
                    product_key = %topic.product_key,
                    device_id = %topic.device_id,
                    error = %e,
                    "Discovery handler error"
                );
            }
        }

        Ok(())
    }

    fn message_types(&self) -> &[ThingMessageType] {
        &[ThingMessageType::Discovery]
    }
}
