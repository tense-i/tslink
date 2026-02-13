# Implementation Plan: tslink-rsdk MQTT 通道

**Branch**: `002-rsdk-mqtt-channel` | **Date**: 2026-02-14 | **Spec**: [spec.md](./spec.md)  
**Input**: Feature specification from `/tslink-rsdk/specs/002-rsdk-mqtt-channel/spec.md`

## Summary

实现 tslink-rsdk 物联 SDK 的 MQTT 通道，参考 JASmartSDK Java 实现，提供：
1. **SmartClient** - 核心客户端接口和默认实现
2. **MessageChannel** - 消息通道抽象（预留 IPC/HTTP 扩展）
3. **MqttChannel** - MQTT 协议实现

## Technical Context

**Language/Version**: Rust 1.75+ (edition 2021)  
**Primary Dependencies**: rumqttc (MQTT), tokio (async), serde (JSON)  
**Storage**: N/A (无状态 SDK)  
**Testing**: cargo test (unit + integration)  
**Target Platform**: Linux, RTOS, WebAssembly (cross-compile)  
**Project Type**: Library crate  
**Performance Goals**: p99 < 100ms, < 5MB memory  
**Constraints**: no_std 兼容（未来），最小依赖

## Constitution Check

*GATE: ✅ 通过*

- ✅ 独立 crate，不依赖 tslink 服务端代码
- ✅ 使用 trait 抽象，支持多通道扩展
- ✅ 异步 API，兼容 tokio runtime
- ✅ 与 JASmartSDK API 保持一致

## Project Structure

### Documentation (this feature)

```text
tslink-rsdk/specs/002-rsdk-mqtt-channel/
├── spec.md              # 需求规格 ✅
├── plan.md              # 本文件 ✅
└── tasks.md             # 任务清单
```

### Source Code (tslink-rsdk)

```text
tslink-rsdk/
├── Cargo.toml               # 新增: crate 配置
├── src/
│   ├── lib.rs               # 新增: crate 入口
│   ├── client/
│   │   ├── mod.rs           # 新增: client 模块
│   │   ├── smart_client.rs  # 新增: SmartClient trait
│   │   ├── default_client.rs # 新增: DefaultSmartClient
│   │   └── builder.rs       # 新增: ClientBuilder
│   ├── channel/
│   │   ├── mod.rs           # 新增: channel 模块
│   │   ├── message_channel.rs # 新增: MessageChannel trait
│   │   └── mqtt_channel.rs  # 新增: MqttChannel 实现
│   ├── message/
│   │   ├── mod.rs           # 新增: message 模块
│   │   ├── common.rs        # 新增: CommonMessage
│   │   └── reply.rs         # 新增: ReplyMessage
│   ├── adapter/
│   │   ├── mod.rs           # 新增: adapter 模块
│   │   └── message_adapter.rs # 新增: MessageReceiveAdapter
│   ├── enums.rs             # 新增: EventType 等枚举
│   └── error.rs             # 新增: SDK 错误类型
└── tests/
    └── integration_test.rs  # 新增: 集成测试
```

**Structure Decision**: 独立 crate，模块化设计，支持后续扩展 IPC/HTTP 通道

## Data Model

### CommonMessage

```rust
pub struct CommonMessage {
    pub tid: String,        // 事务 ID
    pub bid: String,        // 批次 ID  
    pub version: String,    // 协议版本
    pub timestamp: i64,     // 时间戳
    pub method: String,     // 方法名
    pub data: Value,        // 数据载荷
}
```

### ReplyMessage

```rust
pub struct ReplyMessage {
    pub tid: String,
    pub bid: String,
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}
```

## API Contracts

### SmartClient Trait

```rust
#[async_trait]
pub trait SmartClient: Send + Sync {
    async fn thing_property_post(&self, data: Value) -> Result<()>;
    async fn thing_property_post_for(&self, pk: &str, did: &str, data: Value) -> Result<()>;
    async fn thing_event_post(&self, event_type: EventType, name: &str, data: Value) -> Result<()>;
    async fn thing_event_post_with_reply(&self, ..., callback: ServiceReplyCallback) -> Result<()>;
    fn set_service_handle(&self, identity: &str, callback: ServiceCallback);
    fn set_property_set_handle(&self, callback: ServiceCallback);
    async fn platform_service_invoke(&self, ..., callback: ServiceReplyCallback) -> Result<()>;
    async fn start(&self) -> Result<()>;
    async fn release(&self) -> Result<()>;
}
```

### MessageChannel Trait

```rust
#[async_trait]
pub trait MessageChannel: Send + Sync {
    async fn send(&self, topic: &str, data: &str) -> Result<()>;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}

pub trait MessageReceiveCallback: Send + Sync {
    fn receive(&self, topic: &str, data: &str);
}
```

## Complexity Tracking

*无违规，无需记录*

## Reference Implementation

参考 JASmartSDK Java 实现：

| Java 类 | Rust 对应 |
|---------|-----------|
| `JASmartClient` | `SmartClient` trait |
| `DefaultJASmartClient` | `DefaultSmartClient` |
| `MessageChannel` | `MessageChannel` trait |
| `MQTTChannel` | `MqttChannel` |
| `CommonMessage` | `CommonMessage` |
| `MessageReceiveAdapter` | `MessageAdapter` |
