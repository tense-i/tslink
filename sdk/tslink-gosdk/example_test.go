package tslink_test

import (
	"context"
	"fmt"

	tslink "github.com/chingchi/tslink-gosdk"
)

// ExampleNewTslinkClientBuilder demonstrates how to create a TslinkClient
// using the builder pattern.
func ExampleNewTslinkClientBuilder() {
	client, err := tslink.NewTslinkClientBuilder().
		Endpoint("mqtt://broker:1883").
		ProductKey("test_product").
		DeviceID("device_001").
		Username("user").
		Password("pass").
		PublishQoS(tslink.QoSAtLeastOnce).
		Build()
	if err != nil {
		fmt.Printf("build error: %v\n", err)
		return
	}
	fmt.Printf("client created: %v\n", client != nil)
	// Output: client created: true
}

// ExampleNewCommonMessage demonstrates creating a standard IoT message.
func ExampleNewCommonMessage() {
	msg, err := tslink.NewCommonMessage("event.property.post", map[string]any{
		"temperature": 25.5,
		"humidity":    60,
	})
	if err != nil {
		fmt.Printf("error: %v\n", err)
		return
	}
	fmt.Println("method:", msg.Method)
	fmt.Println("version:", msg.Version)
	// Output:
	// method: event.property.post
	// version: 1.0
}

// ExampleCommonMessageBuilder demonstrates the fluent message builder.
func ExampleCommonMessageBuilder() {
	msg := tslink.NewCommonMessageBuilder().
		TID("custom-tid").
		Method("service.reboot.post").
		DataValue(map[string]string{"reason": "update"}).
		Build()
	fmt.Println("tid:", msg.TID)
	fmt.Println("method:", msg.Method)
	// Output:
	// tid: custom-tid
	// method: service.reboot.post
}

// ExampleNewReplySuccess demonstrates creating a success reply.
func ExampleNewReplySuccess() {
	reply := tslink.NewReplySuccess("tid-1", "bid-1")
	fmt.Println("success:", reply.IsSuccess())
	fmt.Println("code:", reply.Code)
	// Output:
	// success: true
	// code: 0
}

// ExampleNewReplyError demonstrates creating an error reply.
func ExampleNewReplyError() {
	reply := tslink.NewReplyError("tid-1", "bid-1", 500, "internal error")
	fmt.Println("success:", reply.IsSuccess())
	fmt.Println("code:", reply.Code)
	fmt.Println("message:", reply.Message)
	// Output:
	// success: false
	// code: 500
	// message: internal error
}

// ExampleNewPlatformServiceRequest demonstrates creating a platform service request.
func ExampleNewPlatformServiceRequest() {
	req := tslink.NewPlatformServiceRequest("ota_upgrade", []byte(`{"version":"2.0"}`)).
		WithDevice("product_a", "device_001").
		WithChannel(tslink.ChannelRemote)
	fmt.Println("service:", req.ServiceIdentifier)
	fmt.Println("pk:", req.ProductKey)
	fmt.Println("channel:", req.Channel)
	// Output:
	// service: ota_upgrade
	// pk: product_a
	// channel: remote
}

// ExampleDeviceDiscovery demonstrates the device discovery service.
func ExampleDeviceDiscovery() {
	config := tslink.DefaultDeviceDiscoveryConfig()
	config.ProductKey = "my_pk"
	config.DeviceID = "my_dev"

	discovery := tslink.NewDeviceDiscovery(config)
	discovery.HandleDiscoveryMessage(`{"product_key":"remote_pk","device_id":"remote_dev"}`)

	if discovery.IsDeviceOnline("remote_pk", "remote_dev") {
		fmt.Println("device found: remote_pk:remote_dev")
	}
	// Output: device found: remote_pk:remote_dev
}

// ExampleTslinkClient_ThingPropertyPost demonstrates property reporting.
// This example creates a client but does not connect to a broker.
func ExampleTslinkClient_ThingPropertyPost() {
	client, _ := tslink.NewTslinkClientBuilder().
		Endpoint("mqtt://broker:1883").
		ProductKey("test").
		DeviceID("dev01").
		Username("u").
		Password("p").
		Build()

	// Note: client.Start() would be needed before actual use.
	// This example only shows the API shape.
	_ = client
	ctx := context.Background()
	_ = ctx
	fmt.Println("client ready for ThingPropertyPost")
	// Output: client ready for ThingPropertyPost
}

// ExampleVersion shows the SDK version constant.
func ExampleVersion() {
	fmt.Println("tslink-gosdk version:", tslink.Version)
	// Output: tslink-gosdk version: 0.1.0
}
