//! Service Invoke Test - 通过 tslink HTTP API 调用设备服务
//!
//! 本测试分为两部分：
//! 1. 设备端：注册服务回调，等待云端调用
//! 2. 模拟云端：通过 tslink HTTP API 发送服务调用
//!
//! 运行方式：
//!   1. 启动 tslink 平台 (MQTT + HTTP Server)
//!   2. 终端1 (设备端): cargo run --example service_invoke_test -- device
//!   3. 终端2 (云端):   cargo run --example service_invoke_test -- cloud
//!
//! 环境变量：
//!   MQTT_ENDPOINT: MQTT broker URL (默认: mqtt://localhost:1883)
//!   TSLINK_API_URL: tslink HTTP API URL (默认: http://localhost:3000)

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use tokio::time::sleep;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

use tslink_rsdk::prelude::*;

const PRODUCT_KEY: &str = "test_product";
const DEVICE_ID: &str = "test_device_001";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("device");

    match mode {
        "device" => run_device().await,
        "cloud" => run_cloud_simulator().await,
        _ => {
            println!("Usage: cargo run --example service_invoke_test -- [device|cloud]");
            Ok(())
        }
    }
}

/// 设备端 - 注册服务并等待云端调用
async fn run_device() -> Result<()> {
    info!("=== 设备端启动 ===");

    let endpoint = std::env::var("MQTT_ENDPOINT")
        .unwrap_or_else(|_| "mqtt://localhost:1883".to_string());

    info!("连接 MQTT Broker: {}", endpoint);

    // 构建客户端
    let client = Arc::new(
        TslinkClientBuilder::new()
            .endpoint(&endpoint)
            .product_key(PRODUCT_KEY)
            .device_id(DEVICE_ID)
            .username("device")
            .password("device123")
            .build()?,
    );

    // 注册服务回调
    register_services(&client);

    // 启动客户端
    client.start().await?;
    info!("✓ 设备已连接，等待云端服务调用...");

    // 保持运行，等待服务调用
    info!("按 Ctrl+C 退出");
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

/// 注册设备服务
fn register_services(client: &Arc<DefaultTslinkClient>) {
    // 服务1: getDeviceInfo - 获取设备信息
    let get_device_info: ServiceCallback = Arc::new(|params| {
        info!(">>> 收到 getDeviceInfo 服务调用");
        info!("    参数: {:?}", params);

        let response = json!({
            "code": 200,
            "data": {
                "deviceId": DEVICE_ID,
                "productKey": PRODUCT_KEY,
                "firmwareVersion": "1.0.0",
                "status": "online",
                "uptime": 3600
            }
        });

        info!("<<< 返回响应: {:?}", response);
        response
    });
    client.set_service_handle("getDeviceInfo", get_device_info);
    info!("✓ 已注册服务: getDeviceInfo");

    // 服务2: takePhoto - 拍照
    let take_photo: ServiceCallback = Arc::new(|params| {
        info!(">>> 收到 takePhoto 服务调用");
        info!("    参数: {:?}", params);

        let resolution = params
            .get("resolution")
            .and_then(|v| v.as_str())
            .unwrap_or("1080p");

        let response = json!({
            "code": 200,
            "data": {
                "photoId": format!("photo_{}", chrono::Utc::now().timestamp()),
                "resolution": resolution,
                "size": 1024000,
                "url": "https://example.com/photos/latest.jpg"
            }
        });

        info!("<<< 返回响应: {:?}", response);
        response
    });
    client.set_service_handle("takePhoto", take_photo);
    info!("✓ 已注册服务: takePhoto");

    // 服务3: reboot - 重启设备
    let reboot: ServiceCallback = Arc::new(|params| {
        info!(">>> 收到 reboot 服务调用");
        info!("    参数: {:?}", params);

        let delay = params
            .get("delay")
            .and_then(|v| v.as_i64())
            .unwrap_or(5);

        info!("    设备将在 {} 秒后重启...", delay);

        json!({
            "code": 200,
            "message": format!("Reboot scheduled in {} seconds", delay)
        })
    });
    client.set_service_handle("reboot", reboot);
    info!("✓ 已注册服务: reboot");

    // 服务4: setConfig - 设置配置
    let set_config: ServiceCallback = Arc::new(|params| {
        info!(">>> 收到 setConfig 服务调用");
        info!("    参数: {:?}", params);

        json!({
            "code": 200,
            "message": "Config updated successfully",
            "applied": true
        })
    });
    client.set_service_handle("setConfig", set_config);
    info!("✓ 已注册服务: setConfig");
}

/// 模拟云端 - 通过 tslink HTTP API 发送服务调用
async fn run_cloud_simulator() -> Result<()> {
    info!("=== 云端模拟器启动 (HTTP API 模式) ===");

    let api_url = std::env::var("TSLINK_API_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    info!("tslink API URL: {}", api_url);

    // 创建 HTTP 客户端 (禁用代理，避免 localhost 请求走系统代理)
    let client = HttpClient::builder()
        .timeout(Duration::from_secs(30))
        .no_proxy()  // 禁用代理
        .build()
        .expect("Failed to create HTTP client");

    info!("✓ HTTP 客户端已创建");

    // 发送服务调用测试
    info!("\n=== 开始服务调用测试 ===\n");

    // 测试1: 调用 getDeviceInfo (异步)
    invoke_service_via_http(
        &client,
        &api_url,
        "getDeviceInfo",
        json!({}),
        false,
    ).await;
    sleep(Duration::from_secs(1)).await;

    // 测试2: 调用 takePhoto (异步)
    invoke_service_via_http(
        &client,
        &api_url,
        "takePhoto",
        json!({
            "resolution": "4K",
            "format": "jpeg"
        }),
        false,
    ).await;
    sleep(Duration::from_secs(1)).await;

    // 测试3: 调用 reboot (异步)
    invoke_service_via_http(
        &client,
        &api_url,
        "reboot",
        json!({
            "delay": 10,
            "reason": "firmware update"
        }),
        false,
    ).await;
    sleep(Duration::from_secs(1)).await;

    // 测试4: 调用 setConfig (同步 - 等待设备响应)
    invoke_service_via_http(
        &client,
        &api_url,
        "setConfig",
        json!({
            "logLevel": "debug",
            "reportInterval": 30,
            "enableFeatureX": true
        }),
        true, // sync = true, 等待设备响应
    ).await;

    info!("\n=== 服务调用测试完成 ===");
    sleep(Duration::from_secs(2)).await;

    Ok(())
}

/// 通过 tslink HTTP API 调用设备服务
///
/// API: POST /api/v1/devices/{pk}/{did}/services/{method}
/// Body: { "data": {...}, "sync": bool }
async fn invoke_service_via_http(
    client: &HttpClient,
    api_url: &str,
    service_name: &str,
    data: Value,
    sync: bool,
) {
    // 构造 API URL
    // 格式: POST /api/v1/devices/{pk}/{did}/services/{method}
    let url = format!(
        "{}/api/v1/devices/{}/{}/services/{}",
        api_url, PRODUCT_KEY, DEVICE_ID, service_name
    );

    // 构造请求体
    let request_body = json!({
        "data": data,
        "sync": sync
    });

    info!(">>> 调用服务: {} (sync={})", service_name, sync);
    info!("    URL: POST {}", url);
    info!("    Body: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default());

    match client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            // 先获取文本，再尝试解析 JSON
            match response.text().await {
                Ok(text) => {
                    if text.is_empty() {
                        error!("✗ 响应为空 [{}] - 请确认 tslink 平台已启动", status);
                        return;
                    }
                    match serde_json::from_str::<Value>(&text) {
                        Ok(body) => {
                            if status.is_success() {
                                info!("✓ 响应 [{}]: {}", status, serde_json::to_string_pretty(&body).unwrap_or_default());
                            } else {
                                error!("✗ 失败 [{}]: {}", status, serde_json::to_string_pretty(&body).unwrap_or_default());
                            }
                        }
                        Err(_) => {
                            error!("✗ 响应非 JSON [{}]: {}", status, text);
                        }
                    }
                }
                Err(e) => {
                    error!("✗ 读取响应失败: {:?}", e);
                }
            }
        }
        Err(e) => {
            if e.is_connect() {
                error!("✗ 连接失败 - 请确认 tslink 平台已启动: {:?}", e);
            } else {
                error!("✗ 请求失败: {:?}", e);
            }
        }
    }
}
