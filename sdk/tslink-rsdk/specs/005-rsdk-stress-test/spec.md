# SPEC-005: tslink-rsdk 深度测试与压力测试

## Overview

对 tslink-rsdk 进行全面的深度测试与压力测试，验证 SDK 在高负载、大数据量、多通道场景下的可靠性与性能。

## Goals

1. **持续大数据上报性能** — 测量高频属性/事件上报的吞吐量、延迟、内存占用
2. **QoS 可靠性验证** — 验证当前 QoS=0 下服务调用的丢失率，并支持 QoS=1 对比测试
3. **IPC 大帧传输性能** — 通过 IPC 通道发布 4K YUV 图片帧，测量发布/订阅延迟与吞吐
4. **事件/服务接口功能验证** — 覆盖 thing_event_post、platform_service_invoke_async/sync、device_service_invoke_async/sync 等全部新 API

## Scope

### In Scope

- MQTT 通道高频属性上报压测（1K~10K msg/s）
- MQTT 通道事件上报压测
- MQTT QoS=0 vs QoS=1 服务调用可靠性对比
- IPC 通道大帧（4K YUV ~12MB）发布/订阅性能
- IPC 通道 MAX_PAYLOAD_SIZE 扩展（当前 64KB 不足以传输 4K YUV）
- 新服务 API 全接口功能验证
- 性能指标采集：吞吐量(msg/s)、P50/P95/P99 延迟、内存占用、丢包率

### Out of Scope

- HTTP 通道测试（未实现）
- 跨网络/跨机器分布式压测
- CI/CD 集成

## Functional Requirements

### FR1: MQTT 高频上报压测
- 支持配置并发数、消息大小、持续时间
- 测量发送吞吐量 (msg/s, MB/s)
- 测量端到端延迟 (P50/P95/P99)
- 测量内存增长趋势

### FR2: QoS 可靠性测试
- 当前 MQTT publish/subscribe 硬编码 QoS=0，需支持可配置 QoS
- 发送 N 条消息，统计接收端实际收到的数量，计算丢包率
- 对比 QoS=0 和 QoS=1 的丢包率差异

### FR3: IPC 大帧传输测试
- 扩展 IPC MAX_PAYLOAD_SIZE 以支持 4K YUV 帧 (~12MB)
- 设备 A 通过 IPC 发布 4K YUV 帧
- 设备 B 通过 IPC 订阅并接收
- 测量单帧发布延迟、订阅延迟、帧率 (fps)

### FR4: 新 API 功能验证
- thing_property_post / thing_property_post_for
- thing_event_post
- set_service_specific_executor / set_service_unified_executor
- platform_service_invoke_sync / platform_service_invoke_async
- device_service_invoke_sync / device_service_invoke_async
- set_property_set_executor

## Non-Functional Requirements

- 压测工具以 Rust example 形式提供，可通过 `cargo run --example` 运行
- 测试结果输出到 stdout，包含结构化统计摘要
- 压测参数通过环境变量或命令行参数配置

## Acceptance Criteria

1. MQTT 高频上报压测可持续运行 60s 无 panic/OOM
2. QoS=0 丢包率统计准确，QoS=1 丢包率显著低于 QoS=0
3. IPC 通道可成功传输 4K YUV 帧（~12MB），帧率 ≥ 10fps
4. 所有新服务 API 功能验证通过
5. 输出包含吞吐量、延迟分位数、丢包率的结构化报告

## Technical Constraints

- IPC 当前 `MAX_PAYLOAD_SIZE = 64KB`，需扩展至 ≥ 16MB
- MQTT QoS 当前硬编码为 0，需重构为可配置
- iceoryx2 feature 为 optional (`ipc` feature flag)

## Open Questions

None.
