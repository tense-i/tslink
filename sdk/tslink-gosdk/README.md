# tslink-gosdk

[![Go Reference](https://pkg.go.dev/badge/github.com/chingchi/tslink-gosdk.svg)](https://pkg.go.dev/github.com/chingchi/tslink-gosdk)
![Go Version](https://img.shields.io/badge/go-%3E%3D1.21-blue)
![Tests](https://img.shields.io/badge/tests-37%20pass-brightgreen)

IoT Device SDK for Go — API-compatible with [tslink-rsdk](../tslink-rsdk/) (Rust SDK).

## Features

- **MQTT Channel** — paho.mqtt.golang, auto-reconnect, QoS 0/1/2
- **IPC Channel** — placeholder for future zero-copy transport (iceoryx2)
- **Property Reporting** — `ThingPropertyPost` / `ThingPropertyPostFor` (proxy)
- **Event Reporting** — info / warning / error events
- **Service Handling** — register device service executors (unified or per-identifier)
- **Platform Service** — sync (with timeout) & async invocation + callback
- **Device Discovery** — local device table with timeout cleanup
- **Multi-Channel** — route messages to MQTT / IPC / All

## Install

```bash
go get github.com/chingchi/tslink-gosdk@latest
```

## Quick Start

```go
package main

import (
    "context"
    "log"

    tslink "github.com/chingchi/tslink-gosdk"
)

func main() {
    client, err := tslink.NewTslinkClientBuilder().
        Endpoint("mqtt://broker:1883").
        ProductKey("your_pk").
        DeviceID("your_did").
        Username("your_username").
        Password("your_password").
        Build()
    if err != nil {
        log.Fatal(err)
    }

    ctx := context.Background()
    if err := client.Start(ctx); err != nil {
        log.Fatal(err)
    }
    defer client.Release(ctx)

    // Report property
    client.ThingPropertyPost(ctx, map[string]any{
        "temperature": 25.5,
    })
}
```

## Project Structure

```
tslink-gosdk/
├── doc.go              # Package documentation (godoc)
├── version.go          # SDK version constant
├── config.go           # MqttConfig configuration
├── client.go           # TslinkClient interface (public API)
├── client_default.go   # DefaultTslinkClient implementation
├── builder.go          # TslinkClientBuilder (fluent API)
├── channel.go          # MessageChannel interface
├── channel_mqtt.go     # MQTT transport (paho.mqtt.golang)
├── channel_ipc.go      # IPC placeholder
├── channel_multi.go    # Multi-channel router
├── adapter.go          # MessageAdapter (message routing)
├── discovery.go        # DeviceDiscovery service
├── enums.go            # EventType, QoS, CommunicationChannel
├── errors.go           # Error types & sentinel errors
├── message.go          # CommonMessage + Builder
├── reply.go            # ReplyMessage
├── service.go          # Service request/response/callback types
├── example_test.go     # Testable examples (godoc)
├── *_test.go           # Unit tests (37 total)
├── Makefile            # Dev commands (test/lint/cover/build)
├── .golangci.yml       # Linter configuration
├── CHANGELOG.md        # Version history
├── examples/
│   ├── mqtt_demo/      # Full MQTT demo
│   └── service_invoke/ # Service registration demo
└── specs/
    └── SPEC-005/       # SDD artifacts
```

## API Overview

### TslinkClient Interface

| Method | Description |
|---|---|
| `ThingPropertyPost` | 上报设备属性 |
| `ThingPropertyPostFor` | 代理模式上报指定设备属性 |
| `ThingEventPost` | 上报设备事件 (info/warning/error) |
| `SetPlatformPushUnifiedExecutor` | 注册统一平台推送处理器 |
| `SetPlatformPushSpecificExecutor` | 注册指定平台推送处理器 |
| `PlatformServiceInvokeSync` | 同步调用平台服务 |
| `PlatformServiceInvokeAsync` | 异步调用平台服务 |
| `SetServiceUnifiedExecutor` | 注册统一设备服务处理器 |
| `SetServiceSpecificExecutor` | 注册指定设备服务处理器 |
| `DeviceServiceInvokeSync` | 同步调用设备服务 |
| `DeviceServiceInvokeAsync` | 异步调用设备服务 |
| `SetPropertySetExecutor` | 注册属性设置处理器 |
| `Start` / `Release` | 生命周期管理 |
| `ThingPropertyPostWithChannel` | 指定通道上报属性 |
| `ThingEventPostWithChannel` | 指定通道上报事件 |
| `GetChannel` | 获取当前通道类型 |

### Message Types

| Type | Description |
|---|---|
| `CommonMessage` | IoT 协议通用消息 (tid/bid/method/data) |
| `ReplyMessage` | 平台回复消息 (code/message/data) |
| `PlatformServiceRequest/Response` | 平台服务请求/响应 |
| `DeviceServiceRequest/Response` | 设备服务请求/响应 |

### Channel Types

| Type | Description |
|---|---|
| `MqttChannel` | MQTT 传输实现 |
| `IpcChannel` | IPC 占位 (未实现) |
| `MultiChannel` | 多通道路由 |

## Development

```bash
# Run tests
make test

# Run tests with coverage
make cover

# Lint (requires golangci-lint)
make lint

# Build check
make build

# All CI checks
make ci
```

## Running Examples

```bash
# MQTT Demo
go run ./examples/mqtt_demo

# Service Invoke
go run ./examples/service_invoke
```

## Architecture

```
┌─────────────────────────────────┐
│      TslinkClient (interface)   │  ← Public API
├─────────────────────────────────┤
│   DefaultTslinkClient           │  ← Default implementation
│   ┌───────────────────────────┐ │
│   │  MessageAdapter           │ │  ← Message routing + callback dispatch
│   └───────────────────────────┘ │
│   ┌───────────────────────────┐ │
│   │  MessageChannel           │ │  ← Transport abstraction
│   │  ├── MqttChannel          │ │  ← MQTT transport
│   │  ├── IpcChannel           │ │  ← IPC placeholder
│   │  └── MultiChannel         │ │  ← Channel router
│   └───────────────────────────┘ │
└─────────────────────────────────┘

Topic format: sys/{productKey}/{deviceId}/thing/...
```

## License

MIT
