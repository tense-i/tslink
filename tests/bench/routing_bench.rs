/// Routing performance benchmark.
///
/// Validates that 10K topic/s routing throughput target is achievable.
/// Run with: cargo test --release -- routing_bench --nocapture
#[cfg(test)]
mod routing_bench {
    use std::time::Instant;

    use tslink::infrastructure::mqtt::topic_parser::{classify_thing_message, parse_topic};

    #[test]
    fn bench_topic_parsing_10k() {
        let topics = [
            "sys/pk001/did001/thing/event/property/post",
            "sys/pk002/did002/thing/service/reboot/post_reply",
            "sys/pk003/did003/thing/pong/post",
            "sys/pk004/did004/thing/ntp/post",
            "sys/pk005/did005/thing/register/post",
            "sys/pk006/did006/thing/event/temperature/info",
            "sys/pk007/did007/thing/properties/state",
            "sys/pk008/did008/thing/dynamic_register/post",
            "region/cn-east/sys/pk009/did009/thing/event/property/post",
            "sys/pk010/did010_link1/thing/service/restart/post_reply",
        ];

        let iterations = 10_000;
        let start = Instant::now();

        for i in 0..iterations {
            let topic = &topics[i % topics.len()];
            let _ = parse_topic(topic);
        }

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Topic parsing: {} iterations in {:?} ({:.0} ops/sec)",
            iterations, elapsed, ops_per_sec
        );

        // Should parse >100K topics/sec on modern hardware
        assert!(
            ops_per_sec > 10_000.0,
            "Topic parsing too slow: {:.0} ops/sec (target: 10K+)",
            ops_per_sec
        );
    }

    #[test]
    fn bench_parse_and_classify_10k() {
        let topics = [
            "sys/pk001/did001/thing/event/property/post",
            "sys/pk002/did002/thing/service/reboot/post_reply",
            "sys/pk003/did003/thing/pong/post",
            "sys/pk004/did004/thing/ntp/post",
            "sys/pk005/did005/thing/register/post",
        ];

        let iterations = 10_000;
        let start = Instant::now();

        for i in 0..iterations {
            let topic = &topics[i % topics.len()];
            if let Ok(info) = parse_topic(topic) {
                let _ = classify_thing_message(&info);
            }
        }

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Parse+classify: {} iterations in {:?} ({:.0} ops/sec)",
            iterations, elapsed, ops_per_sec
        );

        assert!(
            ops_per_sec > 10_000.0,
            "Parse+classify too slow: {:.0} ops/sec",
            ops_per_sec
        );
    }
}
