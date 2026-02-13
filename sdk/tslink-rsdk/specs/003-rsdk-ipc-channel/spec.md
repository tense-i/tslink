# SPEC-003: tslink-rsdk IPC Channel & Multi-Channel Support

## Meta

| Field | Value |
|-------|-------|
| Spec ID | SPEC-003 |
| Title | tslink-rsdk IPC Channel & Multi-Channel Support |
| Status | In Progress |
| Created | 2025-02-14 |
| Updated | 2025-02-14 |
| Author | AI Assistant |

## Problem Statement

设备端 SDK 需要支持同机进程间通信（IPC），用于边缘计算场景下多个进程间的高效数据交换。参考 ja-IOT-SDK-cpp 的 eCAL 实现，使用 Rust 原生的 iceoryx2 实现零拷贝、低延迟的 IPC 通道。

**关键需求**: 参照 ja-IOT-SDK-cpp 接口，发布和订阅时可以选择具体通道（IPC、MQTT、ALL），ALL 表示同时发布到两个通道。当本地没有网络时，只影响 MQTT 通道而不影响 IPC。

## Goals

1. **零拷贝 IPC**: 使用 iceoryx2 实现真正的零拷贝进程间通信
2. **多通道支持**: 支持 IPC、MQTT、ALL 通道选择
3. **通道隔离**: 网络故障只影响 MQTT，不影响 IPC
4. **统一接口**: 实现 `MessageChannel` trait，与 MQTT 通道保持接口一致
5. **设备发现**: 支持同机设备自动发现和状态监控
6. **高性能**: 适用于大数据量传输（图像/点云/音视频帧）

## Non-Goals

- 跨机器通信（由 MQTT 通道负责）
- 持久化存储
- 消息队列功能

---

## User Stories

### US1: 作为边缘计算应用，我需要通过 IPC 发送/接收消息

**验收标准:**
- [ ] 可以创建 IpcChannel 并连接到本地 IPC 服务
- [ ] 可以发布消息到指定 topic
- [ ] 可以订阅 topic 并接收消息
- [ ] 支持零拷贝数据传输

### US2: 作为边缘网关，我需要发现同机的其他设备进程

**验收标准:**
- [ ] 自动广播设备存在信息
- [ ] 可以接收其他设备的发现消息
- [ ] 设备离线时自动清理过期记录

### US3: 作为应用开发者，我需要 IPC 和 MQTT 通道使用相同接口

**验收标准:**
- [ ] IpcChannel 实现 MessageChannel trait
- [ ] 可以通过 TslinkClientBuilder 选择 IPC 或 MQTT 通道
- [ ] 上层业务代码无需关心底层通道类型

---

## Functional Requirements

### FR1: IpcChannel 核心功能

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1.1 | 实现 `IpcChannel` struct | P1 |
| FR1.2 | 实现 `MessageChannel` trait | P1 |
| FR1.3 | 使用 iceoryx2 pub/sub 模式 | P1 |
| FR1.4 | 支持 topic 订阅和回调 | P1 |
| FR1.5 | 支持消息发布 | P1 |

### FR2: 设备发现

| ID | Requirement | Priority |
|----|-------------|----------|
| FR2.1 | 周期性广播设备信息 | P2 |
| FR2.2 | 接收其他设备发现消息 | P2 |
| FR2.3 | 维护设备缓存（带过期时间） | P2 |
| FR2.4 | 设备状态回调通知 | P2 |

### FR3: 配置与生命周期

| ID | Requirement | Priority |
|----|-------------|----------|
| FR3.1 | IpcConfig 配置结构 | P1 |
| FR3.2 | start() 启动 IPC 通道 | P1 |
| FR3.3 | stop() 停止 IPC 通道 | P1 |
| FR3.4 | 优雅关闭和资源释放 | P1 |

---

## Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR1 | 消息延迟 | < 1ms (同机) |
| NFR2 | 零拷贝传输 | 大于 4KB 的数据使用零拷贝 |
| NFR3 | 内存占用 | 共享内存池可配置 |

---

## Key Entities

### CommunicationChannel (通道枚举)

参考 ja-IOT-SDK-cpp 的 `CommunicationChannel` 枚举。

```rust
/// 数据通信通道
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommunicationChannel {
    /// 全通道 - 同时发送到 MQTT 和 IPC
    #[default]
    All,
    /// 远程通道 - 仅 MQTT
    Remote,
    /// IPC 通道 - 仅本地进程间通信
    Ipc,
}
```

### MultiChannel (多通道管理器)

统一管理 MQTT 和 IPC 通道，根据指定通道类型路由消息。

```rust
pub struct MultiChannel {
    mqtt_channel: Option<Arc<MqttChannel>>,
    ipc_channel: Option<Arc<IpcChannel>>,
    default_channel: CommunicationChannel,
}

impl MultiChannel {
    /// 使用指定通道发送消息
    pub async fn send_with_channel(
        &self, 
        topic: &str, 
        message: &str, 
        channel: CommunicationChannel
    ) -> Result<()>;
    
    /// 使用指定通道订阅
    pub async fn subscribe_with_channel(
        &self, 
        topic: &str, 
        channel: CommunicationChannel
    ) -> Result<()>;
}
```

### IpcChannel

IPC 通道实现，基于 iceoryx2 的 pub/sub 模式。

```rust
pub struct IpcChannel {
    config: IpcConfig,
    callback: Arc<RwLock<Option<Arc<dyn MessageReceiveCallback>>>>,
    is_running: Arc<AtomicBool>,
    subscribers: Arc<RwLock<HashMap<String, SubscriberEntry>>>,
    node_name: String,
}
```

### IpcConfig

IPC 通道配置。

```rust
pub struct IpcConfig {
    pub product_key: String,
    pub device_id: String,
    pub discovery_interval_secs: u64,
    pub device_cache_expire_secs: u64,
    pub service_prefix: String,
}
```

---

## API Design

### TslinkClient 扩展 (多通道支持)

```rust
#[async_trait]
pub trait TslinkClient: Send + Sync {
    /// 使用指定通道上报属性
    async fn thing_property_post_with_channel(
        &self,
        properties: Value,
        channel: CommunicationChannel,
    ) -> Result<()>;

    /// 使用指定通道上报事件
    async fn thing_event_post_with_channel(
        &self,
        identifier: &str,
        event_type: EventType,
        data: Value,
        channel: CommunicationChannel,
    ) -> Result<()>;

    // ... 其他方法类似
}
```

### IpcChannel Methods

```rust
impl IpcChannel {
    pub fn new(config: IpcConfig) -> Result<Self>;
    pub fn set_callback(&self, callback: Arc<dyn MessageReceiveCallback>);
}

impl MessageChannel for IpcChannel {
    async fn send(&self, topic: &str, message: &str) -> Result<()>;
    async fn add_topic(&self, topic: &str) -> Result<()>;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    fn is_connected(&self) -> bool;
}
```

---

## Dependencies

- `iceoryx2`: 零拷贝 IPC 中间件
- `tokio`: 异步运行时
- `serde`/`serde_json`: 消息序列化

---

## Success Criteria

1. [ ] IpcChannel 通过单元测试
2. [ ] 两个进程可以通过 IPC 交换消息
3. [ ] 设备发现功能正常工作
4. [ ] 与 MqttChannel 接口兼容
5. [ ] 零拷贝传输验证

---

## Open Questions

1. iceoryx2 的共享内存配置如何与系统资源协调？
2. 是否需要支持 request/response 模式？
3. 设备发现 topic 命名规范？
