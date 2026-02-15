//! MQTT Demo - Integration test for tslink-rsdk
//!
//! This demo tests all MQTT channel functionalities:
//! - Device property reporting (属性上报)
//! - Device event reporting (事件上报)
//! - Cloud service invocation (云端服务调用)
//! - Service callback handling (服务回调处理)
//!
//! Run with: cargo run --example mqtt_demo

use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use tslink_rsdk::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("=== tslink-rsdk MQTT Demo ===");

    // Configuration - adjust these values for your environment
    let endpoint = std::env::var("MQTT_ENDPOINT")
        .unwrap_or_else(|_| "mqtt://localhost:1883".to_string());
    let product_key = std::env::var("PRODUCT_KEY")
        .unwrap_or_else(|_| "test_product".to_string());
    let device_id = std::env::var("DEVICE_ID")
        .unwrap_or_else(|_| "test_device_001".to_string());
    let username = std::env::var("MQTT_USERNAME")
        .unwrap_or_else(|_| "device".to_string());
    let password = std::env::var("MQTT_PASSWORD")
        .unwrap_or_else(|_| "device123".to_string());

    info!("Connecting to MQTT broker: {}", endpoint);
    info!("Product: {}, Device: {}", product_key, device_id);

    // Build the TslinkClient
    let client = Arc::new(TslinkClientBuilder::new()
        .endpoint(&endpoint)
        .product_key(&product_key)
        .device_id(&device_id)
        .username(&username)
        .password(&password)
        .build()?);

    // Start the client
    info!("Starting TslinkClient...");
    client.start().await?;

    // Wait for connection
    sleep(Duration::from_secs(2)).await;

    // Test 1: Property Reporting (属性上报)
    info!("\n=== Test 1: Property Reporting ===");
    test_property_reporting(&client).await?;

    // Test 2: Event Reporting (事件上报)
    info!("\n=== Test 2: Event Reporting ===");
    test_event_reporting(&client).await?;

    // Test 3: Service Callback Registration (服务回调注册)
    info!("\n=== Test 3: Service Callback Registration ===");
    test_service_callback(&client);

    // Test 4: Cloud Service Invocation (云端服务调用)
    info!("\n=== Test 4: Cloud Service Invocation ===");
    test_cloud_service(&client).await?;


    // Keep running to receive messages
    info!("\n=== Waiting for incoming messages (10 seconds) ===");
    sleep(Duration::from_secs(10)).await;

    // Release the client
    info!("Releasing TslinkClient...");
    client.release().await?;

    info!("\n=== Demo completed successfully! ===");
    Ok(())
}

/// Test property reporting functionality
async fn test_property_reporting(client: &Arc<DefaultTslinkClient>) -> Result<()> {
    // Report single property
    let properties = json!({
        "temperature": 25.5,
        "humidity": 60,
        "status": "online"
    });

    info!("Reporting properties: {:?}", properties);
    client.thing_property_post(properties).await?;
    info!("✓ Properties reported successfully");

    // Report batch properties
    let batch_properties = json!({
        "cpu_usage": 45.2,
        "memory_usage": 1024,
        "disk_free": 50000,
        "network_status": "connected"
    });

    info!("Reporting batch properties: {:?}", batch_properties);
    client.thing_property_post(batch_properties).await?;
    info!("✓ Batch properties reported successfully");

    Ok(())
}

/// Test event reporting functionality
async fn test_event_reporting(client: &Arc<DefaultTslinkClient>) -> Result<()> {
    // Report info event
    let info_event = json!({
        "message": "Device started successfully",
        "version": "1.0.0"
    });

    info!("Reporting INFO event: {:?}", info_event);
    client.thing_event_post(EventType::Info, "device_started", info_event).await?;
    info!("✓ INFO event reported successfully");

    // Report warning event
    let warning_event = json!({
        "warning": "High temperature detected",
        "temperature": 85.5,
        "threshold": 80.0
    });

    info!("Reporting WARNING event: {:?}", warning_event);
    client.thing_event_post(EventType::Warning, "high_temperature", warning_event).await?;
    info!("✓ WARNING event reported successfully");

    // Report error event
    let error_event = json!({
        "error": "Sensor disconnected",
        "sensor_id": "temp_001",
        "retry_count": 3
    });

    info!("Reporting ERROR event: {:?}", error_event);
    client.thing_event_post(EventType::Error, "sensor_error", error_event).await?;
    info!("✓ ERROR event reported successfully");

    Ok(())
}

/// Test service callback registration (using new ServiceExecutor API)
fn test_service_callback(client: &Arc<DefaultTslinkClient>) {
    // Register a specific executor for "reboot" command
    let reboot_executor: ServiceExecutor = Arc::new(move |req, reply_cb| {
        info!("Received reboot service call: {:?}", req.service_identifier);
        info!("    param_data: {:?}", String::from_utf8_lossy(&req.param_data));

        // Reply with success
        let response = json!({
            "status": "accepted",
            "message": "Reboot scheduled",
            "delay_seconds": 5
        });
        reply_cb(0, serde_json::to_vec(&response).unwrap_or_default());
    });

    info!("Registering service executor for 'reboot'");
    client.set_service_specific_executor(
        "reboot",
        reboot_executor,
        CommunicationChannel::All,
        "test_product",
        "test_device_001",
    );
    info!("Service executor registered");

    // Register a specific executor for "set_config"
    let config_executor: ServiceExecutor = Arc::new(move |req, reply_cb| {
        info!("Config update received: {:?}", String::from_utf8_lossy(&req.param_data));
        let response = json!({"status": "success", "applied": true});
        reply_cb(0, serde_json::to_vec(&response).unwrap_or_default());
    });

    info!("Registering service executor for 'set_config'");
    client.set_service_specific_executor(
        "set_config",
        config_executor,
        CommunicationChannel::All,
        "test_product",
        "test_device_001",
    );
    info!("Service executor registered");
}

/// Test cloud service invocation (using new PlatformServiceRequest API)
async fn test_cloud_service(client: &Arc<DefaultTslinkClient>) -> Result<()> {
    let service_params = json!({
        "target": "cloud_function_1",
        "data": {
            "action": "sync",
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        }
    });

    // Invoke platform service asynchronously with callback
    let callback: PlatformResponseCallback = Arc::new(move |resp| {
        info!("Received platform service response: result={}, data={:?}",
            resp.result, String::from_utf8_lossy(&resp.param_data));
    });

    let request = PlatformServiceRequest::new(
        "data_sync",
        serde_json::to_vec(&service_params).unwrap_or_default(),
    );

    info!("Invoking platform service 'data_sync': {:?}", service_params);
    client.platform_service_invoke_async(request, callback).await?;
    info!("Platform service invoked (async)");

    // Invoke another service
    let config_callback: PlatformResponseCallback = Arc::new(move |resp| {
        info!("Received config reply: result={}, data={:?}",
            resp.result, String::from_utf8_lossy(&resp.param_data));
    });

    let config_request = PlatformServiceRequest::new(
        "get_config",
        serde_json::to_vec(&json!({"key": "device_settings"})).unwrap_or_default(),
    );

    info!("Invoking platform service 'get_config'");
    client.platform_service_invoke_async(config_request, config_callback).await?;
    info!("Platform service invoked (async)");

    Ok(())
}