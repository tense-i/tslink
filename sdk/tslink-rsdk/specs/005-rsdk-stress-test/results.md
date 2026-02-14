# SPEC-005 压测结果报告

**日期**: 2026-02-14  
**环境**: macOS, localhost Mosquitto broker, debug build  
**RSDK 版本**: 当前开发分支 (SPEC-004 + SPEC-005)

---

## 1. MQTT 高频上报吞吐量 (bench_mqtt_throughput)

| 指标 | QoS=0 | QoS=1 |
|---|---|---|
| 持续时间 | 10s | 10s |
| 消息大小 | 256 bytes | 256 bytes |
| 总发送 | 318,462 | 311,201 |
| 成功 | 318,462 | 311,201 |
| 错误 | 0 | 0 |
| **吞吐量** | **31,846 msg/s** | **31,120 msg/s** |
| 带宽 | 7.775 MB/s | 7.598 MB/s |
| 延迟 avg | 0.031 ms | 0.032 ms |
| 延迟 P50 | 0.017 ms | 0.017 ms |
| 延迟 P95 | 0.061 ms | 0.063 ms |
| 延迟 P99 | 0.256 ms | 0.261 ms |
| 错误率 | 0.00% | 0.00% |

**结论**: SDK 在 QoS=0 和 QoS=1 下吞吐量接近，均超过 31K msg/s。P99 延迟 < 0.3ms。无 OOM、无 panic。

---

## 2. QoS 可靠性测试 (bench_qos_reliability)

### QoS=0 (50,000 条)

| 指标 | 值 |
|---|---|
| 发送成功 | 50,000 |
| 接收 | 50,000 |
| 丢失 | 0 |
| **丢包率** | **0.00%** |
| 乱序 | 0 |
| 重复 | 0 |
| 发送吞吐量 | 208,161 msg/s |

### QoS=1 (50,000 条)

| 指标 | 值 |
|---|---|
| 发送成功 | 50,000 |
| 接收 | 50,000 |
| 丢失 | 0 |
| **丢包率** | **0.00%** |
| 乱序 | 0 |
| 重复 | 0 |
| 发送吞吐量 | 46,472 msg/s |

**结论**: 在 localhost 环境下，QoS=0 和 QoS=1 均实现 0% 丢包。QoS=1 吞吐量约为 QoS=0 的 22%（因 ACK 开销）。在真实网络环境下，QoS=0 可能出现丢包，建议关键业务使用 QoS=1。

---

## 3. 新 API 功能验证 (bench_api_validation)

| # | API | 状态 | 说明 |
|---|---|---|---|
| 1 | thing_property_post | ✅ PASS | 属性上报成功 |
| 2 | thing_property_post_for | ✅ PASS | 代理上报成功 |
| 3 | thing_event_post (Info) | ✅ PASS | 信息事件上报 |
| 4 | thing_event_post (Warning) | ✅ PASS | 告警事件上报 |
| 5 | thing_event_post (Error) | ✅ PASS | 错误事件上报 |
| 6 | set_service_specific_executor | ✅ PASS | 注册特定服务执行器 |
| 7 | set_service_unified_executor | ✅ PASS | 注册统一服务执行器 |
| 8 | set_property_set_executor | ✅ PASS | 注册属性设置执行器 |
| 9 | platform_service_invoke_async | ✅ PASS | 异步平台服务调用 |
| 10 | platform_service_invoke_sync | ✅ PASS | 同步平台服务调用（超时正确） |
| 11 | device_service_invoke_async | ✅ PASS | 异步设备服务调用 |
| 12 | device_service_invoke_sync | ✅ PASS | 同步设备服务调用（超时正确） |

**结论**: 全部 12 项 API 测试通过。SPEC-004 对齐的新服务 API 功能正常。

---

## 4. IPC 大帧传输 (bench_ipc_frame)

**模式**: RSDK IpcChannel API（统一 iceoryx2 线程架构 + slice API），单进程 pub+sub，release build  
**MAX_PAYLOAD_SIZE**: 12MB（可配置，默认 `DEFAULT_MAX_PAYLOAD_SIZE`）  
**架构**: 单一 iceoryx2 线程持有一个 Node，通过 mpsc 命令通道处理 publish/subscribe，50µs 轮询间隔  
**Payload**: iceoryx2 `publish_subscribe::<[u8]>()` slice API，按需分配，无固定大小限制

| Frame Size | Frames | Loss | FPS | Bandwidth | Pub Avg | Pub P99 | E2E Avg | E2E P99 |
|---|---|---|---|---|---|---|---|---|
| 100 B | 10,000 | 0 | 11,988 | 1.14 MB/s | 0.083ms | 0.242ms | 0.080ms | 0.238ms |
| 1 KB | 5,000 | 0 | 6,125 | 5.84 MB/s | 0.162ms | 1.718ms | 0.153ms | 1.224ms |
| 64 KB | 1,000 | 0 | 10,203 | 622.77 MB/s | 0.095ms | 0.146ms | 0.096ms | 0.139ms |
| 200 KB | 100 | 0 | 6,395 | 1,219 MB/s | 0.145ms | 0.230ms | 0.158ms | 0.254ms |
| 1 MB | 100 | 0 | 1,922 | 1,833 MB/s | 0.451ms | 1.067ms | 0.593ms | 1.645ms |
| 4 MB | 50 | 0 | 432 | 1,647 MB/s | 1.803ms | 2.799ms | 2.901ms | 4.680ms |
| **12 MB** | 20 | 0 | 149 | 1,705 MB/s | 5.122ms | 6.985ms | 8.454ms | 11.627ms |

**关键发现**:
- **零丢帧**: 所有帧大小均 0% 丢失（100B ~ 12MB）
- **64KB 限制已移除**: 通过 iceoryx2 slice API (`publish_subscribe::<[u8]>()` + `loan_slice_uninit`)，支持任意大小帧
- **12MB 4K YUV 帧**: 149 fps / 1.7 GB/s 带宽，E2E avg 8.5ms
- **高吞吐**: 64KB 帧达 10K fps / 623 MB/s，1MB 帧达 1.8 GB/s
- **统一线程架构**: 解决了 macOS 上 iceoryx2 多 Node 的 InternalError 问题
- **按需分配**: 小消息不浪费内存，大消息按实际大小分配共享内存

**C++ SDK 对比**: C++ SDK 使用 eCAL（无大小限制），RSDK 使用 iceoryx2 slice API（可配置上限，默认 12MB）。性能对等。

---

## 5. 总结

| 验收标准 | 结果 |
|---|---|
| MQTT 吞吐量 ≥ 1000 msg/s | ✅ 31,846 msg/s (31x) |
| QoS=1 丢包率 = 0% | ✅ 0.00% |
| MQTT P99 延迟 < 100ms | ✅ 0.261 ms |
| IPC 零丢帧 (RSDK IpcChannel) | ✅ 0% loss (100B ~ 12MB) |
| IPC E2E P99 延迟 < 1ms (64KB) | ✅ 0.139ms (64KB帧, RSDK) |
| IPC 吞吐量 | ✅ 10.2K fps / 623 MB/s (64KB帧, RSDK) |
| IPC 12MB 4K YUV 帧 | ✅ 149 fps / 1.7 GB/s (T002 完成) |
| 无 panic / OOM | ✅ 全程稳定 |
| 新 API 全部可用 | ✅ 12/12 通过 |

---

## 运行命令参考

```bash
# MQTT 吞吐量
cargo run --example bench_mqtt_throughput -- --duration 30 --msg-size 256 --qos 0
cargo run --example bench_mqtt_throughput -- --duration 30 --msg-size 256 --qos 1

# QoS 可靠性
cargo run --example bench_qos_reliability -- --count 50000 --qos 0
cargo run --example bench_qos_reliability -- --count 50000 --qos 1

# API 验证
cargo run --example bench_api_validation

# IPC 大帧 (需要 ipc feature, RSDK IpcChannel)
cargo run --release --example bench_ipc_frame --features ipc -- --frames 10000 --msg-size 1000 --settle-ms 1000
cargo run --release --example bench_ipc_frame --features ipc -- --frames 10000 --msg-size 64000 --settle-ms 1000
```
