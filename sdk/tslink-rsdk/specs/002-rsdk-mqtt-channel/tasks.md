# Tasks: tslink-rsdk MQTT 通道

**Input**: Design documents from `/tslink-rsdk/specs/002-rsdk-mqtt-channel/`  
**Prerequisites**: plan.md ✅, spec.md ✅

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)

---

## Phase 1: Setup

**Purpose**: Project initialization and crate structure

- [ ] T001 Create Cargo.toml with dependencies (rumqttc, tokio, serde, async-trait)
- [ ] T002 [P] Create src/lib.rs with module exports
- [ ] T003 [P] Create src/error.rs with SDK error types

---

## Phase 2: Foundational (Message & Channel Abstractions)

**Purpose**: Core abstractions needed by all user stories

- [ ] T004 [P] Create src/enums.rs with EventType enum
- [ ] T005 [P] Create src/message/mod.rs module
- [ ] T006 [P] Create src/message/common.rs with CommonMessage struct
- [ ] T007 [P] Create src/message/reply.rs with ReplyMessage struct
- [ ] T008 Create src/channel/mod.rs module
- [ ] T009 Create src/channel/message_channel.rs with MessageChannel trait

**Checkpoint**: Foundation ready - channel implementation can begin

---

## Phase 3: User Story 1 - 设备属性上报 (Priority: P1) 🎯 MVP

**Goal**: 实现 thing_property_post 方法，支持属性上报

**Independent Test**: 调用 API 上报属性，验证消息格式正确

### Implementation for User Story 1

- [ ] T010 [P] [US1] Create src/channel/mqtt_channel.rs with MqttChannel struct
- [ ] T011 [US1] Implement MqttChannel::new() and connection logic
- [ ] T012 [US1] Implement MqttChannel::send() for message publishing
- [ ] T013 [US1] Implement MqttChannel::start() with auto-reconnect
- [ ] T014 [US1] Implement MqttChannel::stop()
- [ ] T015 [P] [US1] Create src/client/mod.rs module
- [ ] T016 [P] [US1] Create src/client/smart_client.rs with SmartClient trait
- [ ] T017 [US1] Create src/client/default_client.rs with DefaultSmartClient
- [ ] T018 [US1] Implement thing_property_post() method
- [ ] T019 [US1] Implement thing_property_post_for() for proxy devices
- [ ] T020 [US1] Run cargo build and cargo test to verify US1

**Checkpoint**: User Story 1 complete - 属性上报可测试

---

## Phase 4: User Story 2 - 设备事件上报 (Priority: P1)

**Goal**: 实现 thing_event_post 方法，支持事件上报

**Independent Test**: 调用 API 上报事件，验证消息格式和主题正确

### Implementation for User Story 2

- [ ] T021 [US2] Implement thing_event_post() method
- [ ] T022 [US2] Implement thing_event_post_with_reply() with callback support
- [ ] T023 [P] [US2] Create src/adapter/mod.rs module
- [ ] T024 [US2] Create src/adapter/message_adapter.rs with MessageAdapter
- [ ] T025 [US2] Implement callback registration and tid correlation
- [ ] T026 [US2] Run cargo build and cargo test to verify US2

**Checkpoint**: User Story 2 complete - 事件上报可测试

---

## Phase 5: User Story 3 - 接收云端服务调用 (Priority: P1)

**Goal**: 实现服务回调注册，接收平台下发指令

**Independent Test**: 注册回调，模拟收到服务调用消息，验证回调执行

### Implementation for User Story 3

- [ ] T027 [US3] Implement set_service_handle() method
- [ ] T028 [US3] Implement set_property_set_handle() method
- [ ] T029 [US3] Implement topic subscription for service calls
- [ ] T030 [US3] Implement message routing to callbacks
- [ ] T031 [US3] Implement platform_service_invoke() method
- [ ] T032 [US3] Run cargo build and cargo test to verify US3

**Checkpoint**: User Story 3 complete - 服务调用可测试

---

## Phase 6: User Story 4 - 多通道抽象 (Priority: P2)

**Goal**: 确保通道抽象可扩展，预留 IPC/HTTP 接口

**Independent Test**: 验证 MessageChannel trait 可被多种实现

### Implementation for User Story 4

- [ ] T033 [P] [US4] Create src/channel/ipc_channel.rs placeholder (空实现)
- [ ] T034 [P] [US4] Create src/channel/http_channel.rs placeholder (空实现)
- [ ] T035 [US4] Add channel factory pattern in src/client/builder.rs
- [ ] T036 [US4] Document multi-channel extension points

**Checkpoint**: User Story 4 complete - 多通道架构就绪

---

## Phase 7: Polish & Integration

**Purpose**: Documentation, testing, and cleanup

- [ ] T037 [P] Create README.md with usage examples
- [ ] T038 [P] Create tests/integration_test.rs
- [ ] T039 Add rustdoc documentation to public APIs
- [ ] T040 Run cargo clippy and fix warnings
- [ ] T041 Run cargo test --all
- [ ] T042 Update Plane work items with evidence

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies
- **Phase 2 (Foundational)**: Depends on Phase 1
- **Phase 3 (US1)**: Depends on Phase 2 - 属性上报
- **Phase 4 (US2)**: Depends on Phase 3 (需要 MqttChannel)
- **Phase 5 (US3)**: Depends on Phase 4 (需要 MessageAdapter)
- **Phase 6 (US4)**: Depends on Phase 5 (通道抽象验证)
- **Phase 7 (Polish)**: Depends on all user stories

### User Story Dependencies

- **US1 (属性上报)**: 基础功能，无依赖
- **US2 (事件上报)**: 依赖 US1 的 MqttChannel
- **US3 (服务调用)**: 依赖 US2 的 MessageAdapter
- **US4 (多通道)**: 依赖 US1-3 的实现验证

### Parallel Opportunities

- T004, T005, T006, T007 可并行
- T010, T015, T016 可并行
- T023 可与 T021, T022 并行
- T033, T034 可并行
- T037, T038 可并行

---

## Implementation Strategy

### MVP First (User Stories 1-3)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: US1 (属性上报)
4. Complete Phase 4: US2 (事件上报)
5. Complete Phase 5: US3 (服务调用)
6. **STOP and VALIDATE**: MVP 完成
7. Continue to Phase 6-7 for polish

---

## Plane Sync

| Task Range | Plane Work Item Name | Module |
|------------|---------------------|--------|
| T001-T003 | [002-RSDK][T01-03] Setup: 项目初始化 | tslink-rsdk |
| T004-T009 | [002-RSDK][T04-09] Foundational: 消息与通道抽象 | tslink-rsdk |
| T010-T020 | [002-RSDK][US1] 实现: 设备属性上报 | tslink-rsdk |
| T021-T026 | [002-RSDK][US2] 实现: 设备事件上报 | tslink-rsdk |
| T027-T032 | [002-RSDK][US3] 实现: 云端服务调用 | tslink-rsdk |
| T033-T036 | [002-RSDK][US4] 实现: 多通道抽象 | tslink-rsdk |
| T037-T042 | [002-RSDK][Polish] 文档与测试 | tslink-rsdk |

---

## Notes

- 参考 JASmartSDK Java 实现
- 使用 async-trait 实现异步 trait
- 使用 rumqttc 作为 MQTT 客户端
- 保持 API 命名与 Java 版一致
