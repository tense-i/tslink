# Feature Specification: TSLink IoT Core

**Feature Branch**: `1-tslink-iot-core`  
**Created**: 2026-02-13  
**Status**: Draft  
**Input**: 使用 Rust 重写 Java 控制中心的 IoT 核心模块，仅包含设备通信基础设施，剥离所有业务逻辑

## User Scenarios & Testing *(mandatory)*

### User Story 1 - MQTT 连接与消息收发 (Priority: P0)

设备通过 MQTT 协议连接到 TSLink，TSLink 管理连接生命周期（连接、断线重连、心跳），并能可靠地收发消息。支持按 QoS 0/1 发布和订阅 V3 topic 体系（`sys/{productKey}/{deviceId}/...`）。

**Why this priority**: MQTT 通信是整个 IoT 平台的基础，所有其他功能（设备管理、物模型、影子）都依赖消息通道的可用性。

**Independent Test**: 启动 TSLink 连接到 EMQX broker，订阅 `sys/+/+/thing/#` 通配 topic，通过外部 MQTT 客户端发送消息，验证 TSLink 能正确接收、反序列化并路由。

**Acceptance Scenarios**:

1. **Given** TSLink 启动并配置了 EMQX 地址, **When** 启动完成, **Then** 成功连接到 EMQX 并订阅配置的 17 个 inbound topic
2. **Given** 已连接到 EMQX, **When** 设备发布消息到 `sys/{pk}/{did}/thing/event/property/post`, **Then** TSLink 在 100ms 内接收并反序列化为 `CommonTopicReceiver` 结构体
3. **Given** 已连接到 EMQX, **When** 网络中断后恢复, **Then** TSLink 自动重连并重新订阅所有 topic，无消息丢失
4. **Given** 已连接到 EMQX, **When** 调用 publish 接口, **Then** 消息被发布到指定 topic，QoS 级别正确

---

### User Story 2 - Topic 路由与消息分发 (Priority: P0)

TSLink 根据 MQTT topic 模式将收到的消息路由到对应的处理器。支持三级路由：一级路由（thing/platform/app），二级路由（event/service/property/register），三级路由（具体 method/identifier）。支持级联消息前缀剥离（`region/{regionKey}/sys/...` → `sys/...`）。

**Why this priority**: Topic 路由是消息处理的核心调度机制，决定了每条消息由哪个处理器负责。

**Independent Test**: 构造不同 topic 的 MQTT 消息，验证每条消息被路由到正确的处理通道，不匹配的 topic 被记录并丢弃。

**Acceptance Scenarios**:

1. **Given** 消息到达 topic `sys/pk1/did1/thing/event/property/post`, **When** 路由引擎处理, **Then** 消息被分发到 DevicePropertyHandler
2. **Given** 消息到达 topic `sys/pk1/did1/thing/service/live/post_reply`, **When** 路由引擎处理, **Then** 消息被分发到 ServiceReplyHandler 并匹配到 pending 请求
3. **Given** 消息到达 topic `sys/pk1/did1/platform/service/media_server/post`, **When** 路由引擎处理, **Then** 消息被分发到 PlatformServiceHandler
4. **Given** 消息到达 topic `region/r1/sys/pk1/did1/thing/event/property/post`, **When** 路由引擎处理, **Then** 级联前缀被剥离，按 `sys/pk1/did1/thing/event/property/post` 正常路由
5. **Given** 消息到达未注册的 topic, **When** 路由引擎处理, **Then** 记录 warn 日志并丢弃消息

---

### User Story 3 - 设备生命周期管理 (Priority: P0)

TSLink 管理设备的上线/离线状态。当设备连接到 EMQX 时，通过 `$SYS` 系统 topic 或心跳机制感知设备上线，更新 Redis 状态为 ONLINE；设备断线时更新为 OFFLINE。支持设备动态注册（通过 MQTT 上报 productKey + deviceSecret）。

**Why this priority**: 设备在线状态是 IoT 平台的基础数据，影子回放、服务调用都依赖设备状态判断。

**Independent Test**: 模拟设备连接/断线事件，验证 Redis 中设备状态正确更新，动态注册流程生成设备记录。

**Acceptance Scenarios**:

1. **Given** 设备通过 EMQX 连接, **When** 收到 `$SYS/brokers/+/clients/{clientId}/connected` 消息, **Then** 设备状态更新为 ONLINE，记录上线时间
2. **Given** 设备已在线, **When** 收到 `$SYS/brokers/+/clients/{clientId}/disconnected` 消息, **Then** 设备状态更新为 OFFLINE
3. **Given** 收到心跳 `sys/{pk}/{did}/thing/pong/post`, **When** 设备已注册, **Then** 刷新设备最后活跃时间
4. **Given** 设备发送 `sys/{pk}/{did}/thing/register/post` 携带 productKey 和 deviceSecret, **When** 密钥验证通过, **Then** 创建设备记录并回复注册成功（含 deviceId 和服务配置）
5. **Given** 设备发送动态注册 `sys/{pk}/{did}/thing/dynamic_register/post` 携带 SN, **When** SN 在预注册列表中, **Then** 绑定设备并回复三元组

---

### User Story 4 - 设备影子（Device Shadow）(Priority: P1)

TSLink 在 Redis 中维护每个设备的属性影子（最新属性快照）。设备上报属性时更新影子；平台查询属性时从影子读取；设备上线时根据影子配置自动重放服务调用（如恢复推流）。

**Why this priority**: 影子机制解耦了设备和平台的在线时差问题，是设备控制的核心状态管理。

**Independent Test**: 模拟设备属性上报，验证 Redis 影子更新；模拟设备上线，验证影子服务自动重放。

**Acceptance Scenarios**:

1. **Given** 设备上报属性到 `sys/{pk}/{did}/thing/event/property/post`, **When** 属性解析成功, **Then** Redis key `IOT_DEVICE_PROPERTIES_KEY_{pk}_{did}` 被合并更新
2. **Given** 平台查询设备属性, **When** 调用影子读取接口, **Then** 返回 Redis 中完整的属性快照 JSON
3. **Given** 设备上线事件触发, **When** 该设备有配置的影子服务（如 live 推流）, **Then** 自动向设备下发影子中记录的服务调用
4. **Given** 平台设置设备属性, **When** 设备离线, **Then** 属性保存在影子中，设备下次上线时同步

---

### User Story 5 - 物模型服务调用（Thing Model Service Invoke）(Priority: P1)

TSLink 支持通过 MQTT 向设备发起服务调用（`sys/{pk}/{did}/thing/service/{method}/post`），支持同步等待回复（`post_reply`）和异步调用。同步调用通过 tid 匹配请求-响应对，超时可配置。

**Why this priority**: 服务调用是平台控制设备的核心手段，如推流、拍照、云台控制等。

**Independent Test**: 通过 HTTP API 发起服务调用，验证 MQTT 消息正确发布，设备回复后同步返回结果。

**Acceptance Scenarios**:

1. **Given** 调用 service(pk, did, "live", data), **When** 设备在线, **Then** 发布消息到 `sys/{pk}/{did}/thing/service/live/post` 并等待 `post_reply`
2. **Given** 同步调用已发出, **When** 设备在超时时间内回复 `post_reply` 且 tid 匹配, **Then** 返回设备响应数据
3. **Given** 同步调用已发出, **When** 超时未收到回复, **Then** 返回超时错误
4. **Given** 调用 asyncService(pk, did, "takePicture", data), **When** 设备在线, **Then** 消息发布后立即返回，不等待回复

---

### User Story 6 - 物模型事件处理（Thing Model Event）(Priority: P1)

TSLink 接收设备上报的事件（`sys/{pk}/{did}/thing/event/{identifier}/{level}`），按 info/warning/error 三个级别分类处理，回复 ACK 确认，并将事件转发到 Kafka topic 供下游消费。

**Why this priority**: 事件是设备主动上报异常/状态的通道，转发到 Kafka 实现与业务系统的解耦。

**Independent Test**: 模拟设备上报事件消息，验证 ACK 回复、Kafka 消息发送。

**Acceptance Scenarios**:

1. **Given** 设备上报 `sys/{pk}/{did}/thing/event/lowBattery/warning`, **When** 消息到达, **Then** 解析事件并回复 `warning_reply` ACK
2. **Given** 事件处理完成, **When** Kafka producer 可用, **Then** 事件消息被发送到 `iot-device-message` topic
3. **Given** 事件处理完成, **When** Kafka 不可用, **Then** 事件被记录到本地日志，不阻塞 MQTT 处理

---

### User Story 7 - 多链路设备通信 (Priority: P2)

TSLink 支持同一设备通过多条链路（如 4G + 卫星）同时连接。通过 Redis 滑动窗口统计各链路 QPS，自动选择最优链路或由用户手动指定主链路。服务调用可配置走指定链路、主链路或全链路广播。

**Why this priority**: 多链路是该平台的差异化能力，但复杂度较高，可在核心功能稳定后实现。

**Independent Test**: 模拟同一设备通过两条链路发送消息，验证链路权重计算、主链路自动切换、服务调用路由。

**Acceptance Scenarios**:

1. **Given** 设备 did 通过 linkId=A 和 linkId=B 同时在线, **When** 链路 A 的 QPS 高于 B, **Then** 链路 A 的权重分数高于 B
2. **Given** 多条链路在线, **When** 下发服务调用, **Then** 消息发送到权重最高的链路对应 topic
3. **Given** 用户调用 setActiveLink(pk, did, linkId=B), **When** 下发服务调用, **Then** 强制走链路 B
4. **Given** 方法配置为 ALL 广播, **When** 下发服务调用, **Then** 消息发送到所有活跃链路

---

### User Story 8 - HTTP 管理 API (Priority: P2)

TSLink 提供 RESTful HTTP API 用于外部系统查询设备状态、调用设备服务、管理设备影子。API 用于替代原 Java 控制中心的 Controller 层。

**Why this priority**: HTTP API 是外部系统集成的入口，但不影响核心 MQTT 消息处理。

**Independent Test**: 通过 curl/httpie 调用 API，验证设备列表、服务调用、影子查询等接口正确返回。

**Acceptance Scenarios**:

1. **Given** TSLink 运行中, **When** GET /api/v1/devices/{pk}/{did}, **Then** 返回设备详情含在线状态
2. **Given** 设备在线, **When** POST /api/v1/devices/{pk}/{did}/services/{method}, **Then** 同步调用设备服务并返回结果
3. **Given** TSLink 运行中, **When** GET /api/v1/devices/{pk}/{did}/shadow, **Then** 返回设备影子属性
4. **Given** TSLink 运行中, **When** GET /health, **Then** 返回健康状态和 metrics

---

### Edge Cases

- 设备 clientId 格式异常时如何处理上线/离线事件？→ 解析失败记录 warn 日志，丢弃事件
- EMQX 集群节点切换导致的重复 connected/disconnected 事件？→ 幂等处理，通过 Redis 原子操作确保状态一致
- 超大 JSON payload（>1MB）的物模型消息？→ 配置消息最大字节数，超限拒绝并记录错误
- Redis 不可用时的降级策略？→ 设备状态降级为内存缓存（LRU），影子读取返回 503，不阻塞 MQTT 消息处理
- MQTT 消息乱序到达（如先收到 reply 再收到 request 超时通知）？→ tid 匹配使用 DashMap + 过期淘汰

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 系统 MUST 连接 EMQX broker 并维持长连接，支持自动重连（指数退避，最大间隔 30 秒）
- **FR-002**: 系统 MUST 订阅 V3 topic 体系的 17 个 inbound topic（`sys/+/+/thing/*`, `sys/+/+/platform/*`, `sys/+/+/app/*`）
- **FR-003**: 系统 MUST 将 MQTT 消息反序列化为 `CommonTopicReceiver<T>` 强类型结构
- **FR-004**: 系统 MUST 通过正则匹配实现三级 topic 路由（MessageType → DeviceMessage/PlatformMessage/AppMessage → 具体 handler）
- **FR-005**: 系统 MUST 管理设备上线/离线状态，通过 `$SYS` topic 或心跳感知，存储到 Redis
- **FR-006**: 系统 MUST 支持设备动态注册，验证 productKey + deviceSecret 后创建设备记录
- **FR-007**: 系统 MUST 在 Redis 中维护设备属性影子，支持合并更新和全量读取
- **FR-008**: 系统 MUST 支持同步服务调用（request-reply 模式，基于 tid 匹配，默认 10 秒超时）
- **FR-009**: 系统 MUST 支持异步服务调用（fire-and-forget 模式）
- **FR-010**: 系统 MUST 将设备事件转发到 Kafka `iot-device-message` topic
- **FR-011**: 系统 MUST 回复设备事件的 ACK（`info_reply`/`warning_reply`/`error_reply`）
- **FR-012**: 系统 MUST 支持多链路设备的链路权重计算和主链路选择
- **FR-013**: 系统 MUST 提供 Prometheus metrics endpoint（`/metrics`）
- **FR-014**: 系统 MUST 提供 RESTful HTTP API 用于设备查询、服务调用、影子管理
- **FR-015**: 系统 MUST 支持设备上线时自动重放影子服务调用
- **FR-016**: 系统 MUST 从 topic 中解析 productKey 和 deviceId（格式 `sys/{pk}/{did_linkSuffix}/...`）
- **FR-017**: 系统 MUST 支持 NTP 时间同步请求处理（`thing/ntp/post`）
- **FR-018**: 系统 MUST 支持子设备拓扑更新（`thing/update_topo/post`）
- **FR-019**: 系统 MUST 支持级联消息处理（剥离 `region/{regionKey}/` 前缀后按标准流程路由）
- **FR-020**: 系统 MUST 支持产品级广播消息（`sys/{pk}/broadcast/platform/service/{method}/push`）

### Key Entities

- **Device**: productKey, deviceId, deviceName, deviceSecret, deviceStatus(ONLINE/OFFLINE/FAULT/NOT_ACTIVE), parentProductKey, parentId, gmtLastOnline
- **DeviceModel (物模型)**: productKey, properties[], services{}, events{}, configs[]
- **FunctionMethod (服务/事件定义)**: identifier, method, callType(SYNC/ASYNC), inputFields[], outputFields[]
- **DeviceShadow**: productKey, deviceId, properties(JSON), shadowServices[]
- **Link**: linkId, productKey, deviceId, isActive, weight, lastMessageTime
- **CommonTopicReceiver**: tid, bid, version, timestamp, method, productKey, deviceId, data, code, message
- **CommonTopicResponse**: tid, bid, version, timestamp, method, data, code, message, productKey, deviceId

### Success Criteria

- 支持 10,000 台设备同时在线，内存占用低于 200MB
- MQTT 消息处理 P99 延迟低于 100ms
- 启动时间低于 5 秒
- 现有 JASmartSDK 设备无需任何修改即可接入
- 设备上线/离线状态更新延迟低于 1 秒
- 同步服务调用端到端延迟（不含设备处理时间）低于 50ms

### Assumptions

- EMQX broker 已部署且稳定运行，TSLink 仅作为客户端连接
- Redis 6.x+ 已部署，支持 Lua 脚本执行
- Kafka 已部署，topic `iot-device-message` 已创建
- 数据库使用 MySQL 8.x，设备表 `iot_device` 等已存在
- 现有设备使用 V3 topic 体系为主，V2 兼容层为可选