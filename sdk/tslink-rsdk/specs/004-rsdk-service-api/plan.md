# PLAN: SPEC-004 tslink-rsdk Service API Alignment

## Summary

对齐 Rust SDK 的服务调用/处理接口到 `ja-IOT-SDK-cpp` 语义（去掉 `JA_` 前缀），使用强类型请求/响应结构体替换当前 `serde_json::Value` 泛型入参/出参，并直接重写旧 API。

## Technical Context

- 现有服务调用入口在 `TslinkClient`：`set_service_handle`, `platform_service_invoke`, `platform_service_invoke_for`。
- 处理链路由 `MessageAdapter` 解析 `CommonMessage`/`ReplyMessage`，目前回调入参/出参为 `serde_json::Value`。
- MQTT topic 结构：
  - 平台服务调用: `sys/{pk}/{did}/platform/service/{identity}/post`
  - 服务调用回复: `.../post_reply`
  - 设备服务调用: `sys/{pk}/{did}/thing/service/{identity}/post`

## Data Models

### PlatformServiceRequest

```rust
pub struct PlatformServiceRequest {
    pub channel: CommunicationChannel,
    pub service_identifier: String,
    pub param_data: Vec<u8>,
    pub product_key: String,
    pub device_id: String,
}
```

### DeviceServiceRequest

```rust
pub struct DeviceServiceRequest {
    pub channel: CommunicationChannel,
    pub service_identifier: String,
    pub param_data: Vec<u8>,
    pub service_timestamp_ms: i64,
}
```

### PlatformServiceResponse / DeviceServiceResponse

```rust
pub struct PlatformServiceResponse {
    pub channel: CommunicationChannel,
    pub service_identifier: String,
    pub result: i32,
    pub param_data: Vec<u8>,
    pub service_timestamp_ms: i64,
}
```

```rust
pub struct DeviceServiceResponse {
    pub channel: CommunicationChannel,
    pub service_identifier: String,
    pub result: i32,
    pub param_data: Vec<u8>,
    pub service_timestamp_ms: i64,
}
```

### Callbacks

```rust
pub type ReplyCallback = Arc<dyn Fn(i32, Vec<u8>) + Send + Sync>;

pub type ServiceExecutor = Arc<dyn Fn(DeviceServiceRequest, ReplyCallback) + Send + Sync>;
pub type PlatformResponseCallback = Arc<dyn Fn(PlatformServiceResponse) + Send + Sync>;
pub type ServiceResponseCallback = Arc<dyn Fn(DeviceServiceResponse) + Send + Sync>;
```

## API Contracts

### 对外接口命名（去掉 JA_ 前缀）

| C++ 名称 | Rust 名称 |
|---------|-----------|
| JA_setPlatformPushUnifiedExecutor | set_platform_push_unified_executor |
| JA_setPlatformPushSpecificExecutor | set_platform_push_specific_executor |
| JA_platformServiceInvokeSync | platform_service_invoke_sync |
| JA_platformServiceInvokeASync | platform_service_invoke_async |
| JA_setServiceUnifiedExecutor | set_service_unified_executor |
| JA_setServiceSpecificExecutor | set_service_specific_executor |
| JA_deviceServiceInvokeSync | device_service_invoke_sync |
| JA_deviceServiceInvokeASync | device_service_invoke_async |

### Trait 入口（替换 TslinkClient 服务相关方法）

```rust
#[async_trait]
pub trait TslinkClient {
    fn set_platform_push_unified_executor(
        &self,
        executor: ServiceExecutor,
        product_key: &str,
        device_id: &str,
    ) -> Result<()>;

    fn set_platform_push_specific_executor(
        &self,
        identifier: &str,
        executor: ServiceExecutor,
        product_key: &str,
        device_id: &str,
    ) -> Result<()>;

    async fn platform_service_invoke_sync(
        &self,
        request: PlatformServiceRequest,
        timeout_ms: i32,
    ) -> Result<PlatformServiceResponse>;

    async fn platform_service_invoke_async(
        &self,
        request: PlatformServiceRequest,
        callback: PlatformResponseCallback,
    ) -> Result<()>;

    fn set_service_unified_executor(
        &self,
        executor: ServiceExecutor,
        channel: CommunicationChannel,
        product_key: &str,
        device_id: &str,
    ) -> Result<()>;

    fn set_service_specific_executor(
        &self,
        identifier: &str,
        executor: ServiceExecutor,
        channel: CommunicationChannel,
        product_key: &str,
        device_id: &str,
    ) -> Result<()>;

    async fn device_service_invoke_sync(
        &self,
        request: DeviceServiceRequest,
        product_key: &str,
        device_id: &str,
        timeout_ms: i32,
    ) -> Result<DeviceServiceResponse>;

    async fn device_service_invoke_async(
        &self,
        request: DeviceServiceRequest,
        product_key: &str,
        device_id: &str,
        callback: ServiceResponseCallback,
    ) -> Result<()>;
}
```

## Implementation Phases

### Phase 1: 类型定义与导出
- 新增 `src/message/service.rs` 定义请求/响应结构体与回调类型。
- 更新 `src/message/mod.rs` 导出服务类型。
- 更新 `prelude` 导出服务 API。

### Phase 2: 服务处理与调用重写
- 重写 `MessageAdapter`：使用新 ServiceExecutor 与 ReplyCallback；构造/解析 `param_data`。
- 更新 `TslinkClient` trait 与 `DefaultTslinkClient` 实现：替换旧方法。

### Phase 3: 示例与测试
- 更新 `examples/service_invoke_test.rs` / `examples/mqtt_demo.rs` 使用新 API。
- 添加/调整单元测试验证回调与响应结构。

## Risks

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 旧接口移除导致示例/调用失败 | 高 | 同步更新示例与文档 | 
| payload 解析不一致 | 中 | 明确 param_data 为原始 bytes，SDK 不解析 |

## Success Criteria

- 新接口可完成平台服务调用与设备服务调用（同步/异步）。
- handler 入参/出参均使用结构体类型。
- 示例通过编译并能触发服务回调。
