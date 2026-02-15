# TSLink IoT Core

> 高性能边缘物联网平台 —— 连接设备、部署算法、驱动智能

TSLink 是面向边缘计算场景的物联网核心平台，使用 Rust 构建，提供多语言 SDK、高性能 MQTT 消息路由和 IPC 进程间通信能力。无论是将万级设备接入云端，还是在边缘侧部署 AI 算法实现本地闭环推理，TSLink 都提供统一的通信抽象和开箱即用的开发体验。

---

## 场景解决方案

### 场景一：边缘设备接入

将各类传感器、控制器、网关接入物联网平台，实现设备注册、属性上报、事件推送和云端服务下发。

```
┌─────────────┐     MQTT      ┌──────────────────┐     Kafka/Redis
│  传感器/终端  │ ────────────→ │   TSLink Core    │ ────────────→  云端/业务系统
│  (Go/Rust    │ ←──────────── │  (消息路由引擎)    │
│   C/C++ SDK) │   服务下发     └──────────────────┘
└─────────────┘
```

**提供四种语言 SDK，覆盖主流嵌入式与服务端场景：**

| SDK | 语言 | 适用场景 | 通信方式 | 状态 |
|-----|------|---------|---------|------|
| **tslink-rsdk** | Rust | 高性能网关、边缘服务 | MQTT + IPC | ✅ 稳定 |
| **tslink-gosdk** | Go | 云端微服务、业务集成 | MQTT | ✅ 稳定 |
| **tslink-csdk** | C | MCU、RTOS、嵌入式 Linux | MQTT | 🚧 规划中 |
| **tslink-cppsdk** | C++ | 机器人、视觉设备、工控 | MQTT + IPC | 🚧 规划中 |

**设备接入能力：**

- 设备注册与动态注册（免预配置接入）
- 属性上报 `thing_property_post` / 事件推送 `thing_event_post`
- 云端服务调用 `platform_service_invoke`（同步/异步）
- 设备影子 —— 属性缓存、离线期间指令暂存、上线自动重放
- 心跳保活 + 多链路冗余（自动链路选择与切换）
- NTP 时间同步

### 场景二：边缘算法部署

在边缘网关上部署 AI 推理算法（目标检测、异常识别等），算法进程通过 IPC 零拷贝通道与 TSLink 通信，无需经过网络协议栈，实现微秒级延迟的本地闭环控制。

```
┌────────────────────────────────────────────────────────────┐
│                      Edge Gateway                          │
│                                                            │
│  ┌─────────────┐  IPC (共享内存)  ┌──────────────────────┐ │
│  │ AI 算法进程   │ ←────────────→  │    TSLink Core       │ │
│  │ (Rust/C++)   │  零拷贝、μs延迟   │  ┌────────────────┐ │ │
│  │              │                  │  │ MQTT → 云端上报  │ │ │
│  │ · 目标检测    │  ──→ 推理结果 ──→ │  │ Redis → 状态缓存│ │ │
│  │ · 异常识别    │  ←── 控制指令 ←── │  │ Kafka → 事件流  │ │ │
│  │ · 行为分析    │                  │  └────────────────┘ │ │
│  └─────────────┘                  └──────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

**IPC 通信优势：**

- **零拷贝共享内存** —— 大帧图像/点云数据传输无序列化开销
- **进程隔离** —— 算法崩溃不影响通信主进程，独立升级部署
- **语言无关** —— Rust 和 C++ SDK 均支持 IPC channel，算法可用任意语言实现
- **统一抽象** —— SDK 的 `MessageChannel` 接口统一 MQTT 和 IPC，切换通信方式零代码变更

```rust
// Rust SDK —— MQTT 和 IPC 使用同一套 API
let client = SmartClientBuilder::new()
    .channel(IpcChannel::new("/tmp/tslink.sock"))  // 或 MqttChannel::new(config)
    .build()?;

client.thing_property_post("temperature", json!({"value": 36.5})).await?;
```

### 场景三：极致性能

TSLink Core 基于 Rust + Tokio 异步运行时构建，专为高吞吐、低延迟的 IoT 消息处理场景优化。

| 指标 | 数值 | 说明 |
|------|------|------|
| Topic 路由 | **~200ns/msg** | 13 种消息类型零分配分类 |
| 消息吞吐 | **100K+ msg/s** | 单节点 MQTT 消息处理 |
| 内存占用 | **~30MB** | 空载运行内存（vs Java ~300MB） |
| 启动时间 | **<500ms** | 冷启动到就绪 |
| 物模型缓存 | **moka, 30s TTL** | 零锁并发读，热路径零分配 |

**六边形架构** 确保核心 Domain 层零外部依赖，Infrastructure 层可替换：

```
┌──────────────────────────────────────────────────────────┐
│                     HTTP API (axum)                      │
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

---

## 快速开始

### 前置条件

- Rust 1.75+
- EMQX 或其他 MQTT Broker
- Redis 6+
- MySQL 5.7+
- Kafka（可选，事件转发需要）

### 构建 & 运行

```bash
# 构建
cargo build --release

# 配置
cp .env.example .env    # 按需修改

# 运行
cargo run               # 开发模式
./target/release/tslink # 生产模式
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

### 配置项

| 环境变量 | 说明 | 默认值 |
|---------|------|--------|
| `TSLINK__MQTT__HOST` | MQTT Broker 地址 | `localhost` |
| `TSLINK__MQTT__PORT` | MQTT Broker 端口 | `1883` |
| `TSLINK__REDIS__URL` | Redis 连接 URL | `redis://localhost:6379` |
| `TSLINK__DATABASE__URL` | MySQL 连接 URL | — |
| `TSLINK__KAFKA__BROKERS` | Kafka Broker 地址 | `localhost:9092` |
| `TSLINK__HTTP__HOST` | HTTP API 监听地址 | `0.0.0.0` |
| `TSLINK__HTTP__PORT` | HTTP API 端口 | `8080` |

配置加载优先级：`config/default.toml` → `config/{RUN_ENV}.toml` → 环境变量

---

## SDK 快速上手

### Go SDK

```bash
go get github.com/tense-i/tslink-gosdk
```

```go
client, _ := tslink.NewSmartClientBuilder().
    WithMqttChannel(tslink.DefaultMqttConfig()).
    Build()
defer client.Close()

client.Start()
client.ThingPropertyPost("temperature", map[string]any{"value": 36.5})
```

### Rust SDK

```toml
[dependencies]
tslink-rsdk = { path = "sdk/tslink-rsdk" }
```

```rust
let client = DefaultSmartClient::builder()
    .mqtt_config(MqttConfig::default())
    .build()?;

client.start().await?;
client.thing_property_post("temp", json!({"value": 36.5})).await?;
```

---

## HTTP API

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/device/:product_key/:device_id` | 查询设备信息 |
| `POST` | `/service/invoke` | 调用设备服务 |
| `GET` | `/shadow/:product_key/:device_id` | 查询设备影子 |
| `PUT` | `/shadow/:product_key/:device_id` | 更新设备影子 |
| `GET` | `/health` | 健康检查 |
| `GET` | `/metrics` | Prometheus 指标 |

```bash
curl -X POST http://localhost:8080/service/invoke \
  -H "Content-Type: application/json" \
  -d '{"product_key":"pk001","device_id":"did001","service_id":"reboot","params":{}}'
```

## MQTT Topic 格式

```
sys/{productKey}/{deviceId}/thing/{sub_category}/...
```

| Topic 模式 | 消息类型 |
|-----------|---------|
| `.../thing/event/property/post` | 属性上报 |
| `.../thing/event/{id}/{level}` | 自定义事件 |
| `.../thing/properties/set_reply` | 属性设置回复 |
| `.../thing/service/{id}/post_reply` | 服务调用回复 |
| `.../thing/register/post` | 设备注册 |
| `.../thing/dynamic_register/post` | 动态注册 |
| `.../thing/pong/post` | 心跳响应 |
| `.../thing/ntp/post` | NTP 同步 |

支持 Region 前缀：`region/{region}/sys/...`
支持多链路后缀：`sys/{pk}/{did}_{linkSuffix}/thing/...`

---

## 项目结构

```
tslink/
├── src/                          # TSLink Core（Rust 服务端）
│   ├── domain/                   #   领域模型（零外部依赖）
│   ├── application/              #   应用服务 + 消息处理器
│   └── infrastructure/           #   MQTT / Redis / MySQL / Kafka / HTTP
├── sdk/
│   ├── tslink-rsdk/              # Rust SDK（MQTT + IPC）
│   ├── tslink-gosdk/             # Go SDK（MQTT）
│   ├── tslink-csdk/              # C SDK（规划中）
│   └── tslink-cppsdk/            # C++ SDK（规划中）
├── tests/                        # 集成测试 + 性能基准
├── deploy/helm/                  # Kubernetes Helm Chart
├── docker-compose.yml            # 本地开发环境
├── Dockerfile
└── .env.example
```

## 测试

```bash
cargo test                                              # 全部测试
cargo test --release -- routing_bench --nocapture        # 路由性能基准
cargo test --release -- message_bench --nocapture        # 消息处理基准
cd sdk/tslink-gosdk && go test -race ./...              # Go SDK 测试
```

## License

MIT
