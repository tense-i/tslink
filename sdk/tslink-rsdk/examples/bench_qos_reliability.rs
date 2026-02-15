//! QoS Reliability Benchmark
//!
//! Tests message delivery reliability under QoS=0 vs QoS=1.
//! Uses raw rumqttc for both publisher and subscriber for clean measurement.
//!
//! Usage:
//!   cargo run --example bench_qos_reliability -- --count 1000 --qos 0
//!   cargo run --example bench_qos_reliability -- --count 1000 --qos 1

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::time::sleep;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "bench_qos_reliability")]
struct Args {
    /// Number of messages to send
    #[arg(long, default_value_t = 1000)]
    count: u64,

    /// QoS level (0 or 1)
    #[arg(long, default_value_t = 0)]
    qos: u8,

    /// Delay between messages in milliseconds (0 = no delay)
    #[arg(long, default_value_t = 1)]
    delay_ms: u64,

    /// Wait time after sending for late arrivals (seconds)
    #[arg(long, default_value_t = 5)]
    wait_after: u64,

    /// MQTT broker host
    #[arg(long, default_value = "localhost")]
    host: String,

    /// MQTT broker port
    #[arg(long, default_value_t = 1883)]
    port: u16,
}

const TEST_TOPIC: &str = "bench/qos/reliability";

fn parse_qos(q: u8) -> QoS {
    match q {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtMostOnce,
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    let qos = parse_qos(args.qos);

    info!("=== QoS Reliability Benchmark ===");
    info!("  count:    {}", args.count);
    info!("  qos:      {}", args.qos);
    info!("  delay_ms: {}", args.delay_ms);
    info!("  broker:   {}:{}", args.host, args.port);

    // --- Counters ---
    let received_count = Arc::new(AtomicU64::new(0));
    let out_of_order = Arc::new(AtomicU64::new(0));
    let duplicates = Arc::new(AtomicU64::new(0));

    let sub_received = received_count.clone();
    let sub_ooo = out_of_order.clone();
    let sub_dup = duplicates.clone();

    // --- Subscriber ---
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();
    let host_sub = args.host.clone();
    let port_sub = args.port;

    let sub_handle = tokio::spawn(async move {
        let mut opts = MqttOptions::new("bench_qos_sub", host_sub, port_sub);
        opts.set_keep_alive(Duration::from_secs(30));
        opts.set_clean_session(true);

        let (client, mut eventloop) = AsyncClient::new(opts, 10000);
        client.subscribe(TEST_TOPIC, qos).await.expect("sub failed");

        let mut ready_tx = Some(ready_tx);
        let mut seen = std::collections::HashSet::new();
        let mut last_seq: i64 = -1;

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::SubAck(_))) => {
                    info!("Subscriber: subscribed to {}", TEST_TOPIC);
                    if let Some(tx) = ready_tx.take() {
                        let _ = tx.send(());
                    }
                }
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&publish.payload) {
                        if let Some(seq) = v.get("seq").and_then(|s| s.as_u64()) {
                            if seen.contains(&seq) {
                                sub_dup.fetch_add(1, Ordering::Relaxed);
                            } else {
                                seen.insert(seq);
                                sub_received.fetch_add(1, Ordering::Relaxed);
                                if (seq as i64) < last_seq {
                                    sub_ooo.fetch_add(1, Ordering::Relaxed);
                                }
                                last_seq = seq as i64;
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("Subscriber error: {:?}", e);
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    // Wait for subscriber ready
    info!("Waiting for subscriber...");
    let _ = tokio::time::timeout(Duration::from_secs(10), ready_rx).await;
    sleep(Duration::from_millis(500)).await;

    // --- Publisher ---
    let mut pub_opts = MqttOptions::new("bench_qos_pub", &args.host, args.port);
    pub_opts.set_keep_alive(Duration::from_secs(30));
    pub_opts.set_clean_session(true);

    let (pub_client, mut pub_eventloop) = AsyncClient::new(pub_opts, 10000);

    // Spawn publisher event loop
    tokio::spawn(async move {
        loop {
            match pub_eventloop.poll().await {
                Ok(_) => {}
                Err(e) => {
                    warn!("Publisher eventloop error: {:?}", e);
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    // Wait for publisher to connect
    sleep(Duration::from_secs(1)).await;

    info!("Starting to send {} messages...", args.count);
    let start = Instant::now();
    let mut send_ok: u64 = 0;
    let mut send_err: u64 = 0;

    for seq in 0..args.count {
        let payload = serde_json::json!({ "seq": seq }).to_string();

        match pub_client
            .publish(TEST_TOPIC, qos, false, payload.as_bytes())
            .await
        {
            Ok(()) => send_ok += 1,
            Err(e) => {
                send_err += 1;
                if send_err <= 5 {
                    warn!("Send error #{}: {:?}", send_err, e);
                }
            }
        }

        if args.delay_ms > 0 {
            sleep(Duration::from_millis(args.delay_ms)).await;
        }
    }

    let send_elapsed = start.elapsed();
    info!(
        "Sending complete: {} ok, {} err in {:.2}s",
        send_ok, send_err, send_elapsed.as_secs_f64()
    );

    // Wait for late arrivals
    info!("Waiting {}s for late arrivals...", args.wait_after);
    sleep(Duration::from_secs(args.wait_after)).await;

    // --- Results ---
    let recv = received_count.load(Ordering::Relaxed);
    let ooo = out_of_order.load(Ordering::Relaxed);
    let dups = duplicates.load(Ordering::Relaxed);
    let lost = if send_ok > recv { send_ok - recv } else { 0 };
    let loss_rate = lost as f64 / send_ok.max(1) as f64 * 100.0;

    info!("\n=== QoS Reliability Results (QoS={}) ===", args.qos);
    info!("  Sent (ok):       {}", send_ok);
    info!("  Sent (err):      {}", send_err);
    info!("  Received:        {}", recv);
    info!("  Lost:            {}", lost);
    info!("  Loss rate:       {:.2}%", loss_rate);
    info!("  Out-of-order:    {}", ooo);
    info!("  Duplicates:      {}", dups);
    info!(
        "  Send throughput: {:.1} msg/s",
        send_ok as f64 / send_elapsed.as_secs_f64()
    );

    if args.qos == 0 && loss_rate > 0.0 {
        info!("  NOTE: QoS=0 does not guarantee delivery. Some loss is expected under load.");
    }
    if args.qos >= 1 && loss_rate > 0.0 {
        warn!("  WARNING: QoS>=1 should guarantee delivery. Loss indicates a problem!");
    }

    sub_handle.abort();
    info!("=== Done ===");
}
