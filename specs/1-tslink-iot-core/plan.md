# Implementation Plan: TSLink IoT Core

**Branch**: `1-tslink-iot-core` | **Date**: 2026-02-13 | **Spec**: specs/1-tslink-iot-core/spec.md
**Input**: Feature specification from `specs/1-tslink-iot-core/spec.md`

## Summary

使用 Rust 从零构建一个 IoT 设备通信平台内核（TSLink），替代现有 Java 控制中心的设备通信层。采用 hexagonal architecture，核心功能包括 MQTT 连接管理、V3 topic 路由、设备生命周期、设备影子、物模型服务调用/事件处理、多链路通信，以及 HTTP 管理 API。

## Technical Context

**Language/Version**: Rust 1.75+ (Edition 2021)  
**Primary Dependencies**: tokio (async runtime), rumqttc (MQTT client), axum (HTTP), serde/serde_json (serialization), redis (cache), rdkafka (Kafka), sqlx (MySQL), tracing (logging), prometheus (metrics)  
**Storage**: MySQL 8.x (设备表), Redis 6.x+ (影子/状态/链路)  
**Testing**: cargo test + testcontainers-rs (EMQX/Redis/Kafka 集成测试)  
**Target Platform**: Linux amd64/arm64 (K8s 部署)  
**Project Type**: single  
**Performance Goals**: 10K concurrent devices, P99 < 100ms, 50K msg/s throughput  
**Constraints**: <200MB memory, <5s startup, drop-in replacement for Java center  
**Scale/Scope**: 单服务部署，10K-50K 设备

## Constitution Check

| Principle | Status | Notes |
|-----------|--------|-------|
| I. IoT-Only Scope | ✅ PASS | 仅实现 MQTT/设备/物模型/影子，无任务调度/媒体/算法 |
| II. Rust-First Performance | ✅ PASS | tokio + rumqttc + serde 零拷贝 |
| III. Type-Safe Protocol | ✅ PASS | CommonTopicReceiver/Response 为强类型 struct |
| IV. Clean Architecture | ✅ PASS | domain/application/infrastructure 三层分离 |
| V. Observability by Default | ✅ PASS | tracing + prometheus + opentelemetry |

## Project Structure

### Documentation (this feature)

```text
specs/1-tslink-iot-core/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
└── tasks.md


tslink/
├── Cargo.toml
├── Cargo.lock
├── .env.example
├── config/
│   ├── default.toml           # 默认配置
│   └── dev.toml               # 开发环境配置
├── src/
│   ├── main.rs                # 入口：加载配置 → 启动 MQTT → 启动 HTTP
│   ├── config.rs              # 配置结构体（MQTT/Redis/Kafka/MySQL/HTTP）
│   ├── error.rs               # 全局错误类型
│   ├── domain/                # 领域层（零外部依赖）
│   │   ├── mod.rs
│   │   ├── device.rs          # Device 实体 + DeviceStatus 枚举
│   │   ├── thing_model.rs     # DeviceModel / FunctionMethod / MethodField
│   │   ├── shadow.rs          # DeviceShadow / ShadowConfig
│   │   ├── message.rs         # CommonTopicReceiver<T> / CommonTopicResponse<T>
│   │   ├── topic.rs           # TopicInfo 解析结果 / MessageType / DeviceMessageType 枚举
│   │   ├── link.rs            # Link 实体 / LinkWeight
│   │   └── event.rs           # DomainEvent 枚举（DeviceOnline/Offline/PropertyChanged/...）
│   ├── application/           # 应用层（用例编排）
│   │   ├── mod.rs
│   │   ├── device_service.rs  # 设备上下线处理、注册
│   │   ├── thing_service.rs   # 物模型服务调用 / 属性设置
│   │   ├── event_service.rs   # 事件处理 + Kafka 转发
│   │   ├── shadow_service.rs  # 影子读写 + 上线重放
│   │   ├── link_service.rs    # 多链路管理
│   │   └── ntp_service.rs     # NTP 时间同步
│   ├── infrastructure/        # 基础设施层
│   │   ├── mod.rs
│   │   ├── mqtt/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs      # rumqttc 客户端封装 + 重连逻辑
│   │   │   ├── router.rs      # Topic 路由引擎（正则匹配 → handler dispatch）
│   │   │   ├── handler.rs     # InboundMessageHandler trait + 各 handler 实现
│   │   │   ├── publisher.rs   # 消息发布 + request-reply 同步等待（PostSync）
│   │   │   └── topic_parser.rs # Topic 字符串解析 → TopicInfo
│   │   ├── redis/
│   │   │   ├── mod.rs
│   │   │   ├── device_state.rs # 设备在线状态 Redis 操作
│   │   │   ├── shadow.rs       # 影子属性 Redis 读写
│   │   │   └── link.rs         # 多链路权重 Redis Lua 脚本
│   │   ├── kafka/
│   │   │   ├── mod.rs
│   │   │   └── producer.rs     # 事件消息 Kafka 发送
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   ├── device_repo.rs  # 设备 CRUD (sqlx)
│   │   │   ├── shadow_repo.rs  # 影子配置 CRUD
│   │   │   └── model_repo.rs   # 物模型加载
│   │   └── http/
│   │       ├── mod.rs
│   │       ├── server.rs       # axum Server 启动
│   │       ├── routes.rs       # 路由定义
│   │       ├── device_handler.rs # 设备 API handler
│   │       ├── service_handler.rs # 服务调用 API handler
│   │       ├── shadow_handler.rs  # 影子 API handler
│   │       └── health.rs       # /health + /metrics
│   └── telemetry.rs           # tracing + prometheus 初始化
├── tests/
│   ├── common/
│   │   └── mod.rs             # 测试工具函数
│   ├── integration/
│   │   ├── mqtt_test.rs       # MQTT 连接+收发集成测试
│   │   ├── routing_test.rs    # Topic 路由集成测试
│   │   ├── device_test.rs     # 设备生命周期集成测试
│   │   ├── shadow_test.rs     # 影子读写集成测试
│   │   └── service_test.rs    # 服务调用集成测试
│   └── bench/
│       ├── routing_bench.rs   # Topic 路由性能基准
│       └── message_bench.rs   # 消息序列化性能基准
├── migrations/                 # sqlx 数据库迁移（如需要）
├── Dockerfile
└── deploy/
    ├── Chart.yaml             # Helm chart
    ├── values.yaml
    └── templates/


好的，以下是三个文件的完整内容。

1. specs/1-tslink-iot-core/spec.md
2. specs/1-tslink-iot-core/plan.md
Source Code (repository root)
Structure Decision: 选择 single project 结构。TSLink 是单一可部署服务，内部通过 Rust module 系统实现 hexagonal 分层，无需 workspace 多 crate（初期）。当模块成熟后可拆分为 crate。

Key Technology Decisions
Decision	Choice	Rationale	Alternatives Rejected
MQTT Client	rumqttc	生产验证、async 支持、性能好	paho-mqtt (同步为主)
HTTP Framework	axum	tokio 原生、强类型路由、性能顶级	actix-web (actor 模型过重)
Redis Client	fred	全功能 async、连接池、Lua 脚本支持	redis-rs (缺少集群支持)
Serialization	serde + serde_json	Rust 标准选择、零拷贝、derive 宏	无
Database	sqlx	编译时 SQL 检查、async、MySQL 支持	diesel (同步、ORM 过重)
Kafka	rdkafka	librdkafka 绑定、成熟稳定	kafka-rust (不够成熟)
Config	config crate	支持 TOML/YAML/Env 层叠	figment (API 类似但社区较小)
Logging	tracing	结构化日志、span 追踪、async 友好	log (不支持结构化)
Metrics	prometheus crate	标准 Prometheus 导出	metrics (抽象层多一层)
