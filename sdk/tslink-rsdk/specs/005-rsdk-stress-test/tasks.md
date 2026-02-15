# SPEC-005 Tasks: tslink-rsdk 深度测试与压力测试

## Phase 1: SDK 基础设施改造

### T001 [FR2] MQTT QoS 可配置化
- **File**: `src/channel/mqtt_channel.rs`, `src/client/builder.rs`
- **Action**:
  - `MqttConfig` 新增 `publish_qos: QoS` 和 `subscribe_qos: QoS` 字段（默认 AtMostOnce）
  - `MqttChannel::send()` 使用 `self.config.publish_qos.into()` 替代硬编码 `QoS::AtMostOnce`
  - `subscribe_topics()` 和 `add_topic()` 使用 `self.config.subscribe_qos.into()`
  - `TslinkClientBuilder` 新增 `.publish_qos()` / `.subscribe_qos()` 方法
- **Test**: `cargo check -p tslink-rsdk && cargo test -p tslink-rsdk`
- **Depends**: 无
- **Status**: [ ] Pending

### T002 [FR3] IPC MAX_PAYLOAD_SIZE 扩展
- **File**: `src/channel/ipc_channel.rs`
- **Action**:
  - 将 `MAX_PAYLOAD_SIZE` 从 `65536` (64KB) 扩展到 `16777216` (16MB)
  - 验证 `IpcPayload` struct 大小合理性（栈上 16MB 不可行，需改为堆分配）
  - 将 `IpcPayload.data` 从 `[u8; MAX_PAYLOAD_SIZE]` 改为 `Box<[u8]>` 或使用 iceoryx2 的 slice API
- **Test**: `cargo check -p tslink-rsdk --features ipc`
- **Depends**: 无
- **Status**: [ ] Pending

## Phase 2: 压测工具

### T003 [FR1] MQTT 高频上报压测工具
- **File**: `examples/bench_mqtt_throughput.rs`
- **Action**:
  - 支持参数: `--msg-size`, `--rate`, `--duration`, `--qos`
  - 创建 TslinkClient，按指定速率调用 `thing_property_post`
  - 采集: 发送成功/失败数、吞吐量(msg/s, MB/s)、P50/P95/P99 延迟
  - 输出结构化统计报告
- **Test**: `cargo run --example bench_mqtt_throughput -- --duration 10`
- **Depends**: T001
- **Status**: [ ] Pending

### T004 [FR2] QoS 可靠性测试工具
- **File**: `examples/bench_qos_reliability.rs`
- **Action**:
  - 启动 publisher + subscriber 两个 client
  - Publisher 发送 N 条带序号消息到自定义 topic
  - Subscriber 统计收到数量和序号完整性
  - 分别测试 QoS=0 和 QoS=1，输出丢包率对比
- **Test**: `cargo run --example bench_qos_reliability -- --count 1000`
- **Depends**: T001
- **Status**: [ ] Pending

### T005 [FR3] IPC 大帧传输测试工具
- **File**: `examples/bench_ipc_frame.rs`
- **Action**:
  - 生成模拟 4K YUV 帧 (3840×2160×1.5 ≈ 12.4MB)
  - 进程 A: IPC 发布帧
  - 进程 B: IPC 订阅帧
  - 测量: 单帧发布耗时、端到端延迟、帧率(fps)
- **Test**: `cargo run --example bench_ipc_frame --features ipc -- --frames 100`
- **Depends**: T002
- **Status**: [ ] Pending

### T006 [FR4] 新 API 功能验证工具
- **File**: `examples/bench_api_validation.rs`
- **Action**:
  - 逐一调用所有 SPEC-004 新 API 接口
  - 验证: 无 panic、回调触发、超时返回正确错误
  - 包含: property_post, event_post, service executor 注册, platform/device invoke sync/async
  - 输出 PASS/FAIL 报告
- **Test**: `cargo run --example bench_api_validation`
- **Depends**: 无
- **Status**: [ ] Pending

## Phase 3: Cargo.toml 与文档

### T007 更新 Cargo.toml
- **File**: `Cargo.toml`
- **Action**:
  - dev-dependencies 新增 `clap = { version = "4", features = ["derive"] }`
  - 新增 4 个 `[[example]]` 条目
- **Depends**: 无
- **Status**: [ ] Pending

### T008 运行全部压测并记录结果
- **Action**:
  - 依次运行 T003~T006 的 example
  - 记录输出到 `specs/005-rsdk-stress-test/results.md`
  - 标注发现的问题（如丢包率、性能瓶颈）
- **Depends**: T003, T004, T005, T006
- **Status**: [ ] Pending

## Dependency Graph

```
T001 ──→ T003 ──→ T008
     ──→ T004 ──→ T008
T002 ──→ T005 ──→ T008
          T006 ──→ T008
T007 (parallel)
```

## Estimated Effort

| Task | Effort |
|------|--------|
| T001 | 0.5h |
| T002 | 1h |
| T003 | 1.5h |
| T004 | 1h |
| T005 | 1.5h |
| T006 | 1h |
| T007 | 0.25h |
| T008 | 0.5h |
| **Total** | **~7.25h** |
