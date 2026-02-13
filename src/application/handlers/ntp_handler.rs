use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::application::ntp_service::NtpService;
use crate::domain::message::CommonTopicReceiver;
use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::Result;
use crate::infrastructure::mqtt::handler::MessageHandler;

/// Handler for NTP synchronization requests.
///
/// Handles `sys/{pk}/{did}/thing/ntp/post`
pub struct NtpHandler {
    ntp_service: Arc<NtpService>,
}

impl NtpHandler {
    pub fn new(ntp_service: Arc<NtpService>) -> Self {
        Self { ntp_service }
    }
}

#[async_trait]
impl MessageHandler for NtpHandler {
    async fn handle(&self, topic: &TopicInfo, msg: CommonTopicReceiver<Value>) -> Result<()> {
        self.ntp_service
            .handle_ntp(&topic.product_key, &topic.device_id, &msg)
            .await?;
        debug!(pk = %topic.product_key, did = %topic.device_id, "NTP request handled");
        Ok(())
    }

    fn message_types(&self) -> &[ThingMessageType] {
        &[ThingMessageType::Ntp]
    }
}
