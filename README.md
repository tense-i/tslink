# TSLink IoT Core

TSLink 是京安思控制中心 IoT 模块的 Rust 重写版本，提供高性能 MQTT 消息路由、设备管理、物模型服务和设备影子功能。

## 架构

```
┌──────────────────────────────────────────────────────────┐
│                     HTTP API (axum)                      │
│  GET /device/:pk/:did  POST /service/invoke              │
│  GET /shadow/:pk/:did  PUT /shadow/:pk/:did              │
│  GET /health           GET /metrics                      │
├──────────────────────────────────────────────────────────┤
│                  Application Services                    │
│  DeviceService · ThingService · ShadowService            │
│  EventService · NtpService · LinkService                 │
├──────────────────────────────────────────────────────────┤
│                  Message Handlers                        │
│  HeartbeatHandler · RegisterHandler · PropertyHandler    │
│  NtpHandler · EventHandler · ServiceReplyHandler         │
├──────────────────────────────────────────────────────────┤
│                  MQTT Router Engine                      │
│  TopicParser → classify_thing_message → MessageRouter    │
├────────────┬─────────┬──────────┬────────────────────────┤
│   MQTT     │  Redis  │  MySQL   │  Kafka                 │
│  (rumqttc) │ (fred)  │ (sqlx)   │ (rdkafka)              │
└────────────┴─────────┴──────────┴────────────────────────┘
```

**六边形架构分层**：

- **Domain** — 零外部依赖的核心类型：`Device`, `Message`, `Topic`, `ThingModel`, `Shadow`, `Link`, `Event`
- **Application** — 编排业务逻辑的服务层和消息处理器
- **Infrastructure** — 外部系统适配器：MQTT、Redis、MySQL、Kafka、HTTP

## 功能特性

- **MQTT 消息路由** — 支持 13 种 `ThingMessageType` 自动分类与分发
- **设备管理** — 注册、动态注册、心跳、在线/离线状态维护
- **物模型服务** — 同步/异步服务调用（PostSync），设备模型缓存（moka，30s TTL）
- **设备影子** — 属性上报、影子查询与更新、上线时影子重放
- **多链路通信** — Redis ZSet 权重管理，链路选择与切换
- **事件转发** — 自定义事件通过 Kafka 转发至 `iot-device-message` topic
- **NTP 时间同步** — 设备端时钟校准
- **可观测性** — Prometheus 指标 + JSON 结构化日志（tracing）

## 快速开始

### 前置条件

- Rust 1.75+
- EMQX 或其他 MQTT Broker
- Redis 6+
- MySQL 5.7+
- Kafka（可选，事件转发需要）

### 构建

```bash
cargo build --release
```

### 配置

复制 `.env.example` 创建配置文件：

```bash
cp .env.example .env
```

主要配置项：

| 环境变量 | 说明 | 默认值 |
|---------|------|--------|
| `TSLINK__MQTT__HOST` | MQTT Broker 地址 | `localhost` |
| `TSLINK__MQTT__PORT` | MQTT Broker 端口 | `1883` |
| `TSLINK__REDIS__URL` | Redis 连接 URL | `redis://localhost:6379` |
| `TSLINK__DATABASE__URL` | MySQL 连接 URL | — |
| `TSLINK__KAFKA__BROKERS` | Kafka Broker 地址 | `localhost:9092` |
| `TSLINK__HTTP__HOST` | HTTP API 监听地址 | `0.0.0.0` |
| `TSLINK__HTTP__PORT` | HTTP API 端口 | `8080` |

配置文件加载优先级：`config/default.toml` → `config/{RUN_ENV}.toml` → 环境变量

### 运行

```bash
# 开发模式
cargo run

# 生产模式
./target/release/tslink
```

### Docker

```bash
docker build -t tslink:latest .
docker run -p 8080:8080 -p 9090:9090 --env-file .env tslink:latest
```

### Helm 部署

```bash
cd deploy/helm
helm install tslink . -f values.yaml
```

## HTTP API

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/device/:product_key/:device_id` | 查询设备信息 |
| `POST` | `/service/invoke` | 调用设备服务 |
| `GET` | `/shadow/:product_key/:device_id` | 查询设备影子 |
| `PUT` | `/shadow/:product_key/:device_id` | 更新设备影子 |
| `GET` | `/health` | 健康检查 |
| `GET` | `/metrics` | Prometheus 指标 |

### 调用设备服务示例

```bash
curl -X POST http://localhost:8080/service/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "product_key": "pk001",
    "device_id": "did001",
    "service_id": "reboot",
    "params": {}
  }'
```

## MQTT Topic 格式

```
sys/{productKey}/{deviceId}/thing/{sub_category}/...
```

支持的消息类型：

| Topic 模式 | 消息类型 |
|-----------|---------|
| `.../thing/event/property/post` | `EventProperty` |
| `.../thing/event/{id}/{level}` | `EventCustom` |
| `.../thing/properties/state` | `PropertyState` |
| `.../thing/properties/set_reply` | `PropertySetReply` |
| `.../thing/service/{id}/post_reply` | `ServiceReply` |
| `.../thing/register/post` | `Register` |
| `.../thing/dynamic_register/post` | `DynamicRegister` |
| `.../thing/pong/post` | `Pong` |
| `.../thing/ntp/post` | `Ntp` |

支持 Region 前缀：`region/{region}/sys/...`
支持多链路后缀：`sys/{pk}/{did}_{linkSuffix}/thing/...`

## 测试

```bash
# 运行所有测试
cargo test

# 运行性能基准测试（release 模式）
cargo test --release -- routing_bench --nocapture
cargo test --release -- message_bench --nocapture

# 运行集成测试
cargo test --test integration_test
```

## 项目结构

```
tslink/
├── src/
│   ├── main.rs                 # 应用入口
│   ├── lib.rs                  # 库导出
│   ├── config.rs               # 配置管理
│   ├── error.rs                # 错误类型
│   ├── telemetry.rs            # 指标与日志
│   ├── domain/                 # 领域模型（零依赖）
│   │   ├── device.rs
│   │   ├── message.rs
│   │   ├── topic.rs
│   │   ├── thing_model.rs
│   │   ├── shadow.rs
│   │   ├── link.rs
│   │   └── event.rs
│   ├── application/            # 应用层
│   │   ├── device_service.rs
│   │   ├── thing_service.rs
│   │   ├── shadow_service.rs
│   │   ├── event_service.rs
│   │   ├── ntp_service.rs
│   │   ├── link_service.rs
│   │   └── handlers/           # 消息处理器
│   └── infrastructure/         # 基础设施层
│       ├── mqtt/               # MQTT 客户端、路由、发布
│       ├── redis/              # 设备状态、影子、链路
│       ├── database/           # MySQL 仓储
│       ├── kafka/              # 事件生产者
│       └── http/               # RESTful API
├── tests/
│   ├── bench/                  # 性能基准
│   └── integration/            # 集成测试
├── deploy/helm/                # Helm Chart
├── Dockerfile
└── .env.example
```

## License

Internal use only.
