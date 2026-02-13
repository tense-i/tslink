use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::application::device_service::DeviceService;
use crate::domain::message::CommonTopicReceiver;
use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::Result;
use crate::infrastructure::mqtt::handler::MessageHandler;

/// Handler for heartbeat (pong) messages.
///
/// Handles `sys/{pk}/{did}/thing/pong/post`
pub struct HeartbeatHandler {
    device_service: Arc<DeviceService>,
}

impl HeartbeatHandler {
    pub fn new(device_service: Arc<DeviceService>) -> Self {
        Self { device_service }
    }
}

#[async_trait]
impl MessageHandler for HeartbeatHandler {
    async fn handle(&self, topic: &TopicInfo, _msg: CommonTopicReceiver<Value>) -> Result<()> {
        self.device_service
            .handle_heartbeat(&topic.product_key, &topic.device_id)
            .await?;
        debug!(pk = %topic.product_key, did = %topic.device_id, "heartbeat processed");
        Ok(())
    }

    fn message_types(&self) -> &[ThingMessageType] {
        &[ThingMessageType::Pong]
    }
}
