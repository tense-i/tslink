# SPEC-005 Plan: tslink-gosdk

## 架构设计

```
┌─────────────────────────────────┐
│      TslinkClient (interface)   │
├─────────────────────────────────┤
│    DefaultTslinkClient          │  ← 默认实现
│    TslinkClientBuilder          │  ← 链式构建器
├─────────────────────────────────┤
│    MessageAdapter               │  ← 消息路由 + 回调分发
├─────────────────────────────────┤
│    MqttChannel                  │  ← MQTT 传输 (paho.mqtt.golang)
│    IpcChannel (placeholder)     │  ← IPC 占位
│    MultiChannel                 │  ← 多通道路由
├─────────────────────────────────┤
│    DeviceDiscovery              │  ← 设备发现 (表管理)
└─────────────────────────────────┘
```

## 文件结构

| 文件 | 职责 |
|---|---|
| enums.go | EventType, QoS, CommunicationChannel |
| error.go | Error 类型 + 哨兵错误 |
| message.go | CommonMessage + Builder |
| reply.go | ReplyMessage |
| service.go | 服务请求/响应类型 + 回调函数类型 |
| channel.go | MessageChannel 接口 |
| mqtt_channel.go | MQTT 传输实现 |
| ipc_channel.go | IPC 占位 |
| multi_channel.go | 多通道路由 |
| adapter.go | MessageAdapter 消息分发 |
| discovery.go | DeviceDiscovery |
| client.go | TslinkClient 接口 |
| default_client.go | DefaultTslinkClient |
| builder.go | TslinkClientBuilder |

## Rust → Go 映射

| Rust 概念 | Go 对应 |
|---|---|
| `async_trait` | `context.Context` 参数 |
| `Arc<dyn Fn(...)>` | Go function types |
| `Result<T>` | `(T, error)` |
| `parking_lot::RwLock` | `sync.RWMutex` |
| `tokio::oneshot` | `chan` |
| `rumqttc` | `paho.mqtt.golang` |
| `serde_json::Value` | `json.RawMessage` / `interface{}` |
| Builder pattern | Method chaining |
