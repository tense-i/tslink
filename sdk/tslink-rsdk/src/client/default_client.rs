//! Default implementation of TslinkClient

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::debug;

use super::{ServiceCallback, ServiceReplyCallback, TslinkClient};
use crate::adapter::MessageAdapter;
use crate::channel::{MessageChannel, MqttChannel, MqttConfig};
use crate::enums::EventType;
use crate::error::Result;
use crate::message::CommonMessage;

/// Default implementation of TslinkClient using MQTT channel
#[derive(Debug)]
pub struct DefaultTslinkClient {
    product_key: String,
    device_id: String,
    channel: Arc<MqttChannel>,
    adapter: Arc<MessageAdapter>,
}

impl DefaultTslinkClient {
    /// Create a new DefaultTslinkClient
    pub fn new(
        endpoint: String,
        product_key: String,
        device_id: String,
        username: String,
        password: String,
    ) -> Self {
        let adapter = Arc::new(MessageAdapter::new());

        let config = MqttConfig {
            endpoint,
            product_key: product_key.clone(),
            device_id: device_id.clone(),
            username,
            password,
            ..Default::default()
        };

        let channel = Arc::new(MqttChannel::new(config, adapter.clone()));

        Self {
            product_key,
            device_id,
            channel,
            adapter,
        }
    }

    /// Get the property post topic
    fn property_topic(&self) -> String {
        format!(
            "sys/{}/{}/thing/event/property/post",
            self.product_key, self.device_id
        )
    }

    /// Get the property post topic for a specific device
    fn property_topic_for(&self, product_key: &str, device_id: &str) -> String {
        format!("sys/{}/{}/thing/event/property/post", product_key, device_id)
    }

    /// Get the event post topic
    fn event_topic(&self, event_name: &str, event_type: EventType) -> String {
        format!(
            "sys/{}/{}/thing/event/{}/{}",
            self.product_key, self.device_id, event_name, event_type
        )
    }

    /// Get the platform service invoke topic
    fn platform_service_topic(&self, identity: &str) -> String {
        format!(
            "sys/{}/{}/platform/service/{}/post",
            self.product_key, self.device_id, identity
        )
    }

    /// Get the platform service invoke topic for a specific device
    fn platform_service_topic_for(
        &self,
        product_key: &str,
        device_id: &str,
        identity: &str,
    ) -> String {
        format!(
            "sys/{}/{}/platform/service/{}/post",
            product_key, device_id, identity
        )
    }
}

#[async_trait]
impl TslinkClient for DefaultTslinkClient {
    async fn thing_property_post(&self, data: Value) -> Result<()> {
        let msg = CommonMessage::new("event.property.post", data);
        let json = msg.to_json()?;
        let topic = self.property_topic();

        debug!("Posting property to {}: {}", topic, json);
        self.channel.send(&topic, &json).await
    }

    async fn thing_property_post_for(
        &self,
        product_key: &str,
        device_id: &str,
        data: Value,
    ) -> Result<()> {
        let msg = CommonMessage::new("event.property.post", data);
        let json = msg.to_json()?;
        let topic = self.property_topic_for(product_key, device_id);

        debug!("Posting property for {}/{} to {}", product_key, device_id, topic);
        self.channel.send(&topic, &json).await
    }

    async fn thing_event_post(
        &self,
        event_type: EventType,
        event_name: &str,
        data: Value,
    ) -> Result<()> {
        let method = format!("event.{}.{}", event_name, event_type);
        let msg = CommonMessage::new(&method, data);
        let json = msg.to_json()?;
        let topic = self.event_topic(event_name, event_type);

        debug!("Posting event to {}: {}", topic, json);
        self.channel.send(&topic, &json).await
    }

    async fn thing_event_post_with_reply(
        &self,
        event_type: EventType,
        event_name: &str,
        data: Value,
        callback: ServiceReplyCallback,
    ) -> Result<()> {
        let method = format!("event.{}.{}", event_name, event_type);
        let msg = CommonMessage::new(&method, data);

        // Register callback for reply
        self.adapter.add_reply_callback(&msg.tid, callback);

        let json = msg.to_json()?;
        let topic = self.event_topic(event_name, event_type);

        debug!("Posting event with reply to {}: {}", topic, json);
        self.channel.send(&topic, &json).await
    }

    fn set_service_handle(&self, identity: &str, callback: ServiceCallback) {
        let method = format!("service.{}.post", identity);
        self.adapter.add_service_callback(&method, callback);
    }

    fn set_property_set_handle(&self, callback: ServiceCallback) {
        self.adapter
            .add_service_callback("thing.properties.set", callback.clone());
        self.adapter
            .add_service_callback("service.property.set", callback);
    }

    async fn platform_service_invoke(
        &self,
        identity: &str,
        request: Value,
        callback: ServiceReplyCallback,
    ) -> Result<()> {
        let method = format!("platform.service.{}.post", identity);
        let msg = CommonMessage::new(&method, request);

        // Register callback for reply
        self.adapter.add_reply_callback(&msg.tid, callback);

        let json = msg.to_json()?;
        let topic = self.platform_service_topic(identity);

        debug!("Invoking platform service {}: {}", topic, json);
        self.channel.send(&topic, &json).await
    }

    async fn platform_service_invoke_for(
        &self,
        product_key: &str,
        device_id: &str,
        identity: &str,
        request: Value,
        callback: ServiceReplyCallback,
    ) -> Result<()> {
        let method = format!("platform.service.{}.post", identity);
        let msg = CommonMessage::new(&method, request);

        // Register callback for reply
        self.adapter.add_reply_callback(&msg.tid, callback);

        let json = msg.to_json()?;
        let topic = self.platform_service_topic_for(product_key, device_id, identity);

        debug!(
            "Invoking platform service for {}/{} at {}: {}",
            product_key, device_id, topic, json
        );
        self.channel.send(&topic, &json).await
    }

    async fn start(&self) -> Result<()> {
        self.channel.start().await
    }

    async fn release(&self) -> Result<()> {
        self.adapter.release();
        self.channel.stop().await
    }
}
