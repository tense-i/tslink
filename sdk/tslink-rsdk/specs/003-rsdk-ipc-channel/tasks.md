# TASKS: SPEC-003 tslink-rsdk IPC Channel & Multi-Channel Support

## Task Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Phase 1: 核心 IPC 通道 | T001-T008 | ✅ Completed |
| Phase 2: 多通道支持 | T009-T014 | Pending |
| Phase 3: 设备发现 | T015-T019 | Pending |
| Phase 4: 集成测试 | T020-T022 | Pending |

---

## Phase 1: 核心 IPC 通道

### T001 [US1] 添加 iceoryx2 依赖
- **File**: `Cargo.toml`
- **Action**: 添加 iceoryx2 依赖和 ipc feature flag
- **Test**: `cargo build --features ipc`
- **Status**: [x] Completed

### T002 [US1] 创建 IpcConfig 配置结构
- **File**: `src/channel/ipc_channel.rs`
- **Action**: 定义 IpcConfig struct
- **Test**: 单元测试配置默认值
- **Status**: [x] Completed

### T003 [US1] 创建 IpcChannel 基本结构
- **File**: `src/channel/ipc_channel.rs`
- **Action**: 定义 IpcChannel struct 和 new() 方法
- **Depends**: T001, T002
- **Status**: [x] Completed

### T004 [US3] 实现 MessageChannel trait for IpcChannel
- **File**: `src/channel/ipc_channel.rs`
- **Action**: 实现 send, add_topic, start, stop, is_connected
- **Depends**: T003
- **Status**: [x] Completed

### T005 [US1] 实现 IpcChannel::send() 发布消息
- **File**: `src/channel/ipc_channel.rs`
- **Action**: 使用 iceoryx2 publisher 发送消息
- **Depends**: T004
- **Status**: [x] Completed

### T006 [US1] 实现 IpcChannel::add_topic() 订阅 topic
- **File**: `src/channel/ipc_channel.rs`
- **Action**: 创建 subscriber 并启动接收循环
- **Depends**: T004
- **Status**: [x] Completed

### T007 [US1] 实现 IpcChannel::start() 和 stop()
- **File**: `src/channel/ipc_channel.rs`
- **Action**: 启动/停止 IPC 节点
- **Depends**: T004
- **Status**: [x] Completed

### T008 [US3] 更新 channel/mod.rs 导出 IpcChannel
- **File**: `src/channel/mod.rs`
- **Action**: 条件导出 IpcChannel (feature = "ipc")
- **Depends**: T007
- **Status**: [x] Completed

---

## Phase 2: 多通道支持

### T009 [US4] 创建 CommunicationChannel 枚举
- **File**: `src/enums.rs`
- **Action**: 定义 All, Remote, Ipc 通道枚举
- **Test**: 单元测试枚举值
- **Status**: [ ] Pending

### T010 [US4] 创建 MultiChannel 结构
- **File**: `src/channel/multi_channel.rs`
- **Action**: 定义 MultiChannel 管理 MQTT 和 IPC 通道
- **Depends**: T009
- **Status**: [ ] Pending

### T011 [US4] 实现 MultiChannel::send_with_channel()
- **File**: `src/channel/multi_channel.rs`
- **Action**: 根据通道类型路由消息发送
- **Depends**: T010
- **Status**: [ ] Pending

### T012 [US4] 实现 MultiChannel::subscribe_with_channel()
- **File**: `src/channel/multi_channel.rs`
- **Action**: 根据通道类型路由订阅
- **Depends**: T010
- **Status**: [ ] Pending

### T013 [US4] 更新 TslinkClient trait 添加通道选择方法
- **File**: `src/client/tslink_client.rs`
- **Action**: 添加 *_with_channel 方法
- **Depends**: T009
- **Status**: [ ] Pending

### T014 [US4] 更新 DefaultTslinkClient 实现多通道
- **File**: `src/client/default_client.rs`
- **Action**: 使用 MultiChannel 替换单一通道
- **Depends**: T013
- **Status**: [ ] Pending

---

## Phase 3: 设备发现

### T015 [US2] 创建 discovery 模块
- **File**: `src/discovery/mod.rs`, `src/discovery/device_discovery.rs`
- **Action**: 创建设备发现模块结构
- **Status**: [ ] Pending

### T016 [US2] 实现 DeviceDiscovery 基本结构
- **File**: `src/discovery/device_discovery.rs`
- **Action**: 定义 DeviceDiscovery struct
- **Depends**: T015
- **Status**: [ ] Pending

### T017 [US2] 实现设备广播功能
- **File**: `src/discovery/device_discovery.rs`
- **Action**: 周期性发布设备信息到发现 topic
- **Depends**: T016
- **Status**: [ ] Pending

### T018 [US2] 实现设备缓存管理
- **File**: `src/discovery/device_discovery.rs`
- **Action**: 维护设备缓存，清理过期设备
- **Depends**: T016
- **Status**: [ ] Pending

### T019 [US2] 实现设备状态回调
- **File**: `src/discovery/device_discovery.rs`
- **Action**: 新设备上线/下线时触发回调
- **Depends**: T018
- **Status**: [ ] Pending

---

## Phase 4: 集成测试

### T020 [US1] 创建 IPC 单元测试
- **File**: `src/channel/ipc_channel.rs` (tests mod)
- **Action**: 测试 IpcConfig, IpcChannel 创建
- **Depends**: T008
- **Status**: [x] Completed

### T021 [US1] 创建 IPC 集成测试 demo
- **File**: `examples/ipc_demo.rs`
- **Action**: 创建双进程通信示例
- **Depends**: T020
- **Status**: [ ] Pending

### T022 [US3] 更新 TslinkClientBuilder 支持多通道
- **File**: `src/client/builder.rs`
- **Action**: 添加 channel_type 配置选项
- **Depends**: T014
- **Status**: [ ] Pending

---

## Completion Checklist

- [ ] 所有 P1 任务完成
- [ ] cargo build --features ipc 通过
- [ ] cargo test --features ipc 通过
- [ ] IPC demo 可运行
- [ ] 代码已提交
