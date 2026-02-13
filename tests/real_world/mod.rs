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
///
/// Test categories:
///   1. Config loading
///   2. MySQL: connection pool, CRUD on iot_device, model queries
///   3. Redis: device state, shadow, multi-link
///   4. MQTT:  connect, subscribe, publish, receive
///   5. Kafka: producer create + send event
///   6. End-to-end: MQTT message → topic parse → Redis write → DB query
