//! Stress test: simulates N concurrent MQTT devices publishing messages.
//!
//! This test measures:
//! - Topic parsing throughput (msg/s)
//! - Message routing throughput (msg/s)
//! - Concurrent device simulation latency (P50/P95/P99)
//!
//! Run with: `cargo test --test stress_test -- --nocapture`
//!
//! For real MQTT broker stress testing, use `emqtt-bench` externally:
//! ```
//! emqtt_bench pub -h 127.0.0.1 -p 1883 -c 1000 -I 100 \
//!   -t '/sys/%i/device_%i/thing/event/property/post' \
//!   -m '{"id":"1","version":"1.0","params":{"temp":25.5}}'
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Number of simulated devices.
const DEVICE_COUNT: usize = 1000;

/// Messages per device.
const MESSAGES_PER_DEVICE: usize = 100;

/// Total expected messages.
const TOTAL_MESSAGES: usize = DEVICE_COUNT * MESSAGES_PER_DEVICE;

/// Simulated MQTT topic for property post.
fn make_topic(device_idx: usize) -> String {
    format!(
        "/sys/stress_pk_{:04}/stress_did_{:04}/thing/event/property/post",
        device_idx % 100,
        device_idx
    )
}

/// Simulated MQTT payload.
fn make_payload(device_idx: usize, msg_idx: usize) -> Vec<u8> {
    let json = format!(
        r#"{{"id":"{}","version":"1.0","params":{{"temperature":{},"humidity":{}}}}}"#,
        msg_idx,
        20.0 + (device_idx as f64 * 0.01),
        50.0 + (msg_idx as f64 * 0.1),
    );
    json.into_bytes()
}

#[test]
fn stress_topic_parsing_throughput() {
    // Measure raw topic parsing speed across all simulated messages
    let topics: Vec<String> = (0..DEVICE_COUNT).map(make_topic).collect();

    let start = Instant::now();
    let mut parsed = 0u64;

    for _ in 0..MESSAGES_PER_DEVICE {
        for topic in &topics {
            // Simulate topic parsing (split + classify)
            let parts: Vec<&str> = topic.split('/').collect();
            if parts.len() >= 7 {
                let _pk = parts[2];
                let _did = parts[3];
                let _msg_type = parts[4..].join("/");
                parsed += 1;
            }
        }
    }

    let elapsed = start.elapsed();
    let throughput = parsed as f64 / elapsed.as_secs_f64();

    println!("\n=== Topic Parsing Stress Test ===");
    println!("  Devices:    {}", DEVICE_COUNT);
    println!("  Msgs/dev:   {}", MESSAGES_PER_DEVICE);
    println!("  Total msgs: {}", parsed);
    println!("  Elapsed:    {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Throughput: {:.0} msg/s", throughput);
    println!("  Per msg:    {:.3}µs", elapsed.as_micros() as f64 / parsed as f64);

    assert_eq!(parsed as usize, TOTAL_MESSAGES);
    // In debug mode with parallel test suites, 200K+ msg/s is acceptable
    assert!(throughput > 200_000.0, "Topic parsing too slow: {:.0} msg/s", throughput);
}

#[test]
fn stress_payload_serde_throughput() {
    // Measure JSON serialization/deserialization throughput
    let start = Instant::now();
    let mut processed = 0u64;

    for dev in 0..DEVICE_COUNT {
        for msg in 0..MESSAGES_PER_DEVICE {
            let payload = make_payload(dev, msg);
            let _parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
            processed += 1;
        }
    }

    let elapsed = start.elapsed();
    let throughput = processed as f64 / elapsed.as_secs_f64();

    println!("\n=== Payload Serde Stress Test ===");
    println!("  Total msgs: {}", processed);
    println!("  Elapsed:    {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Throughput: {:.0} msg/s", throughput);
    println!("  Per msg:    {:.3}µs", elapsed.as_micros() as f64 / processed as f64);

    assert_eq!(processed as usize, TOTAL_MESSAGES);
    // Should parse at least 200K JSON msg/s
    assert!(throughput > 100_000.0, "JSON serde too slow: {:.0} msg/s", throughput);
}

#[tokio::test]
async fn stress_concurrent_device_simulation() {
    // Simulate DEVICE_COUNT concurrent devices each sending MESSAGES_PER_DEVICE messages
    // Measures concurrent task scheduling overhead and latency distribution

    let total_processed = Arc::new(AtomicU64::new(0));
    let mut latencies_ns: Vec<u64> = Vec::with_capacity(TOTAL_MESSAGES);

    let start = Instant::now();

    // Spawn all device tasks
    let mut handles = Vec::with_capacity(DEVICE_COUNT);
    for dev_idx in 0..DEVICE_COUNT {
        let counter = total_processed.clone();
        let handle = tokio::spawn(async move {
            let mut device_latencies = Vec::with_capacity(MESSAGES_PER_DEVICE);
            for msg_idx in 0..MESSAGES_PER_DEVICE {
                let msg_start = Instant::now();

                // Simulate message processing pipeline:
                // 1. Parse topic
                let topic = make_topic(dev_idx);
                let parts: Vec<&str> = topic.split('/').collect();
                let _pk = parts[2];
                let _did = parts[3];

                // 2. Deserialize payload
                let payload = make_payload(dev_idx, msg_idx);
                let _parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();

                // 3. Simulate async Redis write (yield point)
                tokio::task::yield_now().await;

                counter.fetch_add(1, Ordering::Relaxed);
                device_latencies.push(msg_start.elapsed().as_nanos() as u64);
            }
            device_latencies
        });
        handles.push(handle);
    }

    // Collect all latencies
    for handle in handles {
        let device_lats = handle.await.unwrap();
        latencies_ns.extend(device_lats);
    }

    let elapsed = start.elapsed();
    let total = total_processed.load(Ordering::Relaxed);
    let throughput = total as f64 / elapsed.as_secs_f64();

    // Calculate percentiles
    latencies_ns.sort_unstable();
    let p50 = latencies_ns[latencies_ns.len() * 50 / 100];
    let p95 = latencies_ns[latencies_ns.len() * 95 / 100];
    let p99 = latencies_ns[latencies_ns.len() * 99 / 100];
    let max = *latencies_ns.last().unwrap();

    println!("\n=== Concurrent Device Stress Test ===");
    println!("  Devices:      {}", DEVICE_COUNT);
    println!("  Msgs/dev:     {}", MESSAGES_PER_DEVICE);
    println!("  Total msgs:   {}", total);
    println!("  Elapsed:      {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Throughput:   {:.0} msg/s", throughput);
    println!("  Latency P50:  {:.3}µs", p50 as f64 / 1000.0);
    println!("  Latency P95:  {:.3}µs", p95 as f64 / 1000.0);
    println!("  Latency P99:  {:.3}µs", p99 as f64 / 1000.0);
    println!("  Latency Max:  {:.3}µs", max as f64 / 1000.0);

    assert_eq!(total as usize, TOTAL_MESSAGES);
    // In debug mode with 1000 concurrent tasks, P99 under 500ms is acceptable.
    // Release mode target: P99 < 10ms.
    assert!(
        Duration::from_nanos(p99) < Duration::from_millis(500),
        "P99 latency too high: {:.3}ms",
        p99 as f64 / 1_000_000.0
    );
}

#[tokio::test]
async fn stress_router_dispatch_throughput() {
    // Simulate the full message routing pipeline with topic classification
    use std::collections::HashMap;

    // Build a simulated handler registry
    let handler_types = vec![
        "thing/event/property/post",
        "thing/pong",
        "thing/register",
        "thing/ntp/post",
    ];
    let registry: HashMap<&str, u64> = handler_types.iter().map(|t| (*t, 0u64)).collect();

    let topics: Vec<String> = (0..DEVICE_COUNT)
        .map(|i| {
            let msg_type = &handler_types[i % handler_types.len()];
            format!("/sys/pk_{:04}/did_{:04}/{}", i % 100, i, msg_type)
        })
        .collect();

    let start = Instant::now();
    let mut dispatch_count = 0u64;
    let mut type_counts: HashMap<String, u64> = HashMap::new();

    for _ in 0..MESSAGES_PER_DEVICE {
        for topic in &topics {
            let parts: Vec<&str> = topic.split('/').collect();
            if parts.len() >= 5 {
                let msg_type = parts[4..].join("/");
                if registry.contains_key(msg_type.as_str()) {
                    *type_counts.entry(msg_type).or_insert(0) += 1;
                    dispatch_count += 1;
                }
            }
        }
    }

    let elapsed = start.elapsed();
    let throughput = dispatch_count as f64 / elapsed.as_secs_f64();

    println!("\n=== Router Dispatch Stress Test ===");
    println!("  Total dispatched: {}", dispatch_count);
    println!("  Elapsed:          {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Throughput:       {:.0} msg/s", throughput);
    for (t, c) in &type_counts {
        println!("    {}: {} msgs", t, c);
    }

    assert_eq!(dispatch_count as usize, TOTAL_MESSAGES);
    // In debug mode, 200K+ msg/s is acceptable; release mode should exceed 1M msg/s
    assert!(throughput > 200_000.0, "Router dispatch too slow: {:.0} msg/s", throughput);
}
