// mqtt_demo demonstrates all major tslink-gosdk capabilities:
// - Property reporting
// - Event reporting
// - Service callback registration
// - Platform service invocation
//
// Run: go run ./examples/mqtt_demo
package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"time"

	tslink "github.com/chingchi/tslink-gosdk"
)

func main() {
	log.SetFlags(log.Ltime | log.Lmicroseconds)
	log.Println("=== tslink-gosdk MQTT Demo ===")

	endpoint := envOr("MQTT_ENDPOINT", "mqtt://localhost:1883")
	productKey := envOr("PRODUCT_KEY", "test_product")
	deviceID := envOr("DEVICE_ID", "test_device_001")
	username := envOr("MQTT_USERNAME", "device")
	password := envOr("MQTT_PASSWORD", "device123")

	log.Printf("Connecting to MQTT broker: %s", endpoint)
	log.Printf("Product: %s, Device: %s", productKey, deviceID)

	client, err := tslink.NewTslinkClientBuilder().
		Endpoint(endpoint).
		ProductKey(productKey).
		DeviceID(deviceID).
		Username(username).
		Password(password).
		Build()
	if err != nil {
		log.Fatalf("Build error: %v", err)
	}

	ctx := context.Background()

	log.Println("Starting TslinkClient...")
	if err := client.Start(ctx); err != nil {
		log.Fatalf("Start error: %v", err)
	}

	time.Sleep(2 * time.Second)

	// Test 1: Property Reporting
	log.Println("\n=== Test 1: Property Reporting ===")
	testPropertyReporting(ctx, client)

	// Test 2: Event Reporting
	log.Println("\n=== Test 2: Event Reporting ===")
	testEventReporting(ctx, client)

	// Test 3: Service Callback
	log.Println("\n=== Test 3: Service Callback Registration ===")
	testServiceCallback(client, productKey, deviceID)

	// Test 4: Platform Service Invocation
	log.Println("\n=== Test 4: Platform Service Invocation ===")
	testPlatformService(ctx, client)

	// Wait for incoming messages
	log.Println("\n=== Waiting for incoming messages (10 seconds) ===")
	time.Sleep(10 * time.Second)

	log.Println("Releasing TslinkClient...")
	if err := client.Release(ctx); err != nil {
		log.Fatalf("Release error: %v", err)
	}

	log.Println("\n=== Demo completed successfully! ===")
}

func testPropertyReporting(ctx context.Context, client tslink.TslinkClient) {
	properties := map[string]interface{}{
		"temperature": 25.5,
		"humidity":    60,
		"status":      "online",
	}
	log.Printf("Reporting properties: %v", properties)
	if err := client.ThingPropertyPost(ctx, properties); err != nil {
		log.Printf("ERROR: %v", err)
		return
	}
	log.Println("✓ Properties reported successfully")

	batch := map[string]interface{}{
		"cpu_usage":      45.2,
		"memory_usage":   1024,
		"disk_free":      50000,
		"network_status": "connected",
	}
	log.Printf("Reporting batch properties: %v", batch)
	if err := client.ThingPropertyPost(ctx, batch); err != nil {
		log.Printf("ERROR: %v", err)
		return
	}
	log.Println("✓ Batch properties reported successfully")
}

func testEventReporting(ctx context.Context, client tslink.TslinkClient) {
	infoEvent := map[string]interface{}{
		"message": "Device started successfully",
		"version": "1.0.0",
	}
	log.Printf("Reporting INFO event: %v", infoEvent)
	if err := client.ThingEventPost(ctx, tslink.EventTypeInfo, "device_started", infoEvent); err != nil {
		log.Printf("ERROR: %v", err)
		return
	}
	log.Println("✓ INFO event reported successfully")

	warningEvent := map[string]interface{}{
		"warning":     "High temperature detected",
		"temperature": 85.5,
		"threshold":   80.0,
	}
	log.Printf("Reporting WARNING event: %v", warningEvent)
	if err := client.ThingEventPost(ctx, tslink.EventTypeWarning, "high_temperature", warningEvent); err != nil {
		log.Printf("ERROR: %v", err)
		return
	}
	log.Println("✓ WARNING event reported successfully")

	errorEvent := map[string]interface{}{
		"error":       "Sensor disconnected",
		"sensor_id":   "temp_001",
		"retry_count": 3,
	}
	log.Printf("Reporting ERROR event: %v", errorEvent)
	if err := client.ThingEventPost(ctx, tslink.EventTypeError, "sensor_error", errorEvent); err != nil {
		log.Printf("ERROR: %v", err)
		return
	}
	log.Println("✓ ERROR event reported successfully")
}

func testServiceCallback(client tslink.TslinkClient, pk, did string) {
	// Register unified service executor
	client.SetPlatformPushUnifiedExecutor(func(req *tslink.DeviceServiceRequest, reply tslink.ReplyCallback) {
		log.Printf("[Unified] Received service: %s, data: %s", req.ServiceIdentifier, string(req.ParamData))
		reply(0, []byte(`{"handled": true}`))
	}, pk, did)

	// Register specific service executor
	client.SetServiceSpecificExecutor("reboot", func(req *tslink.DeviceServiceRequest, reply tslink.ReplyCallback) {
		log.Printf("[Reboot] Received reboot service call, data: %s", string(req.ParamData))
		reply(0, []byte(`{"rebooting": true}`))
	}, tslink.ChannelRemote, pk, did)

	// Register property set handler
	client.SetPropertySetExecutor(func(req *tslink.DeviceServiceRequest, reply tslink.ReplyCallback) {
		log.Printf("[PropertySet] Setting properties: %s", string(req.ParamData))
		reply(0, nil)
	})

	log.Println("✓ Service executors registered successfully")
}

func testPlatformService(ctx context.Context, client tslink.TslinkClient) {
	req := tslink.NewPlatformServiceRequest("getConfig", []byte(`{"section":"network"}`))
	log.Println("Invoking platform service (async)...")
	err := client.PlatformServiceInvokeAsync(ctx, req, func(resp *tslink.PlatformServiceResponse) {
		log.Printf("[Async callback] result=%d, data=%s", resp.Result, string(resp.ParamData))
	})
	if err != nil {
		log.Printf("Async invoke error: %v", err)
	} else {
		log.Println("✓ Platform service invoked asynchronously")
	}
}

func envOr(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func init() {
	// suppress unused import warning for fmt
	_ = fmt.Sprintf
}
