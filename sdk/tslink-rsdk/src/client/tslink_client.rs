//! TslinkClient trait definition

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::enums::EventType;
use crate::error::Result;
use crate::message::ReplyMessage;

/// Callback for handling service invocations from cloud
pub type ServiceCallback = Arc<dyn Fn(Value) -> Value + Send + Sync>;

/// Callback for handling service reply from cloud
pub type ServiceReplyCallback = Arc<dyn Fn(ReplyMessage) + Send + Sync>;

/// Tslink client trait for IoT device communication
///
/// This trait provides methods for:
/// - Property reporting
/// - Event reporting
/// - Service handling
/// - Platform service invocation
#[async_trait]
pub trait TslinkClient: Send + Sync {
    /// Report device properties to cloud
    ///
    /// # Arguments
    /// * `data` - Property data as JSON value
    async fn thing_property_post(&self, data: Value) -> Result<()>;

    /// Report device properties for a specific device (proxy mode)
    ///
    /// # Arguments
    /// * `product_key` - Target device's product key
    /// * `device_id` - Target device's ID
    /// * `data` - Property data as JSON value
    async fn thing_property_post_for(
        &self,
        product_key: &str,
        device_id: &str,
        data: Value,
    ) -> Result<()>;

    /// Report device event to cloud
    ///
    /// # Arguments
    /// * `event_type` - Type of event (Info, Warning, Error)
    /// * `event_name` - Name of the event
    /// * `data` - Event data as JSON value
    async fn thing_event_post(
        &self,
        event_type: EventType,
        event_name: &str,
        data: Value,
    ) -> Result<()>;

    /// Report device event with reply callback
    ///
    /// # Arguments
    /// * `event_type` - Type of event
    /// * `event_name` - Name of the event
    /// * `data` - Event data
    /// * `callback` - Callback for handling reply
    async fn thing_event_post_with_reply(
        &self,
        event_type: EventType,
        event_name: &str,
        data: Value,
        callback: ServiceReplyCallback,
    ) -> Result<()>;

    /// Register a service handler
    ///
    /// # Arguments
    /// * `identity` - Service identity/name
    /// * `callback` - Callback to handle service invocations
    fn set_service_handle(&self, identity: &str, callback: ServiceCallback);

    /// Register a property set handler
    ///
    /// # Arguments
    /// * `callback` - Callback to handle property set commands
    fn set_property_set_handle(&self, callback: ServiceCallback);

    /// Invoke a platform service
    ///
    /// # Arguments
    /// * `identity` - Service identity
    /// * `request` - Request data
    /// * `callback` - Callback for handling reply
    async fn platform_service_invoke(
        &self,
        identity: &str,
        request: Value,
        callback: ServiceReplyCallback,
    ) -> Result<()>;

    /// Invoke a platform service on a specific device
    ///
    /// # Arguments
    /// * `product_key` - Target device's product key
    /// * `device_id` - Target device's ID
    /// * `identity` - Service identity
    /// * `request` - Request data
    /// * `callback` - Callback for handling reply
    async fn platform_service_invoke_for(
        &self,
        product_key: &str,
        device_id: &str,
        identity: &str,
        request: Value,
        callback: ServiceReplyCallback,
    ) -> Result<()>;

    /// Start the client and establish connection
    async fn start(&self) -> Result<()>;

    /// Release resources and disconnect
    async fn release(&self) -> Result<()>;
}
