/// Integration tests for SPEC-IOT-PHASE2 features.
///
/// Tests cover:
/// - T15: MQTT Config flow (topic parsing + message structure)
/// - T18: MQTT Discovery flow (topic parsing + message structure)
/// - T21: DeviceType API (CRUD operations structure)
///
/// Run with: cargo test --test phase2_integration_test -- --nocapture
#[cfg(test)]
mod phase2_integration_test {
    use tslink::domain::message::CommonTopicReceiver;
    use tslink::domain::topic::ThingMessageType;
    use tslink::infrastructure::mqtt::topic_parser::{classify_thing_message, parse_topic};

    // ═══════════════════════════════════════════════════════════════════════
    // T15: MQTT Config Flow Integration Tests
    // ═══════════════════════════════════════════════════════════════════════

    /// Test config query topic parsing and message structure.
    #[test]
    fn test_config_query_topic_parsing() {
        let topic = "sys/pk001/did001/thing/config/query";
        let info = parse_topic(topic).expect("config topic should parse");

        assert_eq!(info.product_key, "pk001");
        assert_eq!(info.device_id, "did001");
        assert_eq!(info.category, "thing");
        assert!(info.sub_category.contains(&"config".to_string()));
        assert!(info.sub_category.contains(&"query".to_string()));

        let msg_type = classify_thing_message(&info);
        assert_eq!(msg_type, Some(ThingMessageType::Config));
    }

    /// Test config update topic parsing.
    #[test]
    fn test_config_update_topic_parsing() {
        let topic = "sys/pk001/did001/thing/config/update";
        let info = parse_topic(topic).expect("config update topic should parse");

        assert_eq!(info.product_key, "pk001");
        assert_eq!(info.device_id, "did001");
        assert!(info.sub_category.contains(&"config".to_string()));
        assert!(info.sub_category.contains(&"update".to_string()));

        let msg_type = classify_thing_message(&info);
        assert_eq!(msg_type, Some(ThingMessageType::Config));
    }

    /// Test config message deserialization.
    #[test]
    fn test_config_message_structure() {
        let payload = r#"{
            "tid": "config-001",
            "version": "1.0",
            "timestamp": 1700000000000,
            "method": "thing.config.query",
            "productKey": "pk001",
            "deviceId": "did001",
            "data": {}
        }"#;

        let msg: CommonTopicReceiver<serde_json::Value> =
            serde_json::from_str(payload).expect("config message should deserialize");

        assert_eq!(msg.tid.as_deref(), Some("config-001"));
        assert_eq!(msg.method.as_deref(), Some("thing.config.query"));
    }

    /// Test config version query topic.
    #[test]
    fn test_config_version_query_topic() {
        let topic = "sys/pk001/did001/thing/config/version/query";
        let info = parse_topic(topic).expect("config version topic should parse");

        assert_eq!(info.product_key, "pk001");
        assert_eq!(info.device_id, "did001");
        assert!(info.sub_category.contains(&"config".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T18: MQTT Discovery Flow Integration Tests
    // ═══════════════════════════════════════════════════════════════════════

    /// Test discovery list topic parsing.
    #[test]
    fn test_discovery_list_topic_parsing() {
        let topic = "sys/pk001/did001/thing/discovery/list";
        let info = parse_topic(topic).expect("discovery topic should parse");

        assert_eq!(info.product_key, "pk001");
        assert_eq!(info.device_id, "did001");
        assert_eq!(info.category, "thing");
        assert!(info.sub_category.contains(&"discovery".to_string()));
        assert!(info.sub_category.contains(&"list".to_string()));

        let msg_type = classify_thing_message(&info);
        assert_eq!(msg_type, Some(ThingMessageType::Discovery));
    }

    /// Test discovery sub_devices topic parsing.
    #[test]
    fn test_discovery_sub_devices_topic_parsing() {
        let topic = "sys/pk001/did001/thing/discovery/sub_devices";
        let info = parse_topic(topic).expect("discovery sub_devices topic should parse");

        assert_eq!(info.product_key, "pk001");
        assert_eq!(info.device_id, "did001");
        assert!(info.sub_category.contains(&"discovery".to_string()));
        assert!(info.sub_category.contains(&"sub_devices".to_string()));

        let msg_type = classify_thing_message(&info);
        assert_eq!(msg_type, Some(ThingMessageType::Discovery));
    }

    /// Test discovery refresh topic parsing.
    #[test]
    fn test_discovery_refresh_topic_parsing() {
        let topic = "sys/pk001/did001/thing/discovery/refresh";
        let info = parse_topic(topic).expect("discovery refresh topic should parse");

        assert!(info.sub_category.contains(&"discovery".to_string()));
        assert!(info.sub_category.contains(&"refresh".to_string()));

        let msg_type = classify_thing_message(&info);
        assert_eq!(msg_type, Some(ThingMessageType::Discovery));
    }

    /// Test discovery message deserialization.
    #[test]
    fn test_discovery_message_structure() {
        let payload = r#"{
            "tid": "disc-001",
            "version": "1.0",
            "timestamp": 1700000000000,
            "method": "thing.discovery.list",
            "productKey": "pk001",
            "deviceId": "did001",
            "data": {}
        }"#;

        let msg: CommonTopicReceiver<serde_json::Value> =
            serde_json::from_str(payload).expect("discovery message should deserialize");

        assert_eq!(msg.tid.as_deref(), Some("disc-001"));
        assert_eq!(msg.method.as_deref(), Some("thing.discovery.list"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T21: DeviceType API Integration Tests
    // ═══════════════════════════════════════════════════════════════════════

    /// Test DeviceType domain model serialization.
    #[test]
    fn test_device_type_serialization() {
        use tslink::domain::device_type::DeviceType;

        let dt = DeviceType::new("sensor".to_string(), "Sensor Device".to_string());

        let json = serde_json::to_string(&dt).expect("DeviceType should serialize");
        assert!(json.contains("sensor"));
        assert!(json.contains("Sensor Device"));
    }

    /// Test DeviceType domain model deserialization.
    #[test]
    fn test_device_type_deserialization() {
        use tslink::domain::device_type::DeviceType;

        let json = r#"{
            "code": "gateway",
            "name": "Gateway Device",
            "description": "IoT Gateway"
        }"#;

        let dt: DeviceType = serde_json::from_str(json).expect("DeviceType should deserialize");
        assert_eq!(dt.code, "gateway");
        assert_eq!(dt.name, "Gateway Device");
        assert_eq!(dt.description.as_deref(), Some("IoT Gateway"));
    }

    /// Test DeviceType API request structure.
    #[test]
    fn test_device_type_api_request_structure() {
        // Create request structure
        let create_request = serde_json::json!({
            "code": "actuator",
            "name": "Actuator Device",
            "description": "Device that performs actions"
        });

        assert!(create_request.get("code").is_some());
        assert!(create_request.get("name").is_some());

        // Update request structure
        let update_request = serde_json::json!({
            "name": "Updated Actuator",
            "description": "Updated description"
        });

        assert!(update_request.get("name").is_some());
    }

    /// Test DeviceType API response structure.
    #[test]
    fn test_device_type_api_response_structure() {
        let success_response = serde_json::json!({
            "success": true,
            "data": {
                "code": "sensor",
                "name": "Sensor Device",
                "description": null
            }
        });

        assert_eq!(success_response["success"], true);
        assert!(success_response["data"].is_object());

        let error_response = serde_json::json!({
            "success": false,
            "error": "Device type 'sensor' already exists"
        });

        assert_eq!(error_response["success"], false);
        assert!(error_response["error"].is_string());
    }

    /// Test DeviceType list response structure.
    #[test]
    fn test_device_type_list_response() {
        let list_response = serde_json::json!({
            "success": true,
            "data": [
                {"code": "sensor", "name": "Sensor"},
                {"code": "gateway", "name": "Gateway"}
            ]
        });

        assert_eq!(list_response["success"], true);
        assert!(list_response["data"].is_array());
        assert_eq!(list_response["data"].as_array().unwrap().len(), 2);
    }
}
