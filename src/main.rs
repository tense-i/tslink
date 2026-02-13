mod application;
mod config;
mod domain;
mod error;
mod infrastructure;
mod telemetry;

use crate::application::device_service::DeviceService;
use crate::application::event_bus::DeviceEventBus;
use crate::application::event_service::EventService;
use crate::application::config_service::ConfigService;
use crate::application::device_type_service::DeviceTypeService;
use crate::application::function_param_service::FunctionParamService;
use crate::application::discovery_service::DiscoveryService;
use crate::application::handlers::config_handler::ConfigHandler;
use crate::application::handlers::discovery_handler::DiscoveryHandler;
use crate::application::handlers::event_handler::EventHandler;
use crate::application::handlers::heartbeat_handler::HeartbeatHandler;
use crate::application::handlers::ntp_handler::NtpHandler;
use crate::application::handlers::property_handler::PropertyHandler;
use crate::application::handlers::register_handler::RegisterHandler;
use crate::application::handlers::service_reply_handler::ServiceReplyHandler;
use crate::application::link_service::LinkService;
use crate::application::ntp_service::NtpService;
use crate::application::product_function_service::ProductFunctionService;
use crate::application::product_service::ProductService;
use crate::application::shadow_service::ShadowService;
use crate::application::thing_service::ThingService;
use crate::config::AppConfig;
use crate::infrastructure::database::device_config_repo::DeviceConfigRepository;
use crate::infrastructure::database::device_repo::DeviceRepository;
use crate::infrastructure::database::device_type_repo::DeviceTypeRepository;
use crate::infrastructure::database::function_param_repo::FunctionParamRepository;
use crate::infrastructure::database::module_repo::ModuleRepository;
use crate::infrastructure::database::model_repo::ModelRepository;
use crate::infrastructure::database::product_function_repo::ProductFunctionRepository;
use crate::infrastructure::database::product_repo::ProductRepository;
use crate::infrastructure::database::shadow_repo::ShadowRepository;
use crate::infrastructure::http::device_handler::DeviceState;
use crate::infrastructure::http::device_type_handler::DeviceTypeState;
use crate::infrastructure::http::function_param_handler::FunctionParamState;
use crate::infrastructure::http::health::HealthState;
use crate::infrastructure::http::product_handler::ProductState;
use crate::infrastructure::http::routes::build_router;
use crate::infrastructure::http::service_handler::ServiceState;
use crate::infrastructure::http::shadow_handler::ShadowState;
use crate::infrastructure::http::ws_handler::WsState;
use crate::infrastructure::kafka::producer::EventProducer;
use crate::infrastructure::mqtt::client::MqttClient;
use crate::infrastructure::mqtt::config_mqtt_handler::ConfigMqttHandler;
use crate::infrastructure::mqtt::discovery_mqtt_handler::DiscoveryMqttHandler;
use crate::infrastructure::mqtt::publisher::MessagePublisher;
use crate::infrastructure::mqtt::router::MessageRouter;
use crate::infrastructure::redis::device_state::DeviceStateRedis;
use crate::infrastructure::redis::link::LinkRedis;
use crate::infrastructure::redis::shadow::ShadowRedis;
use fred::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // ── 1. Load configuration ────────────────────────────────────
    let config = AppConfig::load().expect("Failed to load configuration");

    // ── 2. Initialize telemetry (tracing + prometheus) ───────────
    let metrics = telemetry::init_telemetry();
    let metrics = Arc::new(metrics);
    tracing::info!(
        app = %config.app.name,
        env = %config.app.env,
        "TSLink IoT Core starting"
    );

    // ── 3. Log configuration summary ─────────────────────────────
    tracing::info!(
        mqtt_broker = %format!("{}:{}", config.mqtt.host, config.mqtt.port),
        mqtt_client_id = %config.mqtt.client_id,
        redis_url = %config.redis.url,
        kafka_brokers = %config.kafka.brokers,
        http_bind = %format!("{}:{}", config.http.host, config.http.port),
        "configuration loaded"
    );

    // ── 4. Connect to Redis ──────────────────────────────────────
    let redis_config =
        RedisConfig::from_url(&config.redis.url).expect("Failed to parse Redis URL");
    let redis_client = RedisClient::new(redis_config, None, None, None);
    redis_client.connect();
    redis_client
        .wait_for_connect()
        .await
        .expect("Failed to connect to Redis");
    let redis_client = Arc::new(redis_client);
    tracing::info!(url = %config.redis.url, "Redis connected");

    // ── 5. Connect to MySQL ──────────────────────────────────────
    let mysql_pool = sqlx::MySqlPool::connect(&config.database.url)
        .await
        .expect("Failed to connect to MySQL");
    let mysql_pool_arc = Arc::new(mysql_pool.clone());
    tracing::info!(url = %config.database.url, "MySQL connected");

    // ── 6. Create Kafka producer ─────────────────────────────────
    let kafka_producer = Arc::new(
        EventProducer::new(&config.kafka).expect("Failed to create Kafka producer"),
    );
    tracing::info!(brokers = %config.kafka.brokers, "Kafka producer created");

    // ── 7. Initialize MQTT client ────────────────────────────────
    let mut mqtt_client = MqttClient::new(&config.mqtt);
    let publisher = Arc::new(MessagePublisher::new(mqtt_client.client_handle()));

    // Spawn event loop with PostSync waiter integration
    let sync_waiters = publisher.waiters();
    let mut mqtt_rx = mqtt_client
        .spawn_event_loop(Some(sync_waiters))
        .expect("Failed to spawn MQTT event loop");

    // Subscribe to configured topics
    mqtt_client
        .subscribe_all()
        .await
        .expect("Failed to subscribe to MQTT topics");
    tracing::info!("MQTT client connected and subscribed");

    // ── 8. Build infrastructure layer ────────────────────────────
    let device_state_redis = Arc::new(DeviceStateRedis::new(redis_client.clone()));
    let shadow_redis = Arc::new(ShadowRedis::new(redis_client.clone()));
    let link_redis = Arc::new(LinkRedis::new(redis_client.clone()));

    let device_repo = Arc::new(DeviceRepository::new(mysql_pool.clone()));
    let device_config_repo = Arc::new(DeviceConfigRepository::new(mysql_pool.clone()));
    let device_type_repo = Arc::new(DeviceTypeRepository::new(mysql_pool.clone()));
    let function_param_repo = Arc::new(FunctionParamRepository::new(mysql_pool.clone()));
    let module_repo = Arc::new(ModuleRepository::new(mysql_pool.clone()));
    let product_repo = Arc::new(ProductRepository::new(mysql_pool.clone()));
    let function_repo = Arc::new(ProductFunctionRepository::new(mysql_pool.clone()));
    let shadow_repo = Arc::new(ShadowRepository::new(mysql_pool.clone()));
    let model_repo = Arc::new(ModelRepository::new(mysql_pool_arc));

    // Warmup model cache (best-effort, don't block startup on failure)
    match model_repo.warmup().await {
        Ok(n) => tracing::info!(count = n, "model cache warmup done"),
        Err(e) => tracing::warn!(error = %e, "model cache warmup failed (non-fatal)"),
    }

    // ── 8b. Create event bus for WebSocket push ───────────────────
    let event_bus = DeviceEventBus::new();
    tracing::info!("device event bus created");

    // ── 9. Build application services ────────────────────────────
    let device_service = Arc::new(
        DeviceService::new(
            device_state_redis.clone(),
            device_repo.clone(),
            product_repo.clone(),
        )
        .with_event_bus(event_bus.clone()),
    );
    let shadow_service = Arc::new(
        ShadowService::new(shadow_redis.clone(), shadow_repo.clone(), publisher.clone())
            .with_event_bus(event_bus.clone()),
    );
    let link_service = Arc::new(LinkService::new(link_redis.clone()));
    let product_function_service = Arc::new(ProductFunctionService::new(
        product_repo.clone(),
        module_repo.clone(),
        function_repo.clone(),
    ));
    let product_service = Arc::new(ProductService::new(
        product_repo.clone(),
        device_repo.clone(),
    ));
    let event_service = Arc::new(EventService::new(
        publisher.clone(),
        kafka_producer.clone(),
    ));
    let ntp_service = Arc::new(NtpService::new(publisher.clone()));
    let config_service = Arc::new(ConfigService::new(device_config_repo.clone()));
    let discovery_service = Arc::new(DiscoveryService::new(device_repo.clone()));
    let device_type_service = Arc::new(DeviceTypeService::new(device_type_repo.clone()));
    let function_param_service = Arc::new(FunctionParamService::new(function_param_repo.clone()));
    let thing_service = Arc::new(
        ThingService::new(model_repo.clone(), publisher.clone(), shadow_service.clone())
            .with_link_service(link_service.clone()),
    );

    // ── 10. Initialize Topic Router with real handlers ───────────
    let mut router = MessageRouter::new().with_metrics(metrics.clone());
    router.register(Arc::new(HeartbeatHandler::new(device_service.clone())));
    router.register(Arc::new(RegisterHandler::new(
        device_service.clone(),
        publisher.clone(),
    )));
    router.register(Arc::new(PropertyHandler::new(shadow_service.clone())));
    router.register(Arc::new(ServiceReplyHandler::new(publisher.clone())));
    router.register(Arc::new(EventHandler::new(event_service.clone())));
    router.register(Arc::new(NtpHandler::new(ntp_service.clone())));
    let config_handler = Arc::new(ConfigHandler::new(config_service.clone()));
    router.register(Arc::new(ConfigMqttHandler::new(
        config_handler,
        publisher.clone(),
    )));
    let discovery_handler = Arc::new(DiscoveryHandler::new(discovery_service.clone()));
    router.register(Arc::new(DiscoveryMqttHandler::new(
        discovery_handler,
        publisher.clone(),
    )));

    tracing::info!(
        handler_count = router.handler_count(),
        "topic router initialized with real handlers"
    );

    let router = Arc::new(router);

    // ── 11. Spawn message processing loop ────────────────────────
    let router_handle = router.clone();
    tokio::spawn(async move {
        while let Some(msg) = mqtt_rx.recv().await {
            if let Err(e) = router_handle.route(&msg.topic, &msg.payload).await {
                tracing::error!(
                    topic = %msg.topic,
                    error = %e,
                    "message routing failed"
                );
            }
        }
        tracing::warn!("MQTT message channel closed");
    });

    // ── 12. Start HTTP server ────────────────────────────────────
    let http_config = config.http.clone();
    let device_state = DeviceState {
        device_service: device_service.clone(),
    };
    let service_state = ServiceState {
        thing_service: thing_service.clone(),
    };
    let shadow_state = ShadowState {
        shadow_service: shadow_service.clone(),
    };
    let health_state = HealthState {
        metrics: metrics.clone(),
    };
    let product_state = ProductState {
        product_service: product_service.clone(),
        function_service: product_function_service.clone(),
    };

    let ws_state = WsState {
        event_bus: event_bus.clone(),
    };

    let device_type_state = DeviceTypeState {
        service: device_type_service.clone(),
    };

    let function_param_state = FunctionParamState {
        service: function_param_service.clone(),
    };

    let app_router = build_router(
        device_state,
        service_state,
        shadow_state,
        product_state,
        health_state,
        ws_state,
        device_type_state,
        function_param_state,
    );
    tokio::spawn(async move {
        if let Err(e) =
            infrastructure::http::server::start_http_server(&http_config, app_router).await
        {
            tracing::error!(error = %e, "HTTP server failed");
        }
    });
    tracing::info!(
        "HTTP server started on {}:{}",
        config.http.host,
        config.http.port
    );

    tracing::info!("all subsystems initialized — TSLink IoT Core is ready");

    // ── 13. Spawn SIGHUP config hot-reload listener ──────────────
    #[cfg(unix)]
    {
        let shared_config = config.into_shared();
        tokio::spawn(async move {
            let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
                .expect("failed to install SIGHUP handler");
            loop {
                sighup.recv().await;
                tracing::info!("SIGHUP received — reloading configuration");
                match AppConfig::reload() {
                    Ok(new_config) => {
                        let mut w = shared_config.write().await;
                        *w = new_config;
                        tracing::info!("configuration reloaded successfully");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "configuration reload failed — keeping old config");
                    }
                }
            }
        });
    }

    // ── 14. Wait for shutdown signal ─────────────────────────────
    shutdown_signal().await;
    tracing::info!("TSLink IoT Core shutting down");

    // Flush OpenTelemetry traces before exit
    tslink::telemetry::shutdown_otel();
}

/// Wait for Ctrl+C or SIGTERM (for K8s graceful shutdown).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
