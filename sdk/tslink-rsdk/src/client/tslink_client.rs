//! TslinkClient trait definition

use async_trait::async_trait;
use serde_json::Value;

use crate::enums::{CommunicationChannel, EventType};
use crate::error::Result;
use crate::message::{
    DeviceServiceRequest, DeviceServiceResponse, PlatformResponseCallback,
    PlatformServiceRequest, PlatformServiceResponse, ServiceExecutor,
    ServiceResponseCallback,
};

/// Tslink client trait for IoT device communication
///
/// This trait provides methods for:
/// - Property reporting
/// - Event reporting
/// - Service executor registration (unified / specific)
/// - Platform & device service invocation (sync / async)
#[async_trait]
pub trait TslinkClient: Send + Sync {
    // ==================== Property ====================

    /// Report device properties to cloud
    async fn thing_property_post(&self, data: Value) -> Result<()>;

    /// Report device properties for a specific device (proxy mode)
    async fn thing_property_post_for(
        &self,
        product_key: &str,
        device_id: &str,
        data: Value,
    ) -> Result<()>;

    // ==================== Event ====================

    /// Report device event to cloud
    async fn thing_event_post(
        &self,
        event_type: EventType,
        event_name: &str,
        data: Value,
    ) -> Result<()>;

    // ==================== Platform Push Executor ====================

    /// Register a unified executor for all platform push service invocations
    fn set_platform_push_unified_executor(
        &self,
        executor: ServiceExecutor,
        product_key: &str,
        device_id: &str,
    );

    /// Register a specific executor for a named platform push service
    fn set_platform_push_specific_executor(
        &self,
        identifier: &str,
        executor: ServiceExecutor,
        product_key: &str,
        device_id: &str,
    );

    // ==================== Platform Service Invoke ====================

    /// Invoke a platform service synchronously (with timeout)
    async fn platform_service_invoke_sync(
        &self,
        request: PlatformServiceRequest,
        timeout_ms: i32,
    ) -> Result<PlatformServiceResponse>;

    /// Invoke a platform service asynchronously (with callback)
    async fn platform_service_invoke_async(
        &self,
        request: PlatformServiceRequest,
        callback: PlatformResponseCallback,
    ) -> Result<()>;

    // ==================== Device Service Executor ====================

    /// Register a unified executor for all device service invocations
    fn set_service_unified_executor(
        &self,
        executor: ServiceExecutor,
        channel: CommunicationChannel,
        product_key: &str,
        device_id: &str,
    );

    /// Register a specific executor for a named device service
    fn set_service_specific_executor(
        &self,
        identifier: &str,
        executor: ServiceExecutor,
        channel: CommunicationChannel,
        product_key: &str,
        device_id: &str,
    );

    // ==================== Device Service Invoke ====================

    /// Invoke a device service synchronously (with timeout)
    async fn device_service_invoke_sync(
        &self,
        request: DeviceServiceRequest,
        product_key: &str,
        device_id: &str,
        timeout_ms: i32,
    ) -> Result<DeviceServiceResponse>;

    /// Invoke a device service asynchronously (with callback)
    async fn device_service_invoke_async(
        &self,
        request: DeviceServiceRequest,
        product_key: &str,
        device_id: &str,
        callback: ServiceResponseCallback,
    ) -> Result<()>;

    // ==================== Property Set Handler ====================

    /// Register a handler for property set commands
    fn set_property_set_executor(&self, executor: ServiceExecutor);

    // ==================== Lifecycle ====================

    /// Start the client and establish connection
    async fn start(&self) -> Result<()>;

    /// Release resources and disconnect
    async fn release(&self) -> Result<()>;

    // ==================== Multi-Channel ====================

    /// Report device properties using specified channel
    async fn thing_property_post_with_channel(
        &self,
        data: Value,
        channel: CommunicationChannel,
    ) -> Result<()> {
        let _ = channel;
        self.thing_property_post(data).await
    }

    /// Report device event using specified channel
    async fn thing_event_post_with_channel(
        &self,
        event_type: EventType,
        event_name: &str,
        data: Value,
        channel: CommunicationChannel,
    ) -> Result<()> {
        let _ = channel;
        self.thing_event_post(event_type, event_name, data).await
    }

    /// Get the current communication channel configuration
    fn get_channel(&self) -> CommunicationChannel {
        CommunicationChannel::default()
    }
}
