// Package tslink provides an IoT device SDK for Go, enabling devices to
// communicate with the TSLink IoT platform via MQTT (and future IPC) channels.
//
// # Overview
//
// tslink-gosdk is the Go implementation of the TSLink Device SDK, API-compatible
// with the Rust SDK (tslink-rsdk). It supports:
//
//   - Property reporting (thing.property.post)
//   - Event reporting (thing.event.*.info / warning / error)
//   - Platform service invocation (sync & async)
//   - Device service registration & execution
//   - Property set command handling
//   - Multi-channel transport (MQTT now, IPC placeholder)
//   - Local device discovery (via IPC broadcast, placeholder)
//
// # Quick Start
//
//	client, err := tslink.NewTslinkClientBuilder().
//	    Endpoint("mqtt://broker:1883").
//	    ProductKey("your_pk").
//	    DeviceID("your_device").
//	    Username("user").
//	    Password("pass").
//	    Build()
//	if err != nil {
//	    log.Fatal(err)
//	}
//
//	ctx := context.Background()
//	if err := client.Start(ctx); err != nil {
//	    log.Fatal(err)
//	}
//	defer client.Release(ctx)
//
//	// Report properties
//	client.ThingPropertyPost(ctx, map[string]any{"temperature": 25.5})
//
//	// Report events
//	client.ThingEventPost(ctx, tslink.EventTypeInfo, "boot", map[string]any{"version": "1.0"})
//
// # Architecture
//
// The SDK is layered as follows:
//
//	TslinkClient (interface)
//	  └── DefaultTslinkClient
//	        ├── MessageAdapter   — routes incoming messages to executors/reply handlers
//	        └── MessageChannel   — transport abstraction
//	              ├── MqttChannel   (paho.mqtt.golang)
//	              ├── IpcChannel    (placeholder)
//	              └── MultiChannel  (router)
//
// # Topic Convention
//
// All MQTT topics follow the pattern:
//
//	sys/{productKey}/{deviceId}/thing/event/{eventName}/{eventType}
//	sys/{productKey}/{deviceId}/thing/service/{identifier}/post
//	sys/{productKey}/{deviceId}/platform/service/{identifier}/post
//
// # Thread Safety
//
// All exported types are safe for concurrent use.
package tslink
