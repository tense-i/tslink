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

/// Test service callback registration
fn test_service_callback(client: &Arc<DefaultTslinkClient>) {
    // Register a service handler for "reboot" command
    let reboot_handler: ServiceCallback = Arc::new(move |params| {
        info!("Received reboot service call with params: {:?}", params);
        
        // Simulate reboot processing - return response
        json!({
            "status": "accepted",
            "message": "Reboot scheduled",
            "delay_seconds": 5
        })
    });

    info!("Registering service handler for 'reboot'");
    client.set_service_handle("reboot", reboot_handler);
    info!("✓ Service handler registered");

    // Register a service handler for "set_config"
    let config_handler: ServiceCallback = Arc::new(move |params| {
        info!("Config update received: {:?}", params);
        
        json!({
            "status": "success",
            "applied": true
        })
    });

    info!("Registering service handler for 'set_config'");
    client.set_service_handle("set_config", config_handler);
    info!("✓ Service handler registered");
}

/// Test cloud service invocation
async fn test_cloud_service(client: &Arc<DefaultTslinkClient>) -> Result<()> {
    // Invoke a cloud service with reply callback
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

    // Invoke with reply callback
    let reply_callback: ServiceReplyCallback = Arc::new(move |reply| {
        info!("Received service reply: {:?}", reply);
    });

    info!("Invoking platform service 'data_sync': {:?}", service_params);
    client.platform_service_invoke("data_sync", service_params, reply_callback).await?;
    info!("✓ Platform service invoked");

    // Invoke another service
    let config_callback: ServiceReplyCallback = Arc::new(move |reply| {
        info!("Received config reply: {:?}", reply);
    });

    info!("Invoking platform service 'get_config'");
    client.platform_service_invoke(
        "get_config",
        json!({"key": "device_settings"}),
        config_callback,
    ).await?;
    info!("✓ Platform service invoked");

    Ok(())
}

fn test_service_registry(client: &Arc<DefaultTslinkClient>) -> Result<()> {
    let get_device_info: ServiceCallback = Arc::new(|_params| {
        json!({"status": "success", "device": "test_device_001"})
    });
    
    let take_photo: ServiceCallback = Arc::new(|_params| {
        json!({"status": "success", "photo_id": "img_001"})
    });
    
    client.set_service_handle("getDeviceInfo", get_device_info);
    client.set_service_handle("takephoto", take_photo);
    Ok(())
}

// Note: thing_service_invoke is not implemented yet
// Use platform_service_invoke for cloud service invocation