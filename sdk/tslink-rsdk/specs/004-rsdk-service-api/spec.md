# SPEC-004: tslink-rsdk Service API Alignment

## Meta

| Field | Value |
|-------|-------|
| Spec ID | SPEC-004 |
| Title | tslink-rsdk Service API Alignment |
| Status | Draft |
| Created | 2026-02-14 |
| Updated | 2026-02-14 |
| Author | AI Assistant |

## Problem Statement

Rust 版 tslink-rsdk 当前的服务调用接口与 `ja-IOT-SDK-cpp` 的对外接口命名与数据类型不一致，且 handler 使用泛型 JSON 作为输入输出，导致跨语言对齐困难与类型语义不清晰。需要在 Rust SDK 中提供与 C++ SDK 语义一致的服务调用与处理接口，并使用明确的请求/响应类型，而不是 `serde_json::Value`。

## Goals

1. **接口命名对齐**：服务调用相关接口名称与 `ja-IOT-SDK-cpp` 保持一致（去掉 `JA_` 前缀）。
2. **类型明确**：服务调用的请求/响应与 handler 入参/出参使用明确类型定义，避免泛型 JSON 直接暴露。
3. **覆盖核心场景**：支持平台能力调用、设备间能力调用、服务处理器注册（统一/特定）与异步响应回调。
4. **直接替换旧接口**：不需要兼容旧的服务调用 API，按对齐接口直接重写。

## Non-Goals

- 属性/事件上报接口的完整对齐（本阶段不包含）。
- 二进制订阅、IPC、设备物模型等扩展能力对齐。
- 变更底层 MQTT/IPC 传输机制。

## User Stories

### US1: 作为设备开发者，我需要按 C++ SDK 命名调用平台服务

**验收标准:**
- [ ] 可以使用与 C++ SDK 对齐的接口名称调用平台服务（去掉 `JA_`）。
- [ ] 调用参数使用明确的请求结构体。
- [ ] 同步/异步调用均可用。

### US2: 作为服务提供方，我需要用强类型 handler 处理服务请求

**验收标准:**
- [ ] handler 接收结构化的服务请求类型，而非 `serde_json::Value`。
- [ ] handler 使用标准 reply 回调返回结构化响应。

### US3: 作为平台集成开发者，我需要设备间服务调用接口一致

**验收标准:**
- [ ] 设备间服务调用接口名称与 C++ SDK 对齐。
- [ ] 设备间服务调用响应类型明确。

## Functional Requirements

### FR1: 服务调用类型定义

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1.1 | 定义 `PlatformServiceRequest` 类型，包含 serviceIdentifier、paramData、productKey、deviceId、channel | P1 |
| FR1.2 | 定义 `PlatformServiceResponse` 类型，包含 serviceIdentifier、result、paramData、serviceTimestampMs、channel | P1 |
| FR1.3 | 定义 `DeviceServiceRequest` 类型，包含 serviceIdentifier、paramData、serviceTimestampMs、channel | P1 |
| FR1.4 | 定义 `DeviceServiceResponse` 类型，包含 serviceIdentifier、result、paramData、serviceTimestampMs、channel | P1 |
| FR1.5 | 定义 `ReplyCallback` 类型用于服务响应，入参包含 result 与数据内容 | P1 |

### FR2: 服务处理器接口

| ID | Requirement | Priority |
|----|-------------|----------|
| FR2.1 | 提供 `set_platform_push_unified_executor` 对应统一平台推送处理器 | P1 |
| FR2.2 | 提供 `set_platform_push_specific_executor` 对应特定平台推送处理器 | P1 |
| FR2.3 | 提供 `set_service_unified_executor` 对应统一设备服务处理器 | P1 |
| FR2.4 | 提供 `set_service_specific_executor` 对应特定设备服务处理器 | P1 |
| FR2.5 | handler 入参/出参使用 FR1 中定义的强类型 | P1 |

### FR3: 服务调用接口

| ID | Requirement | Priority |
|----|-------------|----------|
| FR3.1 | 提供 `platform_service_invoke_sync` 与 `platform_service_invoke_async` | P1 |
| FR3.2 | 提供 `device_service_invoke_sync` 与 `device_service_invoke_async` | P1 |
| FR3.3 | 同步接口支持超时参数 | P1 |
| FR3.4 | 异步接口使用回调返回响应 | P1 |

### FR4: 命名对齐

| ID | Requirement | Priority |
|----|-------------|----------|
| FR4.1 | Rust 对外接口名称与 C++ SDK 对应名称一致（去掉 `JA_` 前缀） | P1 |
| FR4.2 | 公开类型名称与 C++ SDK 对应类型一致（去掉 `Ja` 前缀） | P1 |

### FR5: 替换策略

| ID | Requirement | Priority |
|----|-------------|----------|
| FR5.1 | 旧的服务调用接口将被移除或替换为对齐接口 | P1 |

## Key Entities

### PlatformServiceRequest

- `channel`: CommunicationChannel
- `service_identifier`: String
- `param_data`: Vec<u8>
- `product_key`: String
- `device_id`: String

### DeviceServiceRequest

- `channel`: CommunicationChannel
- `service_identifier`: String
- `param_data`: Vec<u8>
- `service_timestamp_ms`: i64

### PlatformServiceResponse / DeviceServiceResponse

- `channel`: CommunicationChannel
- `service_identifier`: String
- `result`: i32
- `param_data`: Vec<u8>
- `service_timestamp_ms`: i64

### ServiceExecutor / ReplyCallback

- `ServiceExecutor`: 接收服务请求类型与 `ReplyCallback`
- `ReplyCallback`: 返回 `result` 与 `data` 内容

## Assumptions

- Rust 接口名称使用 snake_case，但语义与 C++ SDK 对应名称一致（去掉 `JA_` 前缀）。
- `param_data` 内容为 UTF-8 JSON 字符串或二进制数据，SDK 不强制解析。
- 旧服务调用接口将被替换为本次对齐接口。

## Success Criteria

1. [ ] Rust SDK 可以使用对齐名称完成平台服务调用（同步/异步）。
2. [ ] 服务 handler 的输入/输出使用明确类型定义。
3. [ ] 接口文档或示例能展示与 C++ SDK 一致的服务调用流程。

## Open Questions

None.
