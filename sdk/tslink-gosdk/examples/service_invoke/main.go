// service_invoke_test demonstrates device service registration and invocation.
//
// Run: go run ./examples/service_invoke
package main

import (
	"context"
	"encoding/json"
	"log"
	"os"
	"time"

	tslink "github.com/chingchi/tslink-gosdk"
)

func main() {
	log.SetFlags(log.Ltime | log.Lmicroseconds)
	log.Println("=== tslink-gosdk Service Invoke Test ===")

	endpoint := envOr("MQTT_ENDPOINT", "mqtt://localhost:1883")
	productKey := envOr("PRODUCT_KEY", "test_product")
	deviceID := envOr("DEVICE_ID", "test_device_001")
	username := envOr("MQTT_USERNAME", "device")
	password := envOr("MQTT_PASSWORD", "device123")

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

	// Register device service handler
	client.SetServiceSpecificExecutor("getStatus", func(req *tslink.DeviceServiceRequest, reply tslink.ReplyCallback) {
		log.Printf("[Service] getStatus called, params=%s", string(req.ParamData))

		status := map[string]interface{}{
			"cpu_usage":      42.5,
			"memory_free_mb": 512,
			"uptime_secs":    86400,
			"version":        "1.0.0",
		}
		data, _ := json.Marshal(status)
		reply(0, data)
	}, tslink.ChannelRemote, productKey, deviceID)

	client.SetServiceSpecificExecutor("setConfig", func(req *tslink.DeviceServiceRequest, reply tslink.ReplyCallback) {
		log.Printf("[Service] setConfig called, params=%s", string(req.ParamData))
		reply(0, []byte(`{"applied": true}`))
	}, tslink.ChannelRemote, productKey, deviceID)

	// Property set handler
	client.SetPropertySetExecutor(func(req *tslink.DeviceServiceRequest, reply tslink.ReplyCallback) {
		log.Printf("[PropertySet] data=%s", string(req.ParamData))
		reply(0, nil)
	})

	log.Println("Starting client...")
	if err := client.Start(ctx); err != nil {
		log.Fatalf("Start error: %v", err)
	}

	log.Println("Client started. Waiting for service invocations...")
	log.Println("Registered handlers: getStatus, setConfig, property.set")

	time.Sleep(30 * time.Second)

	log.Println("Releasing client...")
	_ = client.Release(ctx)
	log.Println("Done.")
}

func envOr(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func init() {
	_ = os.Getenv // suppress lint
}
