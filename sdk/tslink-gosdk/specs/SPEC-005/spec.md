# SPEC-005: tslink-gosdk — Go IoT Device SDK

## 概述
基于 tslink-rsdk (Rust SDK) 的 API 规范，实现 Go 语言版本的 IoT 设备端 SDK。
IPC 通道作为占位实现（placeholder），不做实际 IPC 传输。

## 范围
- 完整复刻 tslink-rsdk 的 `TslinkClient` trait → Go `TslinkClient` interface
- MQTT 通道完整实现
- IPC 通道占位（所有方法返回 ErrIPCNotImplemented）
- MessageAdapter 消息路由 + 回调分发
- DeviceDiscovery 设备发现（表管理逻辑，无实际 IPC 广播）
- 单元测试 28+ 项
- 示例程序 2 个 (mqtt_demo, service_invoke)

## 验收标准
1. `go test -v ./...` 全部通过 (28 tests)
2. `go vet ./...` 无警告
3. `go build ./...` 编译成功（含 examples）
4. API 与 tslink-rsdk TslinkClient 1:1 对齐
5. IPC 通道为占位实现

## 技术栈
- Go 1.21+
- github.com/eclipse/paho.mqtt.golang v1.4.3
- github.com/google/uuid v1.6.0
