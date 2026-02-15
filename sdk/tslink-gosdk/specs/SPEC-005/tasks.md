# SPEC-005 Tasks: tslink-gosdk

## Phase 1: 基础类型 (T01-T04)

| ID | 任务 | 优先级 | 状态 |
|---|---|---|---|
| T01 | 新增：Go 项目初始化 (go.mod + 依赖) | P1-high | Done |
| T02 | 新增：枚举类型 (EventType, QoS, CommunicationChannel) | P1-high | Done |
| T03 | 新增：错误类型体系 | P1-high | Done |
| T04 | 新增：消息类型 (CommonMessage, ReplyMessage, Service 类型) | P1-high | Done |

## Phase 2: 通道层 (T05-T08)

| ID | 任务 | 优先级 | 状态 |
|---|---|---|---|
| T05 | 新增：MessageChannel 接口定义 | P1-high | Done |
| T06 | 实现：MqttChannel (paho.mqtt.golang) | P1-high | Done |
| T07 | 新增：IpcChannel 占位实现 | P2-medium | Done |
| T08 | 实现：MultiChannel 多通道路由 | P1-high | Done |

## Phase 3: 核心层 (T09-T12)

| ID | 任务 | 优先级 | 状态 |
|---|---|---|---|
| T09 | 实现：MessageAdapter 消息路由 | P1-high | Done |
| T10 | 新增：TslinkClient 接口定义 | P1-high | Done |
| T11 | 实现：DefaultTslinkClient | P1-high | Done |
| T12 | 实现：TslinkClientBuilder | P1-high | Done |

## Phase 4: 辅助 + 验证 (T13-T16)

| ID | 任务 | 优先级 | 状态 |
|---|---|---|---|
| T13 | 实现：DeviceDiscovery 设备发现 | P2-medium | Done |
| T14 | 新增：单元测试 28+ 项 | P1-high | Done |
| T15 | 新增：mqtt_demo 示例 | P2-medium | Done |
| T16 | 新增：service_invoke 示例 | P2-medium | Done |
