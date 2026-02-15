//! API Validation Benchmark
//!
//! Validates all SPEC-004 new service APIs end-to-end.
//! Requires a running MQTT broker on localhost:1883.
//!
//! Usage:
//!   cargo run --example bench_api_validation

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tokio::time::sleep;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

use tslink_rsdk::prelude::*;

#[derive(Parser, Debug)]
#[command(name = "bench_api_validation")]
struct Args {
    /// MQTT broker endpoint
    #[arg(long, default_value = "mqtt://localhost:1883")]
    endpoint: String,

    /// Timeout for sync calls in seconds
    #[arg(long, default_value_t = 5)]
    timeout: u64,
}

struct TestResult {
    name: String,
    passed: bool,
    detail: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    info!("=== API Validation Benchmark ===");
    info!("  endpoint: {}", args.endpoint);

    let client = Arc::new(
        TslinkClientBuilder::new()
            .endpoint(&args.endpoint)
            .product_key("test_product")
            .device_id("test_device_001")
            .username("device")
            .password("device123")
            .publish_qos(QoS::AtLeastOnce)
            .subscribe_qos(QoS::AtLeastOnce)
            .build()?,
    );

    client.start().await?;
    sleep(Duration::from_secs(2)).await;

    let mut results: Vec<TestResult> = Vec::new();

    // --- Test 1: thing_property_post ---
    {
        let name = "thing_property_post".to_string();
        let data = serde_json::json!({"temperature": 25.5, "humidity": 60});
        match client.thing_property_post(data).await {
            Ok(()) => results.push(TestResult {
                name,
                passed: true,
                detail: "Published successfully".into(),
            }),
            Err(e) => results.push(TestResult {
                name,
                passed: false,
                detail: format!("{:?}", e),
            }),
        }
    }

    // --- Test 2: thing_property_post_for ---
    {
        let name = "thing_property_post_for".to_string();
        let data = serde_json::json!({"voltage": 3.3});
        match client
            .thing_property_post_for("test_product", "test_device_002", data)
            .await
        {
            Ok(()) => results.push(TestResult {
                name,
                passed: true,
                detail: "Published for other device".into(),
            }),
            Err(e) => results.push(TestResult {
                name,
                passed: false,
                detail: format!("{:?}", e),
            }),
        }
    }

    // --- Test 3: thing_event_post (Info) ---
    {
        let name = "thing_event_post (Info)".to_string();
        let data = serde_json::json!({"event": "boot_complete", "uptime": 120});
        match client
            .thing_event_post(EventType::Info, "system_info", data)
            .await
        {
            Ok(()) => results.push(TestResult {
                name,
                passed: true,
                detail: "Event posted".into(),
            }),
            Err(e) => results.push(TestResult {
                name,
                passed: false,
                detail: format!("{:?}", e),
            }),
        }
    }

    // --- Test 4: thing_event_post (Warning) ---
    {
        let name = "thing_event_post (Warning)".to_string();
        let data = serde_json::json!({"warning": "high_temperature", "value": 85});
        match client
            .thing_event_post(EventType::Warning, "temp_warning", data)
            .await
        {
            Ok(()) => results.push(TestResult {
                name,
                passed: true,
                detail: "Warning event posted".into(),
            }),
            Err(e) => results.push(TestResult {
                name,
                passed: false,
                detail: format!("{:?}", e),
            }),
        }
    }

    // --- Test 5: thing_event_post (Error) ---
    {
        let name = "thing_event_post (Error)".to_string();
        let data = serde_json::json!({"error": "sensor_failure", "code": 500});
        match client
            .thing_event_post(EventType::Error, "sensor_error", data)
            .await
        {
            Ok(()) => results.push(TestResult {
                name,
                passed: true,
                detail: "Error event posted".into(),
            }),
            Err(e) => results.push(TestResult {
                name,
                passed: false,
                detail: format!("{:?}", e),
            }),
        }
    }

    // --- Test 6: set_service_specific_executor ---
    {
        let name = "set_service_specific_executor".to_string();
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = call_count.clone();

        let executor: ServiceExecutor = Arc::new(move |req, reply_cb| {
            cc.fetch_add(1, Ordering::Relaxed);
            info!(
                "  [executor] Received service call: {}",
                req.service_identifier
            );
            reply_cb(200, serde_json::to_vec(&serde_json::json!({"status": "ok"})).unwrap());
        });

        client.set_service_specific_executor(
            "getDeviceInfo",
            executor,
            CommunicationChannel::All,
            "test_product",
            "test_device_001",
        );

        results.push(TestResult {
            name,
            passed: true,
            detail: "Executor registered without panic".into(),
        });
    }

    // --- Test 7: set_service_unified_executor ---
    {
        let name = "set_service_unified_executor".to_string();
        let executor: ServiceExecutor = Arc::new(move |req, reply_cb| {
            info!(
                "  [unified] Fallback handler for: {}",
                req.service_identifier
            );
            reply_cb(200, vec![]);
        });

        client.set_service_unified_executor(
            executor,
            CommunicationChannel::All,
            "test_product",
            "test_device_001",
        );

        results.push(TestResult {
            name,
            passed: true,
            detail: "Unified executor registered without panic".into(),
        });
    }

    // --- Test 8: set_property_set_executor ---
    {
        let name = "set_property_set_executor".to_string();
        let executor: ServiceExecutor = Arc::new(move |req, reply_cb| {
            info!(
                "  [property_set] Property set handler: {}",
                req.service_identifier
            );
            reply_cb(200, vec![]);
        });

        client.set_property_set_executor(executor);

        results.push(TestResult {
            name,
            passed: true,
            detail: "Property set executor registered without panic".into(),
        });
    }

    // --- Test 9: platform_service_invoke_async ---
    {
        let name = "platform_service_invoke_async".to_string();
        let request = PlatformServiceRequest::new(
            "cloud_query",
            serde_json::to_vec(&serde_json::json!({"query": "status"})).unwrap_or_default(),
        )
        .with_channel(CommunicationChannel::Remote);

        let callback_called = Arc::new(AtomicU32::new(0));
        let cc = callback_called.clone();
        let callback: PlatformResponseCallback = Arc::new(move |resp| {
            cc.fetch_add(1, Ordering::Relaxed);
            info!("  [async_callback] Platform response result: {}", resp.result);
        });

        match client.platform_service_invoke_async(request, callback).await {
            Ok(()) => results.push(TestResult {
                name,
                passed: true,
                detail: "Async invoke sent (callback may arrive later)".into(),
            }),
            Err(e) => results.push(TestResult {
                name,
                passed: false,
                detail: format!("{:?}", e),
            }),
        }
    }

    // --- Test 10: platform_service_invoke_sync (expect timeout) ---
    {
        let name = "platform_service_invoke_sync (timeout expected)".to_string();
        let request = PlatformServiceRequest::new("cloud_sync_test", vec![])
            .with_channel(CommunicationChannel::Remote);

        match client
            .platform_service_invoke_sync(request, 2000)
            .await
        {
            Ok(resp) => results.push(TestResult {
                name,
                passed: true,
                detail: format!("Got response result: {}", resp.result),
            }),
            Err(e) => {
                let is_timeout = format!("{:?}", e).contains("imeout");
                results.push(TestResult {
                    name,
                    passed: is_timeout,
                    detail: if is_timeout {
                        "Correctly timed out (no cloud responder)".into()
                    } else {
                        format!("Unexpected error: {:?}", e)
                    },
                });
            }
        }
    }

    // --- Test 11: device_service_invoke_async ---
    {
        let name = "device_service_invoke_async".to_string();
        let request = DeviceServiceRequest::new("peer_query", vec![1, 2, 3])
            .with_channel(CommunicationChannel::Remote);

        let callback: ServiceResponseCallback = Arc::new(move |resp| {
            info!("  [device_async_cb] Device response result: {}", resp.result);
        });

        match client
            .device_service_invoke_async(request, "test_product", "test_device_002", callback)
            .await
        {
            Ok(()) => results.push(TestResult {
                name,
                passed: true,
                detail: "Async device invoke sent".into(),
            }),
            Err(e) => results.push(TestResult {
                name,
                passed: false,
                detail: format!("{:?}", e),
            }),
        }
    }

    // --- Test 12: device_service_invoke_sync (expect timeout) ---
    {
        let name = "device_service_invoke_sync (timeout expected)".to_string();
        let request = DeviceServiceRequest::new("peer_sync_test", vec![])
            .with_channel(CommunicationChannel::Remote);

        match client
            .device_service_invoke_sync(
                request,
                "test_product",
                "test_device_002",
                2000,
            )
            .await
        {
            Ok(resp) => results.push(TestResult {
                name,
                passed: true,
                detail: format!("Got response result: {}", resp.result),
            }),
            Err(e) => {
                let is_timeout = format!("{:?}", e).contains("imeout");
                results.push(TestResult {
                    name,
                    passed: is_timeout,
                    detail: if is_timeout {
                        "Correctly timed out (no peer responder)".into()
                    } else {
                        format!("Unexpected error: {:?}", e)
                    },
                });
            }
        }
    }

    // --- Print Report ---
    info!("\n=== API Validation Report ===");
    let mut pass_count = 0;
    let mut fail_count = 0;
    for (i, r) in results.iter().enumerate() {
        let status = if r.passed {
            pass_count += 1;
            "PASS"
        } else {
            fail_count += 1;
            "FAIL"
        };
        info!(
            "  [{:>2}] [{}] {} — {}",
            i + 1,
            status,
            r.name,
            r.detail
        );
    }

    info!("\n  Total: {} tests, {} passed, {} failed", results.len(), pass_count, fail_count);

    if fail_count > 0 {
        error!("  ❌ Some tests FAILED");
    } else {
        info!("  ✅ All tests PASSED");
    }

    client.release().await?;
    info!("=== Done ===");
    Ok(())
}
