# TASKS: SPEC-004 tslink-rsdk Service API Alignment

## Task Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Phase 1: 类型与导出 | T001-T003 | Pending |
| Phase 2: API 替换 | T004-T010 | Pending |
| Phase 3: 示例与测试 | T011-T013 | Pending |

---

## Phase 1: 类型与导出

### T001 [FR1] 新增服务请求/响应类型
- **File**: `src/message/service.rs`
- **Action**: 定义 `PlatformServiceRequest/Response`, `DeviceServiceRequest/Response`, `ReplyCallback`, `ServiceExecutor` 等类型
- **Test**: `cargo test -p tslink-rsdk message::service`（新增单测）
- **Status**: [ ] Pending

### T002 [FR1][FR4] 导出服务类型
- **File**: `src/message/mod.rs`, `src/lib.rs`
- **Action**: 导出服务类型与回调别名；更新 prelude
- **Depends**: T001
- **Status**: [ ] Pending

### T003 [FR4] 替换 TslinkClient 服务接口定义
- **File**: `src/client/tslink_client.rs`
- **Action**: 移除旧接口，新增对齐命名接口（sync/async + unified/specific executors）
- **Depends**: T001
- **Status**: [ ] Pending

---

## Phase 2: API 替换

### T004 [FR2] 重写 MessageAdapter 服务回调模型
- **File**: `src/adapter/message_adapter.rs`
- **Action**: 使用 `ServiceExecutor` + `ReplyCallback`；将 `CommonMessage` data 映射为 `param_data` bytes
- **Depends**: T001
- **Status**: [ ] Pending

### T005 [FR2] 适配平台 push 处理器注册
- **File**: `src/client/default_client.rs`
- **Action**: 实现 `set_platform_push_unified_executor` / `set_platform_push_specific_executor`
- **Depends**: T003, T004
- **Status**: [ ] Pending

### T006 [FR2] 适配设备服务处理器注册
- **File**: `src/client/default_client.rs`
- **Action**: 实现 `set_service_unified_executor` / `set_service_specific_executor`
- **Depends**: T003, T004
- **Status**: [ ] Pending

### T007 [FR3] 实现平台服务调用 sync/async
- **File**: `src/client/default_client.rs`
- **Action**: 实现 `platform_service_invoke_sync` / `platform_service_invoke_async`
- **Depends**: T003
- **Status**: [ ] Pending

### T008 [FR3] 实现设备服务调用 sync/async
- **File**: `src/client/default_client.rs`
- **Action**: 实现 `device_service_invoke_sync` / `device_service_invoke_async`
- **Depends**: T003
- **Status**: [ ] Pending

### T009 [FR4] 更新默认订阅 topic
- **File**: `src/channel/mqtt_channel.rs`
- **Action**: 保证 platform/service 与 thing/service 回复 topic 仍可匹配新回调
- **Depends**: T004
- **Status**: [ ] Pending

### T010 [FR5] 移除旧服务 API 调用
- **File**: `src/client/default_client.rs`, `examples/*`
- **Action**: 删除旧 `set_service_handle` / `platform_service_invoke` 等接口使用
- **Depends**: T003
- **Status**: [ ] Pending

---

## Phase 3: 示例与测试

### T011 [FR3] 更新 service_invoke_test 示例
- **File**: `examples/service_invoke_test.rs`
- **Action**: 使用新 service request/response 与执行器 API
- **Depends**: T007, T008
- **Status**: [ ] Pending

### T012 [FR3] 更新 mqtt_demo 示例
- **File**: `examples/mqtt_demo.rs`
- **Action**: 替换服务调用相关 API
- **Depends**: T007, T008
- **Status**: [ ] Pending

### T013 [FR1] 添加服务类型单元测试
- **File**: `src/message/service.rs`
- **Action**: 测试序列化/反序列化、回调调用
- **Depends**: T001
- **Status**: [ ] Pending

---

## Completion Checklist

- [ ] 新服务 API 编译通过
- [ ] 示例可运行
- [ ] 相关单测通过
