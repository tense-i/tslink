package tslink

import "context"

// TslinkClient defines the device-side IoT SDK interface.
// API-aligned with tslink-rsdk (Rust) TslinkClient trait.
type TslinkClient interface {
	// ==================== Property ====================

	// ThingPropertyPost reports device properties to cloud.
	ThingPropertyPost(ctx context.Context, data interface{}) error

	// ThingPropertyPostFor reports properties for a specific device (proxy mode).
	ThingPropertyPostFor(ctx context.Context, productKey, deviceID string, data interface{}) error

	// ==================== Event ====================

	// ThingEventPost reports a device event to cloud.
	ThingEventPost(ctx context.Context, eventType EventType, eventName string, data interface{}) error

	// ==================== Platform Push Executor ====================

	// SetPlatformPushUnifiedExecutor registers a unified executor for all platform push services.
	SetPlatformPushUnifiedExecutor(executor ServiceExecutor, productKey, deviceID string)

	// SetPlatformPushSpecificExecutor registers a specific executor for a named platform push service.
	SetPlatformPushSpecificExecutor(identifier string, executor ServiceExecutor, productKey, deviceID string)

	// ==================== Platform Service Invoke ====================

	// PlatformServiceInvokeSync invokes a platform service synchronously (with timeout).
	PlatformServiceInvokeSync(ctx context.Context, request *PlatformServiceRequest, timeoutMs int) (*PlatformServiceResponse, error)

	// PlatformServiceInvokeAsync invokes a platform service asynchronously (with callback).
	PlatformServiceInvokeAsync(ctx context.Context, request *PlatformServiceRequest, callback PlatformResponseCallback) error

	// ==================== Device Service Executor ====================

	// SetServiceUnifiedExecutor registers a unified executor for all device service invocations.
	SetServiceUnifiedExecutor(executor ServiceExecutor, channel CommunicationChannel, productKey, deviceID string)

	// SetServiceSpecificExecutor registers a specific executor for a named device service.
	SetServiceSpecificExecutor(identifier string, executor ServiceExecutor, channel CommunicationChannel, productKey, deviceID string)

	// ==================== Device Service Invoke ====================

	// DeviceServiceInvokeSync invokes a device service synchronously (with timeout).
	DeviceServiceInvokeSync(ctx context.Context, request *DeviceServiceRequest, productKey, deviceID string, timeoutMs int) (*DeviceServiceResponse, error)

	// DeviceServiceInvokeAsync invokes a device service asynchronously (with callback).
	DeviceServiceInvokeAsync(ctx context.Context, request *DeviceServiceRequest, productKey, deviceID string, callback ServiceResponseCallback) error

	// ==================== Property Set Handler ====================

	// SetPropertySetExecutor registers a handler for property set commands.
	SetPropertySetExecutor(executor ServiceExecutor)

	// ==================== Lifecycle ====================

	// Start the client and establish connection.
	Start(ctx context.Context) error

	// Release resources and disconnect.
	Release(ctx context.Context) error

	// ==================== Multi-Channel ====================

	// ThingPropertyPostWithChannel reports properties using a specified channel.
	ThingPropertyPostWithChannel(ctx context.Context, data interface{}, channel CommunicationChannel) error

	// ThingEventPostWithChannel reports an event using a specified channel.
	ThingEventPostWithChannel(ctx context.Context, eventType EventType, eventName string, data interface{}, channel CommunicationChannel) error

	// GetChannel returns the current communication channel configuration.
	GetChannel() CommunicationChannel
}
