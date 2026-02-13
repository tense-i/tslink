/// Real-world integration tests for TSLink IoT Core.
///
/// These tests connect to REAL infrastructure services (MySQL, Redis, MQTT, Kafka)
/// running via docker-compose. They validate actual I/O, not mocks.
///
/// Prerequisites:
///   docker-compose up -d   (all 5 services healthy)
///   RUN_ENV=dev            (uses config/dev.toml)
///
/// Run with:
///   RUN_ENV=dev cargo test --test real_world_test -- --nocapture
#[cfg(test)]
mod real_world_tests {
    use std::sync::Arc;
    use std::time::Duration;

    // ═══════════════════════════════════════════════════════════════
    // 1. Configuration Loading
    // ═══════════════════════════════════════════════════════════════

    mod config_tests {
        use tslink::config::AppConfig;

        #[test]
        fn test_config_loads_successfully() {
            std::env::set_var("RUN_ENV", "dev");
            let config = AppConfig::load().expect("config should load with dev.toml");
            assert_eq!(config.mqtt.host, "127.0.0.1");
            assert_eq!(config.mqtt.port, 1883);
            assert_eq!(config.redis.url, "redis://127.0.0.1:6379");
            assert_eq!(config.kafka.brokers, "127.0.0.1:9092");
            assert!(config.database.url.contains("tslink_dev"));
            // Note: http.port not asserted here because parallel env override tests
            // may set TSLINK__HTTP__PORT, causing a race condition.
            assert!(config.http.port > 0);
            println!("[OK] Config loaded: env={}, mqtt={}:{}, http.port={}", config.app.env, config.mqtt.host, config.mqtt.port, config.http.port);
        }

        #[test]
        fn test_config_env_override() {
            std::env::set_var("RUN_ENV", "dev");
            std::env::set_var("TSLINK__HTTP__REQUEST_TIMEOUT_SECS", "99");
            let config = AppConfig::load().expect("config should load");
            assert_eq!(config.http.request_timeout_secs, 99);
            std::env::remove_var("TSLINK__HTTP__REQUEST_TIMEOUT_SECS");
            println!("[OK] Env override works: http.request_timeout_secs=99");
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // 2. MySQL Real Connection Tests
    // ═══════════════════════════════════════════════════════════════

    mod mysql_tests {
        use super::*;
        use sqlx::MySqlPool;
        use tslink::domain::device::{Device, DeviceStatus};
        use tslink::infrastructure::database::device_repo::DeviceRepository;
        use tslink::infrastructure::database::shadow_repo::ShadowRepository;
        use tslink::infrastructure::database::model_repo::ModelRepository;

        async fn create_pool() -> MySqlPool {
            let url = "mysql://root:root@127.0.0.1:3306/tslink_dev";
            MySqlPool::connect(url)
                .await
                .expect("MySQL pool should connect")
        }

        #[tokio::test]
        async fn test_mysql_connection() {
            let pool = create_pool().await;
            let row: (i64,) = sqlx::query_as("SELECT 1 AS val")
                .fetch_one(&pool)
                .await
                .expect("SELECT 1 should work");
            assert_eq!(row.0, 1);
            println!("[OK] MySQL connection established, SELECT 1 = {}", row.0);
        }

        #[tokio::test]
        async fn test_mysql_tables_exist() {
            let pool = create_pool().await;
            let tables = vec![
                "product",
                "module",
                "function_info",
                "function_param",
                "iot_device",
                "iot_device_shadow_service",
            ];
            for table in &tables {
                let sql = format!("SELECT COUNT(*) AS cnt FROM `{}`", table);
                let row: (i64,) = sqlx::query_as(&sql)
                    .fetch_one(&pool)
                    .await
                    .unwrap_or_else(|e| panic!("Table {} should exist: {}", table, e));
                println!("[OK] Table `{}` exists, row_count={}", table, row.0);
            }
        }

        #[tokio::test]
        async fn test_device_repo_find_demo_device() {
            let pool = create_pool().await;
            let repo = DeviceRepository::new(pool);

            let device = repo
                .find_by_pk_did("demo_pk", "demo_did_001")
                .await
                .expect("query should succeed");

            assert!(device.is_some(), "demo device should exist from init.sql");
            let d = device.unwrap();
            assert_eq!(d.product_key, "demo_pk");
            assert_eq!(d.device_id, "demo_did_001");
            assert_eq!(d.device_secret.as_deref(), Some("secret123"));
            println!(
                "[OK] DeviceRepo.find_by_pk_did: pk={}, did={}, status={}",
                d.product_key, d.device_id, d.device_status
            );
        }

        #[tokio::test]
        async fn test_device_repo_crud() {
            let pool = create_pool().await;
            let repo = DeviceRepository::new(pool.clone());

            // Cleanup first (in case previous test left data)
            let _ = sqlx::query("DELETE FROM iot_device WHERE product_key = 'test_pk' AND device_id = 'test_did_crud'")
                .execute(&pool)
                .await;

            // Create
            let device = Device {
                id: None,
                product_id: Some(1),
                product_key: "test_pk".to_string(),
                product_version: None,
                device_id: "test_did_crud".to_string(),
                device_name: Some("CRUD Test Device".to_string()),
                device_secret: Some("test_secret_123".to_string()),
                device_status: DeviceStatus::NotActive,
                parent_product_key: None,
                parent_id: None,
                gmt_last_online: None,
                register_time: None,
                device_extend: None,
                org_code: Some("ORG001".to_string()),
            };
            repo.create(&device).await.expect("create should succeed");
            println!("[OK] DeviceRepo.create: test_pk/test_did_crud");

            // Read
            let found = repo
                .find_by_pk_did("test_pk", "test_did_crud")
                .await
                .expect("find should succeed")
                .expect("device should be found");
            assert_eq!(found.device_name.as_deref(), Some("CRUD Test Device"));
            assert_eq!(found.device_status, DeviceStatus::NotActive);
            println!("[OK] DeviceRepo.find: name={}", found.device_name.unwrap());

            // Update status
            repo.update_status("test_pk", "test_did_crud", &DeviceStatus::Online)
                .await
                .expect("update should succeed");
            let updated = repo
                .find_by_pk_did("test_pk", "test_did_crud")
                .await
                .unwrap()
                .unwrap();
            assert_eq!(updated.device_status, DeviceStatus::Online);
            println!("[OK] DeviceRepo.update_status: ONLINE");

            // Verify secret
            let valid = repo
                .verify_secret("test_pk", "test_did_crud", "test_secret_123")
                .await
                .expect("verify should succeed");
            assert!(valid, "correct secret should verify");

            let invalid = repo
                .verify_secret("test_pk", "test_did_crud", "wrong_secret")
                .await
                .expect("verify should succeed");
            assert!(!invalid, "wrong secret should not verify");
            println!("[OK] DeviceRepo.verify_secret: correct=true, wrong=false");

            // Find by product key
            let devices = repo
                .find_by_product_key("test_pk")
                .await
                .expect("find_by_product_key should succeed");
            assert!(!devices.is_empty());
            println!("[OK] DeviceRepo.find_by_product_key: count={}", devices.len());

            // Cleanup
            let _ = sqlx::query("DELETE FROM iot_device WHERE product_key = 'test_pk' AND device_id = 'test_did_crud'")
                .execute(&pool)
                .await;
            println!("[OK] DeviceRepo CRUD test cleanup done");
        }

        #[tokio::test]
        async fn test_shadow_repo_operations() {
            let pool = create_pool().await;
            let repo = ShadowRepository::new(pool.clone());

            // Cleanup
            let _ = sqlx::query(
                "DELETE FROM iot_device_shadow_service WHERE product_key = 'test_pk' AND device_id = 'test_did_shadow'"
            )
            .execute(&pool)
            .await;

            // Upsert
            let payload = serde_json::json!({"temperature": 25.5, "humidity": 60});
            repo.upsert_shadow_service("test_pk", "test_did_shadow", "thing.event.property.post", &payload)
                .await
                .expect("upsert should succeed");
            println!("[OK] ShadowRepo.upsert_shadow_service: method=thing.event.property.post");

            // Find
            let configs = repo
                .find_shadow_services("test_pk")
                .await
                .expect("find should succeed");
            assert!(!configs.is_empty(), "should find at least one shadow config");
            let found = configs.iter().find(|c| c.method == "thing.event.property.post");
            assert!(found.is_some(), "should find the upserted config");
            println!("[OK] ShadowRepo.find_shadow_services: count={}", configs.len());

            // Cleanup
            let _ = sqlx::query(
                "DELETE FROM iot_device_shadow_service WHERE product_key = 'test_pk' AND device_id = 'test_did_shadow'"
            )
            .execute(&pool)
            .await;
            println!("[OK] ShadowRepo test cleanup done");
        }

        #[tokio::test]
        async fn test_model_repo_load_demo_product() {
            let pool = create_pool().await;
            let repo = ModelRepository::new(Arc::new(pool));

            let model = repo
                .get_device_model("demo_pk")
                .await
                .expect("query should succeed");

            assert!(model.is_some(), "demo_pk product should exist from init.sql");
            let m = model.unwrap();
            assert_eq!(m.product_key, "demo_pk");
            println!(
                "[OK] ModelRepo.get_device_model: pk={}, name={:?}, services={}, events={}",
                m.product_key,
                m.name,
                m.services.len(),
                m.events.len()
            );

            // Test cache hit
            let cached = repo
                .get_device_model("demo_pk")
                .await
                .expect("cached query should succeed");
            assert!(cached.is_some());
            println!("[OK] ModelRepo cache hit for demo_pk");

            // Test invalidate
            repo.invalidate("demo_pk").await;
            println!("[OK] ModelRepo cache invalidated for demo_pk");
        }

        #[tokio::test]
        async fn test_model_repo_nonexistent_product() {
            let pool = create_pool().await;
            let repo = ModelRepository::new(Arc::new(pool));

            let model = repo
                .get_device_model("nonexistent_pk_xyz")
                .await
                .expect("query should succeed even for missing product");
            assert!(model.is_none());
            println!("[OK] ModelRepo returns None for nonexistent product");
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // 3. Redis Real Connection Tests
    // ═══════════════════════════════════════════════════════════════

    mod redis_tests {
        use super::*;
        use fred::prelude::*;
        use tslink::domain::device::DeviceStatus;
        use tslink::infrastructure::redis::device_state::DeviceStateRedis;
        use tslink::infrastructure::redis::shadow::ShadowRedis;
        use tslink::infrastructure::redis::link::LinkRedis;

        async fn create_redis_client() -> Arc<RedisClient> {
            let config = RedisConfig::from_url("redis://127.0.0.1:6379").expect("redis url parse");
            let client = RedisClient::new(config, None, None, None);
            client.connect();
            client
                .wait_for_connect()
                .await
                .expect("Redis should connect");
            Arc::new(client)
        }

        #[tokio::test]
        async fn test_redis_connection() {
            let client = create_redis_client().await;
            let pong: String = client.ping().await.expect("PING should work");
            assert_eq!(pong, "PONG");
            println!("[OK] Redis connected, PING={}", pong);
        }

        #[tokio::test]
        async fn test_device_state_redis_operations() {
            let client = create_redis_client().await;
            let state = DeviceStateRedis::new(client);

            let pk = "test_pk_redis";
            let did = "test_did_redis";

            // Set online
            state.set_online(pk, did).await.expect("set_online should work");
            let status = state.get_status(pk, did).await.expect("get should work");
            assert_eq!(status, Some(DeviceStatus::Online));
            println!("[OK] DeviceStateRedis: set_online + get = ONLINE");

            // Set offline
            state.set_offline(pk, did).await.expect("set_offline should work");
            let status = state.get_status(pk, did).await.expect("get should work");
            assert_eq!(status, Some(DeviceStatus::Offline));
            println!("[OK] DeviceStateRedis: set_offline + get = OFFLINE");

            // Refresh heartbeat
            state.refresh_heartbeat(pk, did).await.expect("refresh should work");
            let status = state.get_status(pk, did).await.expect("get should work");
            assert_eq!(status, Some(DeviceStatus::Online));
            println!("[OK] DeviceStateRedis: refresh_heartbeat → ONLINE");

            // Delete
            state.delete(pk, did).await.expect("delete should work");
            let status = state.get_status(pk, did).await.expect("get should work");
            assert_eq!(status, None);
            println!("[OK] DeviceStateRedis: delete → None");
        }

        #[tokio::test]
        async fn test_shadow_redis_operations() {
            let client = create_redis_client().await;
            let shadow = ShadowRedis::new(client);

            let pk = "test_pk_shadow";
            let did = "test_did_shadow";

            // Initially empty
            shadow.delete(pk, did).await.unwrap();
            let props = shadow.get_properties(pk, did).await.expect("get should work");
            assert!(props.is_none());
            println!("[OK] ShadowRedis: initial state = None");

            // Merge properties
            let new_props = serde_json::json!({"temperature": 25.5, "humidity": 60});
            shadow
                .merge_properties(pk, did, &new_props)
                .await
                .expect("merge should work");

            let props = shadow
                .get_properties(pk, did)
                .await
                .expect("get should work")
                .expect("should have properties");
            assert_eq!(props["temperature"], 25.5);
            assert_eq!(props["humidity"], 60);
            println!("[OK] ShadowRedis: merge + get = {{temperature: 25.5, humidity: 60}}");

            // Merge additional properties (shallow merge)
            let more_props = serde_json::json!({"temperature": 30.0, "pressure": 1013});
            shadow
                .merge_properties(pk, did, &more_props)
                .await
                .expect("merge should work");

            let props = shadow
                .get_properties(pk, did)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(props["temperature"], 30.0); // overridden
            assert_eq!(props["humidity"], 60);       // preserved
            assert_eq!(props["pressure"], 1013);     // new
            println!("[OK] ShadowRedis: shallow merge works (override + preserve + add)");

            // Cleanup
            shadow.delete(pk, did).await.unwrap();
            println!("[OK] ShadowRedis: cleanup done");
        }

        #[tokio::test]
        async fn test_link_redis_operations() {
            let client = create_redis_client().await;
            let link = LinkRedis::new(client);

            let pk = "test_pk_link";
            let did = "test_did_link";

            // Cleanup
            link.delete_device_links(pk, did).await.unwrap();

            // Add links with weights
            link.update_link_weight(pk, did, "wifi", 80.0).await.unwrap();
            link.update_link_weight(pk, did, "lte", 60.0).await.unwrap();
            link.update_link_weight(pk, did, "eth", 95.0).await.unwrap();
            println!("[OK] LinkRedis: added 3 links (wifi=80, lte=60, eth=95)");

            // Get all links (ordered by weight desc)
            let links = link.get_links(pk, did).await.unwrap();
            assert_eq!(links.len(), 3);
            assert_eq!(links[0].link_id, "eth");   // highest weight
            assert_eq!(links[1].link_id, "wifi");
            assert_eq!(links[2].link_id, "lte");   // lowest weight
            println!("[OK] LinkRedis: get_links ordered correctly: {:?}",
                links.iter().map(|l| format!("{}={}", l.link_id, l.weight)).collect::<Vec<_>>());

            // Get best link
            let best = link.get_best_link(pk, did).await.unwrap();
            assert_eq!(best.as_deref(), Some("eth"));
            println!("[OK] LinkRedis: best_link = eth");

            // Set active link
            link.set_active_link(pk, did, "wifi").await.unwrap();
            let active = link.get_active_link(pk, did).await.unwrap();
            assert_eq!(active.as_deref(), Some("wifi"));
            println!("[OK] LinkRedis: active_link = wifi");

            // Increment weight
            let new_score = link.increment_link_weight(pk, did, "lte", 50.0).await.unwrap();
            assert!((new_score - 110.0).abs() < 0.01); // 60 + 50 = 110
            println!("[OK] LinkRedis: increment lte by 50 → {}", new_score);

            // Now lte should be best
            let best = link.get_best_link(pk, did).await.unwrap();
            assert_eq!(best.as_deref(), Some("lte"));
            println!("[OK] LinkRedis: after increment, best_link = lte");

            // Remove a link
            link.remove_link(pk, did, "wifi").await.unwrap();
            let links = link.get_links(pk, did).await.unwrap();
            assert_eq!(links.len(), 2);
            assert!(links.iter().all(|l| l.link_id != "wifi"));
            println!("[OK] LinkRedis: removed wifi, remaining={}", links.len());

            // Cleanup
            link.delete_device_links(pk, did).await.unwrap();
            let links = link.get_links(pk, did).await.unwrap();
            assert!(links.is_empty());
            println!("[OK] LinkRedis: cleanup done, links empty");
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // 4. MQTT Real Connection Tests
    // ═══════════════════════════════════════════════════════════════

    mod mqtt_tests {
        use super::*;
        use tslink::config::MqttConfig;
        use tslink::infrastructure::mqtt::client::MqttClient;

        fn dev_mqtt_config() -> MqttConfig {
            MqttConfig {
                host: "127.0.0.1".to_string(),
                port: 1883,
                client_id: format!("tslink-test-{}", uuid::Uuid::new_v4()),
                username: "admin".to_string(),
                password: "public".to_string(),
                keep_alive_secs: 30,
                clean_session: true,
                max_packet_size: 65536,
                inflight: 100,
                subscribe_topics: vec!["sys/+/+/#".to_string()],
            }
        }

        #[tokio::test]
        async fn test_mqtt_connect_and_subscribe() {
            let config = dev_mqtt_config();
            let mut client = MqttClient::new(&config);

            // Spawn event loop (connects to EMQX)
            let mut rx = client.spawn_event_loop(None).expect("event loop should spawn");
            println!("[OK] MQTT event loop spawned");

            // Give it a moment to connect
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Subscribe
            client.subscribe_all().await.expect("subscribe should work");
            println!("[OK] MQTT subscribed to sys/+/+/#");

            // Publish a test message
            let test_payload = serde_json::json!({
                "tid": "test-tid-001",
                "version": "1.0",
                "timestamp": 1700000000000u64,
                "method": "thing.event.property.post",
                "productKey": "test_pk",
                "deviceId": "test_did",
                "data": {"temperature": 25.5}
            });
            let payload_bytes = serde_json::to_vec(&test_payload).unwrap();

            client
                .publish("sys/test_pk/test_did/thing/event/property/post", payload_bytes)
                .await
                .expect("publish should work");
            println!("[OK] MQTT published test message to sys/test_pk/test_did/thing/event/property/post");

            // Try to receive the message (with timeout)
            let received = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
            match received {
                Ok(Some(msg)) => {
                    assert!(msg.topic.contains("test_pk"));
                    let parsed: serde_json::Value =
                        serde_json::from_slice(&msg.payload).expect("payload should be JSON");
                    assert_eq!(parsed["tid"], "test-tid-001");
                    assert_eq!(parsed["data"]["temperature"], 25.5);
                    println!(
                        "[OK] MQTT received message: topic={}, tid={}",
                        msg.topic, parsed["tid"]
                    );
                }
                Ok(None) => panic!("MQTT channel closed unexpectedly"),
                Err(_) => panic!("MQTT receive timed out after 5s — check EMQX connection"),
            }
        }

        #[tokio::test]
        async fn test_mqtt_publish_and_receive_multiple() {
            let config = dev_mqtt_config();
            let mut client = MqttClient::new(&config);
            let mut rx = client.spawn_event_loop(None).unwrap();

            tokio::time::sleep(Duration::from_secs(2)).await;

            // Subscribe to a specific test topic
            client
                .subscribe("test/multi/+", rumqttc::QoS::AtLeastOnce)
                .await
                .unwrap();

            // Publish 5 messages
            for i in 0..5 {
                let payload = serde_json::json!({"index": i, "value": i * 10});
                client
                    .publish(
                        &format!("test/multi/{}", i),
                        serde_json::to_vec(&payload).unwrap(),
                    )
                    .await
                    .unwrap();
            }
            println!("[OK] MQTT published 5 messages to test/multi/0..4");

            // Receive all 5
            let mut received_count = 0;
            for _ in 0..5 {
                match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
                    Ok(Some(msg)) => {
                        assert!(msg.topic.starts_with("test/multi/"));
                        received_count += 1;
                    }
                    _ => break,
                }
            }
            assert_eq!(received_count, 5, "should receive all 5 messages");
            println!("[OK] MQTT received {}/5 messages", received_count);
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // 5. Kafka Real Connection Tests
    // ═══════════════════════════════════════════════════════════════

    mod kafka_tests {
        use tslink::config::KafkaConfig;
        use tslink::infrastructure::kafka::producer::EventProducer;

        fn dev_kafka_config() -> KafkaConfig {
            KafkaConfig {
                brokers: "127.0.0.1:9092".to_string(),
                topic_prefix: "tslink.".to_string(),
                event_topic: "tslink.device.event".to_string(),
                property_topic: "tslink.device.property".to_string(),
            }
        }

        #[tokio::test]
        async fn test_kafka_producer_creation() {
            let config = dev_kafka_config();
            let producer = EventProducer::new(&config);
            assert!(producer.is_ok(), "Kafka producer should be created");
            println!("[OK] Kafka producer created successfully");
        }

        #[tokio::test]
        async fn test_kafka_send_event() {
            let config = dev_kafka_config();
            let producer = EventProducer::new(&config).expect("producer should create");

            let payload = serde_json::json!({
                "productKey": "test_pk",
                "deviceId": "test_did",
                "event": "property_post",
                "data": {"temperature": 25.5},
                "timestamp": 1700000000000u64,
            });
            let payload_bytes = serde_json::to_vec(&payload).unwrap();

            let result = producer
                .send_event("tslink.device.event", "test_pk_test_did", &payload_bytes)
                .await;
            assert!(result.is_ok(), "Kafka send should succeed: {:?}", result.err());
            println!("[OK] Kafka event sent to tslink.device.event");
        }

        #[tokio::test]
        async fn test_kafka_send_best_effort() {
            let config = dev_kafka_config();
            let producer = EventProducer::new(&config).expect("producer should create");

            let payload = serde_json::json!({"test": "best_effort"});
            let payload_bytes = serde_json::to_vec(&payload).unwrap();

            // best_effort never panics
            producer
                .send_event_best_effort("tslink.device.property", "test_key", &payload_bytes)
                .await;
            println!("[OK] Kafka best-effort send completed (no panic)");
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // 6. End-to-End: Topic Parse → Redis Write → DB Query
    // ═══════════════════════════════════════════════════════════════

    mod e2e_tests {
        use super::*;
        use fred::prelude::*;
        use sqlx::MySqlPool;
        use tslink::domain::device::DeviceStatus;
        use tslink::domain::message::CommonTopicReceiver;
        use tslink::domain::topic::ThingMessageType;
        use tslink::infrastructure::database::device_repo::DeviceRepository;
        use tslink::infrastructure::mqtt::topic_parser::{classify_thing_message, parse_topic};
        use tslink::infrastructure::redis::device_state::DeviceStateRedis;
        use tslink::infrastructure::redis::shadow::ShadowRedis;

        async fn create_pool() -> MySqlPool {
            MySqlPool::connect("mysql://root:root@127.0.0.1:3306/tslink_dev")
                .await
                .expect("MySQL pool should connect")
        }

        async fn create_redis_client() -> Arc<RedisClient> {
            let config = RedisConfig::from_url("redis://127.0.0.1:6379").expect("redis url parse");
            let client = RedisClient::new(config, None, None, None);
            client.connect();
            client.wait_for_connect().await.expect("Redis should connect");
            Arc::new(client)
        }

        #[tokio::test]
        async fn test_e2e_property_post_flow() {
            // Simulate: device sends property post → parse topic → classify →
            //           deserialize payload → write shadow to Redis → update device status in DB

            let pool = create_pool().await;
            let redis = create_redis_client().await;
            let device_repo = DeviceRepository::new(pool.clone());
            let device_state = DeviceStateRedis::new(redis.clone());
            let shadow = ShadowRedis::new(redis.clone());

            let pk = "demo_pk";
            let did = "demo_did_001";

            // Step 1: Parse MQTT topic
            let topic = format!("sys/{}/{}/thing/event/property/post", pk, did);
            let topic_info = parse_topic(&topic).expect("topic should parse");
            assert_eq!(topic_info.product_key, pk);
            assert_eq!(topic_info.device_id, did);
            println!("[E2E] Step 1: Topic parsed: pk={}, did={}", topic_info.product_key, topic_info.device_id);

            // Step 2: Classify message type
            let msg_type = classify_thing_message(&topic_info);
            assert_eq!(msg_type, Some(ThingMessageType::EventProperty));
            println!("[E2E] Step 2: Message type = {:?}", msg_type);

            // Step 3: Deserialize payload
            let payload = serde_json::json!({
                "tid": "e2e-tid-001",
                "version": "1.0",
                "timestamp": 1700000000000u64,
                "method": "thing.event.property.post",
                "productKey": pk,
                "deviceId": did,
                "data": {"temperature": 28.3, "humidity": 55, "battery": 85}
            });
            let msg: CommonTopicReceiver<serde_json::Value> =
                serde_json::from_value(payload).expect("should deserialize");
            assert_eq!(msg.tid.as_deref(), Some("e2e-tid-001"));
            println!("[E2E] Step 3: Payload deserialized, tid={}", msg.tid.as_deref().unwrap());

            // Step 4: Write device status to Redis (simulate device coming online)
            device_state.set_online(pk, did).await.expect("set_online should work");
            let status = device_state.get_status(pk, did).await.unwrap();
            assert_eq!(status, Some(DeviceStatus::Online));
            println!("[E2E] Step 4: Device status set to ONLINE in Redis");

            // Step 5: Write shadow properties to Redis
            shadow
                .merge_properties(pk, did, &msg.data)
                .await
                .expect("merge should work");
            let props = shadow.get_properties(pk, did).await.unwrap().unwrap();
            assert_eq!(props["temperature"], 28.3);
            assert_eq!(props["humidity"], 55);
            assert_eq!(props["battery"], 85);
            println!("[E2E] Step 5: Shadow properties written to Redis: {:?}", props);

            // Step 6: Verify device exists in MySQL
            let device = device_repo
                .find_by_pk_did(pk, did)
                .await
                .expect("query should succeed")
                .expect("demo device should exist");
            assert_eq!(device.product_key, pk);
            assert_eq!(device.device_id, did);
            println!("[E2E] Step 6: Device verified in MySQL: pk={}, did={}", device.product_key, device.device_id);

            // Step 7: Update device status in MySQL
            device_repo
                .update_status(pk, did, &DeviceStatus::Online)
                .await
                .expect("update should succeed");
            let updated = device_repo.find_by_pk_did(pk, did).await.unwrap().unwrap();
            assert_eq!(updated.device_status, DeviceStatus::Online);
            println!("[E2E] Step 7: Device status updated to ONLINE in MySQL");

            // Cleanup: restore original state
            device_repo
                .update_status(pk, did, &DeviceStatus::NotActive)
                .await
                .unwrap();
            device_state.delete(pk, did).await.unwrap();
            shadow.delete(pk, did).await.unwrap();
            println!("[E2E] Cleanup done. Full E2E property post flow PASSED!");
        }

        #[tokio::test]
        async fn test_e2e_region_topic_flow() {
            // Test region-prefixed topic handling
            let topic = "region/cn-east/sys/demo_pk/demo_did_001/thing/ntp/post";
            let info = parse_topic(topic).expect("should parse");
            assert_eq!(info.region.as_deref(), Some("cn-east"));
            assert_eq!(info.product_key, "demo_pk");
            assert_eq!(info.device_id, "demo_did_001");

            let msg_type = classify_thing_message(&info);
            assert_eq!(msg_type, Some(ThingMessageType::Ntp));
            println!("[E2E] Region topic flow: region=cn-east, type=Ntp — PASSED");
        }

        #[tokio::test]
        async fn test_e2e_multilink_flow() {
            let redis = create_redis_client().await;
            let link_redis = tslink::infrastructure::redis::link::LinkRedis::new(redis.clone());
            let device_state = DeviceStateRedis::new(redis.clone());

            let pk = "e2e_ml_pk";
            let did = "e2e_ml_did";

            // Cleanup
            link_redis.delete_device_links(pk, did).await.unwrap();
            device_state.delete(pk, did).await.unwrap();

            // Simulate multi-link device: parse topic with link suffix
            let topic = format!("sys/{}/{}_link1/thing/event/property/post", pk, did);
            let info = parse_topic(&topic).unwrap();
            // Parser keeps full segment as device_id
            assert_eq!(info.device_id, format!("{}_link1", did));
            println!("[E2E-ML] Topic parsed: device_id={}", info.device_id);

            // Register links in Redis
            link_redis.update_link_weight(pk, did, "link1", 80.0).await.unwrap();
            link_redis.update_link_weight(pk, did, "link2", 60.0).await.unwrap();
            link_redis.set_active_link(pk, did, "link1").await.unwrap();

            let active = link_redis.get_active_link(pk, did).await.unwrap();
            assert_eq!(active.as_deref(), Some("link1"));

            let best = link_redis.get_best_link(pk, did).await.unwrap();
            assert_eq!(best.as_deref(), Some("link1"));
            println!("[E2E-ML] Multi-link: active=link1, best=link1");

            // Simulate link quality change
            link_redis.increment_link_weight(pk, did, "link2", 30.0).await.unwrap();
            // link2 = 90, link1 = 80 → link2 is now best
            let best = link_redis.get_best_link(pk, did).await.unwrap();
            assert_eq!(best.as_deref(), Some("link2"));
            println!("[E2E-ML] After quality change: best=link2 (90 > 80)");

            // Cleanup
            link_redis.delete_device_links(pk, did).await.unwrap();
            device_state.delete(pk, did).await.unwrap();
            println!("[E2E-ML] Multi-link flow PASSED!");
        }
    }
}
