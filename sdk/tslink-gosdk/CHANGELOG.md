# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-15

### Added
- `TslinkClient` interface — 17 methods aligned with tslink-rsdk (Rust SDK)
- `TslinkClientBuilder` — fluent builder with 5-field validation
- `MqttChannel` — MQTT transport via paho.mqtt.golang v1.4.3
- `IpcChannel` — placeholder for future IPC (iceoryx2) transport
- `MultiChannel` — channel router (MQTT / IPC / All)
- `MessageAdapter` — incoming message routing to service executors & reply handlers
- `DeviceDiscovery` — local device discovery with timeout cleanup
- Message types: `CommonMessage`, `CommonMessageBuilder`, `ReplyMessage`
- Service types: `PlatformServiceRequest/Response`, `DeviceServiceRequest/Response`
- Enums: `EventType`, `QoS`, `CommunicationChannel`
- Error system: `Error` struct + sentinel errors (`ErrNotStarted`, `ErrAlreadyStarted`)
- 28 unit tests, 8 godoc examples
- MQTT demo and service invoke examples
