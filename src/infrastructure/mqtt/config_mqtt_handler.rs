use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, warn};

use crate::application::handlers::config_handler::ConfigHandler;
use crate::domain::message::{CommonTopicReceiver, CommonTopicResponse};
use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::Result;
use crate::infrastructure::mqtt::handler::MessageHandler;
use crate::infrastructure::mqtt::publisher::MessagePublisher;

/// MQTT handler for device configuration messages.
pub struct ConfigMqttHandler {
    config_handler: Arc<ConfigHandler>,
    publisher: Arc<MessagePublisher>,
}

impl ConfigMqttHandler {
    pub fn new(config_handler: Arc<ConfigHandler>, publisher: Arc<MessagePublisher>) -> Self {
        Self {
            config_handler,
            publisher,
        }
    }
}

#[async_trait]
impl MessageHandler for ConfigMqttHandler {
    async fn handle(&self, topic: &TopicInfo, msg: CommonTopicReceiver<Value>) -> Result<()> {
        let sub_path = topic.sub_category.join("/");
        let identifier = topic.identifier.as_deref().unwrap_or("");

        debug!(
            product_key = %topic.product_key,
            device_id = %topic.device_id,
            sub_path = %sub_path,
            identifier = %identifier,
            "ConfigMqttHandler processing message"
        );

        let payload = msg.data.clone();

        let result = match (sub_path.as_str(), identifier) {
            ("config", "get") | ("config/get", "post") => {
                self.config_handler.handle_query(topic).await
            }
            ("config", "push") | ("config/push", "post") => {
                self.config_handler.handle_update(topic, &payload).await
            }
            ("config/version", "get") | ("config/version/get", "post") => {
                self.config_handler.handle_version_query(topic).await
            }
            _ => {
                warn!(
                    product_key = %topic.product_key,
                    device_id = %topic.device_id,
                    sub_path = %sub_path,
                    identifier = %identifier,
                    "Unknown config operation"
                );
                return Ok(());
            }
        };

        match result {
            Ok(data) => {
                let reply_topic = format!(
                    "sys/{}/{}/thing/config/{}_reply",
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
                    "Config handler error"
                );
            }
        }

        Ok(())
    }

    fn message_types(&self) -> &[ThingMessageType] {
        &[ThingMessageType::Config]
    }
}
