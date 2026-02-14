//! IPC Large Frame Benchmark — using RSDK IpcChannel API
//!
//! Tests IPC publish/subscribe performance through the tslink-rsdk IpcChannel layer
//! (iceoryx2 zero-copy shared memory underneath).
//!
//! Measures:
//!   - IPC publish latency (channel.send)
//!   - IPC subscriber callback throughput
//!   - End-to-end pub→sub average latency (timestamp embedded in message)
//!
//! Usage:
//!   cargo run --example bench_ipc_frame --features ipc -- --frames 1000
//!   cargo run --example bench_ipc_frame --features ipc -- --frames 5000 --msg-size 32000
//!   cargo run --example bench_ipc_frame --features ipc -- --frames 500 --msg-size 60000

use std::time::{Duration, Instant};

use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "bench_ipc_frame")]
struct Args {
    /// Message payload size in bytes (default 12MB max via iceoryx2 slice API)
    #[arg(long, default_value_t = 32000)]
    msg_size: usize,

    /// Number of frames to send
    #[arg(long, default_value_t = 1000)]
    frames: u32,

    /// Delay between frames in milliseconds (0 = as fast as possible)
    #[arg(long, default_value_t = 0)]
    delay_ms: u64,

    /// Wait time for subscriber to settle before publishing (ms)
    #[arg(long, default_value_t = 1000)]
    settle_ms: u64,

    /// Max wait time for subscriber to receive all frames (seconds)
    #[arg(long, default_value_t = 30)]
    timeout_secs: u64,

    /// Product key
    #[arg(long, default_value = "ipc_bench")]
    product_key: String,

    /// Device ID
    #[arg(long, default_value = "ipc_device_001")]
    device_id: String,
}

#[cfg(feature = "ipc")]
mod ipc_bench {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use tslink_rsdk::channel::{IpcChannel, IpcConfig, MessageChannel, MessageReceiveCallback};

    const BENCH_TOPIC: &str = "bench/ipc/frame/test";

    fn now_ns() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    fn percentile(sorted: &[u64], pct: u32) -> u64 {
        if sorted.is_empty() {
            return 0;
        }
        let idx = ((pct as f64 / 100.0) * (sorted.len() - 1) as f64) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    fn print_latency_stats(label: &str, latencies_ns: &mut Vec<u64>) {
        if latencies_ns.is_empty() {
            info!("  {} — no samples", label);
            return;
        }
        latencies_ns.sort_unstable();
        let count = latencies_ns.len();
        let avg = latencies_ns.iter().sum::<u64>() / count as u64;
        let min = latencies_ns[0];
        let max = *latencies_ns.last().unwrap();
        let p50 = percentile(latencies_ns, 50);
        let p95 = percentile(latencies_ns, 95);
        let p99 = percentile(latencies_ns, 99);
        info!(
            "  {} (n={}) avg={:.3}ms  min={:.3}ms  P50={:.3}ms  P95={:.3}ms  P99={:.3}ms  max={:.3}ms",
            label,
            count,
            avg as f64 / 1_000_000.0,
            min as f64 / 1_000_000.0,
            p50 as f64 / 1_000_000.0,
            p95 as f64 / 1_000_000.0,
            p99 as f64 / 1_000_000.0,
            max as f64 / 1_000_000.0,
        );
    }

    /// Callback that records receive timestamps and end-to-end latencies
    struct BenchCallback {
        recv_count: AtomicU64,
        e2e_latencies_ns: Mutex<Vec<u64>>,
        recv_latencies_ns: Mutex<Vec<u64>>,
    }

    impl BenchCallback {
        fn new(capacity: usize) -> Self {
            Self {
                recv_count: AtomicU64::new(0),
                e2e_latencies_ns: Mutex::new(Vec::with_capacity(capacity)),
                recv_latencies_ns: Mutex::new(Vec::with_capacity(capacity)),
            }
        }
    }

    impl MessageReceiveCallback for BenchCallback {
        fn receive(&self, _topic: &str, data: &str) {
            let recv_ts = now_ns();
            self.recv_count.fetch_add(1, Ordering::Relaxed);

            // Extract the publish timestamp from the message
            // Message format: "ts_ns:XXXXXXX|padding..."
            if let Some(ts_str) = data.strip_prefix("ts_ns:") {
                if let Some(pipe_pos) = ts_str.find('|') {
                    if let Ok(pub_ts) = ts_str[..pipe_pos].parse::<u64>() {
                        let e2e_ns = recv_ts.saturating_sub(pub_ts);
                        if let Ok(mut lat) = self.e2e_latencies_ns.lock() {
                            lat.push(e2e_ns);
                        }
                    }
                }
            }

            // Record callback processing time
            let cb_elapsed = now_ns().saturating_sub(recv_ts);
            if let Ok(mut lat) = self.recv_latencies_ns.lock() {
                lat.push(cb_elapsed);
            }
        }
    }

    pub async fn run_bench(args: &Args) {
        let msg_size = args.msg_size;
        info!("=== RSDK IpcChannel Benchmark ===");
        info!("  msg_size:   {} bytes ({:.2} KB)", msg_size, msg_size as f64 / 1024.0);
        info!("  frames:     {}", args.frames);

        // Create IPC channel
        let config = IpcConfig::new(&args.product_key, &args.device_id);
        let channel = IpcChannel::new(config).expect("Failed to create IpcChannel");

        // Set up callback
        let callback = Arc::new(BenchCallback::new(args.frames as usize));
        channel.set_callback(callback.clone()).await;

        // Start channel
        channel.start().await.expect("Failed to start IpcChannel");

        // Subscribe to bench topic
        channel.add_topic(BENCH_TOPIC).await.expect("Failed to subscribe");

        // Wait for subscriber to settle
        info!("Waiting {}ms for subscriber to settle...", args.settle_ms);
        tokio::time::sleep(Duration::from_millis(args.settle_ms)).await;

        // Build message template: "ts_ns:XXXXXXXXXXXXXXXX|<padding>"
        // The padding fills the rest to reach msg_size
        let header_template = format!("ts_ns:{}|", "0".repeat(20)); // ~28 bytes header
        let padding_len = msg_size.saturating_sub(header_template.len());
        let padding: String = "X".repeat(padding_len);

        // Publish frames
        info!("Publishing {} frames...", args.frames);
        let mut pub_latencies_ns = Vec::with_capacity(args.frames as usize);
        let mut pub_errors = 0u32;
        let overall_start = Instant::now();

        for _seq in 0..args.frames {
            let ts = now_ns();
            let msg = format!("ts_ns:{}|{}", ts, &padding);

            let pub_start = Instant::now();
            match channel.send(BENCH_TOPIC, &msg).await {
                Ok(()) => {
                    pub_latencies_ns.push(pub_start.elapsed().as_nanos() as u64);
                }
                Err(e) => {
                    pub_errors += 1;
                    if pub_errors <= 3 {
                        info!("  Publish error: {:?}", e);
                    }
                }
            }

            if args.delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(args.delay_ms)).await;
            }
        }

        let pub_elapsed = overall_start.elapsed();
        let pub_ok = pub_latencies_ns.len() as u32;
        info!(
            "Publishing done: {}/{} OK in {:.3}s",
            pub_ok,
            args.frames,
            pub_elapsed.as_secs_f64()
        );

        // Wait for subscriber to catch up
        let deadline = Instant::now() + Duration::from_secs(args.timeout_secs);
        loop {
            let recv = callback.recv_count.load(Ordering::Relaxed);
            if recv >= pub_ok as u64 {
                break;
            }
            if Instant::now() > deadline {
                info!(
                    "  Timeout waiting for subscriber: received {}/{}",
                    recv, pub_ok
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let overall_elapsed = overall_start.elapsed();
        let recv_count = callback.recv_count.load(Ordering::Relaxed);
        let fps_pub = pub_ok as f64 / pub_elapsed.as_secs_f64();
        let bandwidth_mbps =
            (msg_size as f64 * pub_ok as f64) / pub_elapsed.as_secs_f64() / 1_048_576.0;

        // Print results
        info!("\n=== IPC Frame Benchmark Results (RSDK IpcChannel) ===");
        info!("  Message size:   {} bytes", msg_size);
        info!("  Frames sent:    {} (errors: {})", pub_ok, pub_errors);
        info!("  Frames recv:    {}", recv_count);
        info!("  Loss:           {}", pub_ok as u64 - recv_count);
        info!("  Total time:     {:.3}s", overall_elapsed.as_secs_f64());
        info!("  Pub throughput: {:.1} fps", fps_pub);
        info!("  Bandwidth:      {:.2} MB/s", bandwidth_mbps);
        print_latency_stats("Publish (channel.send)", &mut pub_latencies_ns);
        {
            let mut e2e = callback.e2e_latencies_ns.lock().unwrap();
            print_latency_stats("End-to-end (pub→sub callback)", &mut e2e);
        }
        {
            let mut recv = callback.recv_latencies_ns.lock().unwrap();
            print_latency_stats("Callback processing", &mut recv);
        }

        // Stop channel
        channel.stop().await.ok();
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    #[cfg(feature = "ipc")]
    {
        ipc_bench::run_bench(&args).await;
    }

    #[cfg(not(feature = "ipc"))]
    {
        let _ = args;
        eprintln!("This example requires the 'ipc' feature.");
        eprintln!("Run with: cargo run --example bench_ipc_frame --features ipc");
    }
}
