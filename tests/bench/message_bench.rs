/// Message serialization/deserialization benchmark.
///
/// Validates that 50K msg/s deserialization target is achievable.
/// Run with: cargo test --release -- message_bench --nocapture
#[cfg(test)]
mod message_bench {
    use std::time::Instant;

    use tslink::domain::message::{CommonTopicReceiver, CommonTopicResponse};

    #[test]
    fn bench_deserialize_50k() {
        let json_payloads = [
            r#"{"tid":"abc-001","version":"1.0","timestamp":1700000000000,"method":"thing.event.property.post","productKey":"pk001","deviceId":"did001","data":{"temperature":25.5,"humidity":60}}"#,
            r#"{"tid":"abc-002","version":"1.0","data":{"status":"ok"},"code":"200"}"#,
            r#"{"tid":"abc-003","bid":"biz-001","version":"1.0","timestamp":1700000001000,"method":"thing.service.reboot","productKey":"pk002","deviceId":"did002","data":{}}"#,
            r#"{"data":{"result":0,"info":{"message":"success"}},"tid":"abc-004","version":"1.0","code":"200","message":"success"}"#,
            r#"{"tid":"abc-005","version":"1.0","timestamp":1700000002000,"data":{"deviceName":"sensor-01","productKey":"pk003"}}"#,
        ];

        let iterations = 50_000;
        let start = Instant::now();

        for i in 0..iterations {
            let payload = json_payloads[i % json_payloads.len()].as_bytes();
            let _: CommonTopicReceiver<serde_json::Value> =
                serde_json::from_slice(payload).unwrap();
        }

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Deserialization: {} iterations in {:?} ({:.0} ops/sec)",
            iterations, elapsed, ops_per_sec
        );

        assert!(
            ops_per_sec > 50_000.0,
            "Deserialization too slow: {:.0} ops/sec (target: 50K+)",
            ops_per_sec
        );
    }

    #[test]
    fn bench_serialize_50k() {
        let response = CommonTopicResponse {
            tid: Some("tid-bench".into()),
            bid: Some("bid-bench".into()),
            method: Some("thing.service.reboot".into()),
            data: serde_json::json!({"result": 0, "message": "success"}),
            timestamp: Some(1700000000000),
            version: "1.0".into(),
            code: Some("200".into()),
            message: Some("success".into()),
            product_key: Some("pk001".into()),
            device_id: Some("did001".into()),
        };

        let iterations = 50_000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = serde_json::to_vec(&response).unwrap();
        }

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Serialization: {} iterations in {:?} ({:.0} ops/sec)",
            iterations, elapsed, ops_per_sec
        );

        assert!(
            ops_per_sec > 50_000.0,
            "Serialization too slow: {:.0} ops/sec (target: 50K+)",
            ops_per_sec
        );
    }
}
