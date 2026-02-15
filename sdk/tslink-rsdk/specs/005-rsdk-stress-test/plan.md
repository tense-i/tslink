# SPEC-005 Plan: tslink-rsdk 深度测试与压力测试

## Technical Context

- MQTT channel (`mqtt_channel.rs`): QoS 硬编码为 `QoS::AtMostOnce` (0)，publish 和 subscribe 均如此
- IPC channel (`ipc_channel.rs`): 基于 iceoryx2，`MAX_PAYLOAD_SIZE = 64KB`，不足以传输 4K YUV (~12MB)
- `MessageChannel` trait: `send()` 接受 `&str`，大二进制帧需要 base64 或扩展接口
- 新服务 API (SPEC-004): 已完成，需要端到端功能验证
- dev-dependencies 已有 `tokio-test`, `chrono`, `reqwest`, `tracing-subscriber`

## Implementation Phases

### Phase 1: SDK 基础设施改造

#### 1.1 MQTT QoS 可配置化
- `MqttConfig` 新增 `publish_qos: QoS` 和 `subscribe_qos: QoS` 字段
- `MqttChannel::send()` 使用 `self.config.publish_qos` 而非硬编码
- `MqttChannel::subscribe_topics()` 使用 `self.config.subscribe_qos`
- `TslinkClientBuilder` 新增 `.qos()` 方法
- 默认值保持 QoS=0 以兼容现有行为

#### 1.2 IPC MAX_PAYLOAD_SIZE 扩展
- 将 `MAX_PAYLOAD_SIZE` 从 64KB 扩展到 16MB (`16 * 1024 * 1024`)
- 考虑使用编译时 feature 或 const generic 控制大小
- 验证 iceoryx2 shared memory 对大 payload 的支持

### Phase 2: 压测工具实现

#### 2.1 MQTT 高频上报压测 (`examples/bench_mqtt_throughput.rs`)
```
参数: --msg-size <bytes> --rate <msg/s> --duration <secs> --qos <0|1>
流程:
  1. 创建 TslinkClient，配置指定 QoS
  2. 启动发送循环，按指定速率发送属性上报
  3. 采集指标: 发送成功数、失败数、吞吐量、延迟直方图
  4. 输出结构化报告
```

#### 2.2 QoS 可靠性测试 (`examples/bench_qos_reliability.rs`)
```
流程:
  1. 启动两个 client: publisher + subscriber
  2. Publisher 发送 N 条带序号的消息
  3. Subscriber 统计收到的消息数和序号
  4. 计算丢包率、乱序率
  5. 分别测试 QoS=0 和 QoS=1
```

#### 2.3 IPC 大帧传输测试 (`examples/bench_ipc_frame.rs`)
```
参数: --width <px> --height <px> --frames <count>
流程:
  1. 生成模拟 4K YUV 帧数据 (3840x2160x1.5 ≈ 12.4MB)
  2. 设备 A 通过 IPC 发布帧
  3. 设备 B 通过 IPC 订阅帧
  4. 测量: 单帧发布耗时、端到端延迟、帧率
```

#### 2.4 新 API 功能验证 (`examples/bench_api_validation.rs`)
```
流程:
  1. 创建 TslinkClient
  2. 逐一调用所有新 API 接口
  3. 验证: 无 panic、返回值正确、回调触发
  4. 输出通过/失败报告
```

### Phase 3: 统计与报告

- 使用内置 `std::time::Instant` 计时
- 延迟分位数使用排序数组计算 P50/P95/P99
- 内存占用通过 `jemalloc` 或 `/proc/self/status` 采集（macOS 用 `mach_task_basic_info`）
- 输出格式: 结构化文本 + 可选 JSON

## Dependencies

| 新增 dev-dependency | 用途 |
|---|---|
| `clap` | 命令行参数解析 |
| `hdrhistogram` (可选) | 延迟直方图 |

## Risks

| 风险 | 影响 | 缓解 |
|---|---|---|
| iceoryx2 不支持 16MB payload | IPC 大帧测试无法进行 | 分片传输或降低分辨率 |
| QoS=1 导致 rumqttc 背压 | 高频发送阻塞 | 增大 channel buffer |
| macOS 上 iceoryx2 共享内存限制 | IPC 测试失败 | 调整 sysctl 参数 |

## Success Criteria

1. `cargo run --example bench_mqtt_throughput` 持续 60s 无 panic
2. QoS 可靠性测试输出丢包率对比数据
3. IPC 大帧测试达到 ≥ 10fps
4. 全部新 API 功能验证通过
