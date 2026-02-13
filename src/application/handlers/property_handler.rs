use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::application::shadow_service::ShadowService;
use crate::domain::message::CommonTopicReceiver;
use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::Result;
use crate::infrastructure::mqtt::handler::MessageHandler;

/// Handler for property event reports.
///
/// Handles `sys/{pk}/{did}/thing/event/property/post`
/// Updates the device shadow in Redis with reported properties.
pub struct PropertyHandler {
    shadow_service: Arc<ShadowService>,
}

impl PropertyHandler {
    pub fn new(shadow_service: Arc<ShadowService>) -> Self {
        Self { shadow_service }
    }
}

#[async_trait]
impl MessageHandler for PropertyHandler {
    async fn handle(&self, topic: &TopicInfo, msg: CommonTopicReceiver<Value>) -> Result<()> {
        let pk = &topic.product_key;
        let did = &topic.device_id;

        // The `data` field contains reported properties
        self.shadow_service
            .update_properties(pk, did, &msg.data)
            .await?;

        debug!(pk = %pk, did = %did, "property report processed, shadow updated");
        Ok(())
    }

    fn message_types(&self) -> &[ThingMessageType] {
        &[ThingMessageType::EventProperty]
    }
}
