<!-- Sync Impact Report
- Version change: N/A → 1.0.0
- Added sections: All (initial constitution)
- Templates requiring updates: ✅ All templates reviewed
- Follow-up TODOs: None
-->

# TSLink Constitution

## Core Principles

### I. IoT-Only Scope
TSLink MUST only implement IoT infrastructure concerns: MQTT communication, device lifecycle, thing model protocol, topic routing, device shadow, and message serialization. Business logic (task scheduling, media dispatch, algorithm management, 4A auth, WebSocket push) MUST NOT be included. Business concerns MUST be exposed only through well-defined event interfaces (Kafka/gRPC) for downstream consumers.

### II. Rust-First Performance
All core paths (MQTT message handling, topic routing, device shadow reads) MUST be implemented in Rust using async/await (tokio). Memory usage MUST stay below 200MB for 10K concurrent devices. P99 message processing latency MUST be under 100ms. Zero-copy deserialization SHOULD be used wherever possible via serde.

### III. Type-Safe Protocol
All MQTT message structures (CommonTopicReceiver, CommonTopicResponse, ThingModel) MUST be expressed as strongly-typed Rust structs with serde Deserialize/Serialize. No `Map<String, Object>` equivalents (serde_json::Value) in core domain types. Protocol errors MUST be caught at compile time, not runtime.

### IV. Clean Architecture
The codebase MUST follow hexagonal architecture: domain layer (entities, traits) has zero external dependencies; application layer orchestrates use cases; infrastructure layer implements adapters (MQTT, Redis, Kafka, Database). Dependency inversion MUST be enforced — domain defines trait interfaces, infrastructure provides implementations.

### V. Observability by Default
Every MQTT message MUST be traced with structured logging (tracing crate). Prometheus metrics MUST be exported for: message throughput, processing latency histograms, device online/offline counts, Redis operation latency, error rates. OpenTelemetry trace context MUST be propagated through message processing pipeline.

## Compatibility Requirements

- MUST maintain full compatibility with existing EMQX broker (V3 topic structure: `sys/{productKey}/{deviceId}/...`)
- MUST support the same CommonTopicReceiver/CommonTopicResponse JSON message format
- MUST be a drop-in replacement — existing devices and SDKs (JASmartSDK) MUST NOT require changes
- MUST support multi-link device communication with Redis-based link weight calculation
- V2 legacy topics ($SYS/*, DEVICE/*) MAY be supported as optional compatibility layer

## Development Workflow

- Rust edition 2021, MSRV 1.75+
- `cargo fmt` and `cargo clippy` MUST pass with zero warnings before merge
- Unit test coverage MUST exceed 80% for domain and application layers
- Integration tests MUST verify MQTT round-trip with EMQX testcontainer
- Benchmarks MUST be provided for hot paths (topic routing, message deserialization, Redis shadow ops)

## Governance

This constitution supersedes all other development practices for TSLink. Amendments require:
1. Written proposal documenting the change and rationale
2. Impact analysis on existing modules
3. Version bump following semver

**Version**: 1.0.0 | **Ratified**: 2026-02-13 | **Last Amended**: 2026-02-13
