# Feature Specification: tslink-rsdk MQTT 通道

**Version**: 1.0  
**Created**: 2026-02-14  
**Status**: Draft  
**Priority**: P1 - MVP

## Background

### Context

当前 Java 版本的 JASmartSDK 已在生产环境运行，为设备端提供物联网连接能力。随着 Rust 生态系统在嵌入式和边缘计算领域的成熟，需要开发 Rust 版本的设备 SDK（tslink-rsdk），以支持资源受限设备和高性能场景。

### Problem Statement

1. Java SDK 内存占用大，不适合资源受限的边缘设备
2. 需要跨平台支持（Linux、RTOS、WebAssembly）
3. 需要可扩展的多通道架构（MQTT、IPC、HTTP）

### References

- JASmartSDK Java 实现：`@JASmartSDK/`
- tslink 服务端：`@tslink/`
- 物模型协议规范：`@doc/thing-model-protocol.md`

---

## User Scenarios & Testing

### User Story 1: 设备属性上报 (P1) 🎯 MVP

**As a** 边缘设备开发者  
**I want to** 使用 Rust SDK 上报设备属性到云端  
**So that** 云端可以实时监控设备状态

#### Acceptance Scenarios

**Scenario 1.1**: 上报单个属性
```
Given 设备已连接到 MQTT Broker
When 调用 thing_property_post({"temperature": 25.5})
Then 消息发送到 sys/{pk}/{did}/thing/event/property/post
And 消息格式符合物模型协议
```

**Scenario 1.2**: 上报多个属性
```
Given 设备已连接
When 调用 thing_property_post({"temperature": 25.5, "humidity": 60})
Then 所有属性在同一消息中发送
```

**Scenario 1.3**: 代理设备属性上报
```
Given 网关设备已连接
When 调用 thing_property_post(sub_pk, sub_did, properties)
Then 消息发送到子设备的主题
```

#### Edge Cases

- 网络断开时属性上报应返回错误
- 属性值超出范围时应记录警告但仍发送

---

### User Story 2: 设备事件上报 (P1)

**As a** 设备开发者  
**I want to** 上报设备事件（信息/警告/错误）  
**So that** 云端可以处理设备告警

#### Acceptance Scenarios

**Scenario 2.1**: 上报信息事件
```
Given 设备已连接
When 调用 thing_event_post(EventType::Info, "startup", data)
Then 消息发送到 sys/{pk}/{did}/thing/event/startup/info
```

**Scenario 2.2**: 上报带回调的事件
```
Given 设备已连接
When 调用 thing_event_post(..., callback)
Then 收到平台回复时调用 callback
```

---

### User Story 3: 接收云端服务调用 (P1)

**As a** 设备开发者  
**I want to** 注册服务处理器接收云端下发的服务调用  
**So that** 设备可以执行云端指令

#### Acceptance Scenarios

**Scenario 3.1**: 注册服务处理器
```
Given SDK 已初始化
When 调用 set_service_handle("reboot", callback)
Then 收到 reboot 服务调用时执行 callback
And 自动回复处理结果
```

**Scenario 3.2**: 属性设置回调
```
Given SDK 已初始化
When 调用 set_property_set_handle(callback)
Then 收到属性设置指令时执行 callback
```

---

### User Story 4: 多通道抽象 (P2)

**As a** SDK 架构师  
**I want to** 定义通道抽象接口  
**So that** 后续可以扩展 IPC/HTTP 通道

#### Acceptance Scenarios

**Scenario 4.1**: 通道接口定义
```
Given MessageChannel trait 已定义
When 实现 MqttChannel
Then 可以通过统一接口发送/接收消息
```

---

## Functional Requirements

### FR1: SmartClient 接口

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1.1 | 提供 `SmartClient` trait 定义核心接口 | P1 |
| FR1.2 | 提供 `DefaultSmartClient` 实现 | P1 |
| FR1.3 | 支持 Builder 模式构造客户端 | P1 |
| FR1.4 | 支持 `start()` 和 `release()` 生命周期管理 | P1 |

### FR2: 属性/事件上报

| ID | Requirement | Priority |
|----|-------------|----------|
| FR2.1 | 实现 `thing_property_post()` 上报属性 | P1 |
| FR2.2 | 实现 `thing_event_post()` 上报事件 | P1 |
| FR2.3 | 支持 EventType 枚举（Info/Warning/Error） | P1 |
| FR2.4 | 支持代理设备上报（指定 pk/did） | P1 |

### FR3: 服务调用

| ID | Requirement | Priority |
|----|-------------|----------|
| FR3.1 | 实现 `set_service_handle()` 注册服务回调 | P1 |
| FR3.2 | 实现 `set_property_set_handle()` 注册属性设置回调 | P1 |
| FR3.3 | 实现 `platform_service_invoke()` 调用平台服务 | P1 |
| FR3.4 | 支持异步回复机制（tid 关联） | P1 |

### FR4: 消息通道

| ID | Requirement | Priority |
|----|-------------|----------|
| FR4.1 | 定义 `MessageChannel` trait | P1 |
| FR4.2 | 实现 `MqttChannel` | P1 |
| FR4.3 | 支持自动重连机制 | P1 |
| FR4.4 | 支持多主题订阅 | P1 |
| FR4.5 | 预留 IPC/HTTP 通道接口 | P2 |

### FR5: 消息格式

| ID | Requirement | Priority |
|----|-------------|----------|
| FR5.1 | 实现 `CommonMessage` 消息结构 | P1 |
| FR5.2 | 实现 `ReplyMessage` 回复结构 | P1 |
| FR5.3 | 支持 JSON 序列化/反序列化 | P1 |

---

## Key Entities

### SmartClient

核心客户端接口，提供设备与云端交互的所有方法。

### MessageChannel

消息通道抽象，定义 send/start/stop 方法和消息接收回调。

### MqttChannel

MQTT 协议实现的消息通道，处理连接、订阅、发布。

### CommonMessage

标准消息格式，包含 tid/bid/version/timestamp/method/data。

### EventType

事件类型枚举：Info、Warning、Error。

---

## Success Criteria

| Criterion | Metric | Target |
|-----------|--------|--------|
| SC1 | 属性上报成功率 | ≥99.9% |
| SC2 | 消息延迟 | p99 < 100ms |
| SC3 | 重连恢复时间 | < 5s |
| SC4 | 内存占用 | < 5MB |
| SC5 | 代码测试覆盖率 | ≥80% |

---

## Out of Scope

- IPC 通道实现（P2，后续迭代）
- HTTP 通道实现（P2，后续迭代）
- 设备注册/动态注册（使用 tslink 现有能力）
- OTA 升级功能
- 离线消息缓存
