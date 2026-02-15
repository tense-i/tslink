//! Default implementation of TslinkClient

use std::sync::Arc;
use parking_lot::Mutex;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::oneshot;
use tracing::debug;

use super::TslinkClient;
use crate::adapter::{MessageAdapter, ReplyHandler};
use crate::channel::{MessageChannel, MqttChannel, MqttConfig};
use crate::enums::{CommunicationChannel, EventType};
use crate::error::{Error, Result};
use crate::message::service::{
    DeviceServiceRequest, DeviceServiceResponse, PlatformResponseCallback,
    PlatformServiceRequest, PlatformServiceResponse, ServiceExecutor,
    ServiceResponseCallback,
};
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
        publish_qos: crate::enums::QoS,
        subscribe_qos: crate::enums::QoS,
    ) -> Self {
        let adapter = Arc::new(MessageAdapter::new());

        let config = MqttConfig {
            endpoint,
            product_key: product_key.clone(),
            device_id: device_id.clone(),
            username,
            password,
            publish_qos,
            subscribe_qos,
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

    // ==================== Topic Helpers ====================

    fn property_topic(&self) -> String {
        format!(
            "sys/{}/{}/thing/event/property/post",
            self.product_key, self.device_id
        )
    }

    fn property_topic_for(&self, product_key: &str, device_id: &str) -> String {
        format!("sys/{}/{}/thing/event/property/post", product_key, device_id)
    }

    fn event_topic(&self, event_name: &str, event_type: EventType) -> String {
        format!(
            "sys/{}/{}/thing/event/{}/{}",
            self.product_key, self.device_id, event_name, event_type
        )
    }

    fn platform_service_topic(&self, identity: &str, pk: &str, did: &str) -> String {
        format!("sys/{}/{}/platform/service/{}/post", pk, did, identity)
    }

    fn device_service_topic(&self, identity: &str, pk: &str, did: &str) -> String {
        format!("sys/{}/{}/thing/service/{}/post", pk, did, identity)
    }
}

#[async_trait]
impl TslinkClient for DefaultTslinkClient {
    // ==================== Property ====================

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

    // ==================== Event ====================

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

    // ==================== Platform Push Executor ====================

    fn set_platform_push_unified_executor(
        &self,
        executor: ServiceExecutor,
        _product_key: &str,
        _device_id: &str,
    ) {
        self.adapter.set_unified_executor(executor);
    }

    fn set_platform_push_specific_executor(
        &self,
        identifier: &str,
        executor: ServiceExecutor,
        _product_key: &str,
        _device_id: &str,
    ) {
        let method = format!("platform.service.{}.post", identifier);
        self.adapter.add_service_executor(&method, executor);
    }

    // ==================== Platform Service Invoke ====================

    async fn platform_service_invoke_sync(
        &self,
        request: PlatformServiceRequest,
        timeout_ms: i32,
    ) -> Result<PlatformServiceResponse> {
        let pk = if request.product_key.is_empty() {
            &self.product_key
        } else {
            &request.product_key
        };
        let did = if request.device_id.is_empty() {
            &self.device_id
        } else {
            &request.device_id
        };

        let method = format!("platform.service.{}.post", request.service_identifier);
        let data: Value = serde_json::from_slice(&request.param_data)
            .unwrap_or(Value::Null);
        let msg = CommonMessage::new(&method, data);

        // Use oneshot channel for sync wait
        let (tx, rx) = oneshot::channel::<PlatformServiceResponse>();
        let tx = Mutex::new(Some(tx));
        let handler = ReplyHandler::Platform(Arc::new(move |resp| {
            if let Some(tx) = tx.lock().take() {
                let _ = tx.send(resp);
            }
        }));
        self.adapter.add_reply_handler(&msg.tid, handler);

        let json = msg.to_json()?;
        let topic = self.platform_service_topic(&request.service_identifier, pk, did);

        debug!("Invoking platform service sync {}: {}", topic, json);
        self.channel.send(&topic, &json).await?;

        // Wait with timeout
        let timeout = tokio::time::Duration::from_millis(timeout_ms.max(0) as u64);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_)) => Err(Error::Timeout("platform service reply channel closed".into())),
            Err(_) => Err(Error::Timeout(format!(
                "platform service invoke timeout after {}ms",
                timeout_ms
            ))),
        }
    }

    async fn platform_service_invoke_async(
        &self,
        request: PlatformServiceRequest,
        callback: PlatformResponseCallback,
    ) -> Result<()> {
        let pk = if request.product_key.is_empty() {
            &self.product_key
        } else {
            &request.product_key
        };
        let did = if request.device_id.is_empty() {
            &self.device_id
        } else {
            &request.device_id
        };

        let method = format!("platform.service.{}.post", request.service_identifier);
        let data: Value = serde_json::from_slice(&request.param_data)
            .unwrap_or(Value::Null);
        let msg = CommonMessage::new(&method, data);

        self.adapter
            .add_reply_handler(&msg.tid, ReplyHandler::Platform(callback));

        let json = msg.to_json()?;
        let topic = self.platform_service_topic(&request.service_identifier, pk, did);

        debug!("Invoking platform service async {}: {}", topic, json);
        self.channel.send(&topic, &json).await
    }

    // ==================== Device Service Executor ====================

    fn set_service_unified_executor(
        &self,
        executor: ServiceExecutor,
        _channel: CommunicationChannel,
        _product_key: &str,
        _device_id: &str,
    ) {
        self.adapter.set_unified_executor(executor);
    }

    fn set_service_specific_executor(
        &self,
        identifier: &str,
        executor: ServiceExecutor,
        _channel: CommunicationChannel,
        _product_key: &str,
        _device_id: &str,
    ) {
        let method = format!("service.{}.post", identifier);
        self.adapter.add_service_executor(&method, executor);
    }

    // ==================== Device Service Invoke ====================

    async fn device_service_invoke_sync(
        &self,
        request: DeviceServiceRequest,
        product_key: &str,
        device_id: &str,
        timeout_ms: i32,
    ) -> Result<DeviceServiceResponse> {
        let method = format!("service.{}.post", request.service_identifier);
        let data: Value = serde_json::from_slice(&request.param_data)
            .unwrap_or(Value::Null);
        let msg = CommonMessage::new(&method, data);

        let (tx, rx) = oneshot::channel::<DeviceServiceResponse>();
        let tx = Mutex::new(Some(tx));
        let handler = ReplyHandler::Device(Arc::new(move |resp| {
            if let Some(tx) = tx.lock().take() {
                let _ = tx.send(resp);
            }
        }));
        self.adapter.add_reply_handler(&msg.tid, handler);

        let json = msg.to_json()?;
        let topic = self.device_service_topic(&request.service_identifier, product_key, device_id);

        debug!("Invoking device service sync {}: {}", topic, json);
        self.channel.send(&topic, &json).await?;

        let timeout = tokio::time::Duration::from_millis(timeout_ms.max(0) as u64);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_)) => Err(Error::Timeout("device service reply channel closed".into())),
            Err(_) => Err(Error::Timeout(format!(
                "device service invoke timeout after {}ms",
                timeout_ms
            ))),
        }
    }

    async fn device_service_invoke_async(
        &self,
        request: DeviceServiceRequest,
        product_key: &str,
        device_id: &str,
        callback: ServiceResponseCallback,
    ) -> Result<()> {
        let method = format!("service.{}.post", request.service_identifier);
        let data: Value = serde_json::from_slice(&request.param_data)
            .unwrap_or(Value::Null);
        let msg = CommonMessage::new(&method, data);

        self.adapter
            .add_reply_handler(&msg.tid, ReplyHandler::Device(callback));

        let json = msg.to_json()?;
        let topic = self.device_service_topic(&request.service_identifier, product_key, device_id);

        debug!("Invoking device service async {}: {}", topic, json);
        self.channel.send(&topic, &json).await
    }

    // ==================== Property Set Handler ====================

    fn set_property_set_executor(&self, executor: ServiceExecutor) {
        self.adapter
            .add_service_executor("thing.properties.set", executor.clone());
        self.adapter
            .add_service_executor("service.property.set", executor);
    }

    // ==================== Lifecycle ====================

    async fn start(&self) -> Result<()> {
        self.channel.start().await
    }

    async fn release(&self) -> Result<()> {
        self.adapter.release();
        self.channel.stop().await
    }

    fn get_channel(&self) -> CommunicationChannel {
        CommunicationChannel::Remote
    }
}
