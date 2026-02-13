# PLAN: SPEC-003 tslink-rsdk IPC Channel

## Summary

基于 iceoryx2 实现 tslink-rsdk 的 IPC 通道，提供同机进程间零拷贝通信能力。

核心组件：
1. **IpcChannel** - IPC 通道实现，基于 iceoryx2 pub/sub
2. **IpcConfig** - IPC 配置结构
3. **DeviceDiscovery** - 设备发现服务

## Technical Context

### iceoryx2 核心概念

```rust
// Node: 通信节点
let node = NodeBuilder::new().create::<ipc::Service>()?;

// Service: 服务定义（类似 topic）
let service = node.service_builder(&"topic/name".try_into()?)
    .publish_subscribe::<PayloadType>()
    .open_or_create()?;

// Publisher/Subscriber: 发布者/订阅者
let publisher = service.publisher_builder().create()?;
let subscriber = service.subscriber_builder().create()?;

// 零拷贝发送
let sample = publisher.loan_uninit()?;
let sample = sample.write_payload(data);
sample.send()?;

// 接收
while let Some(sample) = subscriber.receive()? {
    println!("{:?}", *sample);
}
```

### 与 ja-IOT-SDK-cpp 对比

| 功能 | ja-IOT-SDK-cpp (eCAL) | tslink-rsdk (iceoryx2) |
|------|----------------------|------------------------|
| IPC 库 | eCAL | iceoryx2 |
| 语言 | C++ | Rust |
| 零拷贝 | 支持 | 支持（原生） |
| pub/sub | 支持 | 支持 |
| 设备发现 | 自定义实现 | 自定义实现 |

---

## Project Structure

```
tslink-rsdk/src/
├── channel/
│   ├── mod.rs              # 更新: 导出 IpcChannel
│   ├── message_channel.rs  # 已有: MessageChannel trait
│   ├── mqtt_channel.rs     # 已有: MQTT 实现
│   └── ipc_channel.rs      # 新增: IPC 实现
├── discovery/
│   ├── mod.rs              # 新增: 设备发现模块
│   └── device_discovery.rs # 新增: 设备发现服务
└── ...
```

---

## Data Models

### IpcConfig

```rust
#[derive(Debug, Clone)]
pub struct IpcConfig {
    /// 产品标识
    pub product_key: String,
    /// 设备标识
    pub device_id: String,
    /// 设备发现广播间隔（秒）
    pub discovery_interval_secs: u64,
    /// 设备缓存过期时间（秒）
    pub device_cache_expire_secs: u64,
    /// 服务名前缀
    pub service_prefix: String,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            product_key: String::new(),
            device_id: String::new(),
            discovery_interval_secs: 5,
            device_cache_expire_secs: 30,
            service_prefix: "tslink".to_string(),
        }
    }
}
```

### IpcMessage (共享内存消息)

```rust
/// IPC 消息载荷（固定大小用于零拷贝）
#[repr(C)]
pub struct IpcPayload {
    /// 消息长度
    pub len: u32,
    /// 消息内容（最大 64KB）
    pub data: [u8; 65536],
}
```

---

## API Contracts

### IpcChannel

```rust
pub struct IpcChannel {
    config: IpcConfig,
    node: Node<ipc::Service>,
    publishers: Arc<RwLock<HashMap<String, Publisher<...>>>>,
    subscribers: Arc<RwLock<HashMap<String, Subscriber<...>>>>,
    callback: Arc<RwLock<Option<Arc<dyn MessageReceiveCallback>>>>,
    is_running: Arc<AtomicBool>,
    discovery: Option<DeviceDiscovery>,
}

impl IpcChannel {
    pub fn new(config: IpcConfig) -> Result<Self>;
}

#[async_trait]
impl MessageChannel for IpcChannel {
    async fn send(&self, topic: &str, message: &str) -> Result<()>;
    async fn subscribe(&self, topic: &str) -> Result<()>;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    fn set_callback(&self, callback: Arc<dyn MessageReceiveCallback>);
}
```

### DeviceDiscovery

```rust
pub struct DeviceDiscovery {
    product_key: String,
    device_id: String,
    interval: Duration,
    expire_time: Duration,
    device_cache: Arc<RwLock<HashMap<String, DeviceCacheEntry>>>,
    status_callback: Arc<RwLock<Option<DeviceStatusCallback>>>,
}

impl DeviceDiscovery {
    pub fn new(config: &IpcConfig) -> Self;
    pub async fn start(&self, channel: Arc<IpcChannel>) -> Result<()>;
    pub async fn stop(&self);
    pub fn set_status_callback(&self, callback: DeviceStatusCallback);
    pub fn get_online_devices(&self) -> Vec<String>;
}
```

---

## Implementation Phases

### Phase 1: 核心 IPC 通道 (P1)
- IpcConfig 配置
- IpcChannel 基本结构
- MessageChannel trait 实现
- pub/sub 基本功能

### Phase 2: 设备发现 (P2)
- DeviceDiscovery 服务
- 设备广播和接收
- 设备缓存管理
- 状态回调通知

### Phase 3: 集成测试 (P1)
- 单元测试
- 双进程通信测试
- 与 TslinkClient 集成

---

## Dependencies

```toml
[dependencies]
iceoryx2 = "0.8"

[features]
ipc = ["iceoryx2"]
```

---

## Risk Assessment

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| iceoryx2 共享内存权限 | 高 | 文档说明配置要求 |
| 消息大小限制 | 中 | 实现分片传输 |
| 跨平台兼容性 | 中 | 仅支持 Linux/macOS |

---

## Complexity Tracking

| 组件 | 预估复杂度 | 说明 |
|------|-----------|------|
| IpcChannel | 中 | iceoryx2 API 学习曲线 |
| DeviceDiscovery | 低 | 参考 C++ 实现 |
| 集成测试 | 中 | 需要多进程测试 |
