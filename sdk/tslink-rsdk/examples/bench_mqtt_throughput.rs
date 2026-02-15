//! MQTT Throughput Benchmark
//!
//! Measures sustained property reporting throughput and latency.
//!
//! Usage:
//!   cargo run --example bench_mqtt_throughput -- --duration 30 --msg-size 256 --rate 1000 --qos 0

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use tslink_rsdk::prelude::*;

#[derive(Parser, Debug)]
#[command(name = "bench_mqtt_throughput")]
struct Args {
    /// Message payload size in bytes
    #[arg(long, default_value_t = 256)]
    msg_size: usize,

    /// Target send rate (messages per second, 0 = unlimited)
    #[arg(long, default_value_t = 0)]
    rate: u64,

    /// Test duration in seconds
    #[arg(long, default_value_t = 30)]
    duration: u64,

    /// QoS level (0 or 1)
    #[arg(long, default_value_t = 0)]
    qos: u8,

    /// Number of concurrent senders
    #[arg(long, default_value_t = 1)]
    concurrency: u32,

    /// MQTT broker endpoint
    #[arg(long, default_value = "mqtt://localhost:1883")]
    endpoint: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    info!("=== MQTT Throughput Benchmark ===");
    info!("  msg_size:    {} bytes", args.msg_size);
    info!("  rate:        {} msg/s (0=unlimited)", args.rate);
    info!("  duration:    {} s", args.duration);
    info!("  qos:         {}", args.qos);
    info!("  concurrency: {}", args.concurrency);
    info!("  endpoint:    {}", args.endpoint);

    let qos = match args.qos {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => {
            eprintln!("Invalid QoS: {}", args.qos);
            return Ok(());
        }
    };

    // Build client
    let client = Arc::new(
        TslinkClientBuilder::new()
            .endpoint(&args.endpoint)
            .product_key("bench_product")
            .device_id("bench_device_001")
            .username("device")
            .password("device123")
            .publish_qos(qos)
            .subscribe_qos(qos)
            .build()?,
    );

    client.start().await?;
    sleep(Duration::from_secs(2)).await;
    info!("Client connected, starting benchmark...\n");

    // Counters
    let sent_ok = Arc::new(AtomicU64::new(0));
    let sent_err = Arc::new(AtomicU64::new(0));
    let latencies_ns = Arc::new(parking_lot::Mutex::new(Vec::<u64>::with_capacity(
        (args.duration * 10000) as usize,
    )));

    // Generate payload
    let payload_str = "x".repeat(args.msg_size);
    let payload = serde_json::json!({ "data": payload_str });

    let deadline = Instant::now() + Duration::from_secs(args.duration);
    let interval_ns = if args.rate > 0 {
        1_000_000_000u64 / args.rate
    } else {
        0
    };

    // Spawn senders
    let mut handles = Vec::new();
    for _ in 0..args.concurrency {
        let client = client.clone();
        let payload = payload.clone();
        let sent_ok = sent_ok.clone();
        let sent_err = sent_err.clone();
        let latencies_ns = latencies_ns.clone();

        handles.push(tokio::spawn(async move {
            let mut next_send = Instant::now();
            while Instant::now() < deadline {
                if interval_ns > 0 {
                    let now = Instant::now();
                    if now < next_send {
                        sleep(next_send - now).await;
                    }
                    next_send += Duration::from_nanos(interval_ns);
                }

                let start = Instant::now();
                match client.thing_property_post(payload.clone()).await {
                    Ok(()) => {
                        let elapsed = start.elapsed().as_nanos() as u64;
                        sent_ok.fetch_add(1, Ordering::Relaxed);
                        latencies_ns.lock().push(elapsed);
                    }
                    Err(_e) => {
                        sent_err.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    // Wait for all senders
    for h in handles {
        let _ = h.await;
    }

    // Compute stats
    let ok = sent_ok.load(Ordering::Relaxed);
    let err = sent_err.load(Ordering::Relaxed);
    let total = ok + err;
    let duration_secs = args.duration as f64;
    let throughput_msg = ok as f64 / duration_secs;
    let throughput_mb = (ok as f64 * args.msg_size as f64) / (1024.0 * 1024.0 * duration_secs);

    let mut lats = latencies_ns.lock().clone();
    lats.sort_unstable();

    let p50 = percentile(&lats, 50);
    let p95 = percentile(&lats, 95);
    let p99 = percentile(&lats, 99);
    let avg = if lats.is_empty() {
        0
    } else {
        lats.iter().sum::<u64>() / lats.len() as u64
    };

    info!("=== Benchmark Results ===");
    info!("  Duration:      {:.1} s", duration_secs);
    info!("  Total sent:    {}", total);
    info!("  Success:       {}", ok);
    info!("  Errors:        {}", err);
    info!("  Throughput:    {:.1} msg/s", throughput_msg);
    info!("  Throughput:    {:.3} MB/s", throughput_mb);
    info!("  Latency avg:  {:.3} ms", avg as f64 / 1_000_000.0);
    info!("  Latency P50:  {:.3} ms", p50 as f64 / 1_000_000.0);
    info!("  Latency P95:  {:.3} ms", p95 as f64 / 1_000_000.0);
    info!("  Latency P99:  {:.3} ms", p99 as f64 / 1_000_000.0);
    info!("  Error rate:    {:.2}%", err as f64 / total.max(1) as f64 * 100.0);

    client.release().await?;
    info!("=== Done ===");
    Ok(())
}

fn percentile(sorted: &[u64], pct: u32) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((pct as f64 / 100.0) * (sorted.len() - 1) as f64) as usize;
    sorted[idx.min(sorted.len() - 1)]
}
