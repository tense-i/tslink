package tslink

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"time"
)

// DefaultTslinkClient is the standard implementation of TslinkClient.
type DefaultTslinkClient struct {
	productKey string
	deviceID   string
	channel    *MqttChannel
	adapter    *MessageAdapter
}

// newDefaultTslinkClient is called by the builder.
func newDefaultTslinkClient(
	endpoint, productKey, deviceID, username, password string,
	pubQoS, subQoS QoS,
) *DefaultTslinkClient {
	adapter := NewMessageAdapter()
	config := MqttConfig{
		Endpoint:             endpoint,
		ProductKey:           productKey,
		DeviceID:             deviceID,
		Username:             username,
		Password:             password,
		KeepAliveSecs:        20,
		ConnectionTimeoutSec: 10,
		MaxInflight:          1000,
		PublishQoS:           pubQoS,
		SubscribeQoS:         subQoS,
	}
	ch := NewMqttChannel(config, adapter)

	// Wire the reply sender so adapter can publish replies.
	adapter.SetReplySender(func(topic, data string) {
		if err := ch.Send(context.Background(), topic, data); err != nil {
			log.Printf("[tslink] reply send error: %v", err)
		}
	})

	return &DefaultTslinkClient{
		productKey: productKey,
		deviceID:   deviceID,
		channel:    ch,
		adapter:    adapter,
	}
}

// ---------------------------------------------------------------------------
// Topic helpers
// ---------------------------------------------------------------------------

func (c *DefaultTslinkClient) propertyTopic() string {
	return fmt.Sprintf("sys/%s/%s/thing/event/property/post", c.productKey, c.deviceID)
}

func (c *DefaultTslinkClient) propertyTopicFor(pk, did string) string {
	return fmt.Sprintf("sys/%s/%s/thing/event/property/post", pk, did)
}

func (c *DefaultTslinkClient) eventTopic(eventName string, eventType EventType) string {
	return fmt.Sprintf("sys/%s/%s/thing/event/%s/%s", c.productKey, c.deviceID, eventName, eventType)
}

func (c *DefaultTslinkClient) platformServiceTopic(identity, pk, did string) string {
	return fmt.Sprintf("sys/%s/%s/platform/service/%s/post", pk, did, identity)
}

func (c *DefaultTslinkClient) deviceServiceTopic(identity, pk, did string) string {
	return fmt.Sprintf("sys/%s/%s/thing/service/%s/post", pk, did, identity)
}

// ---------------------------------------------------------------------------
// TslinkClient implementation
// ---------------------------------------------------------------------------

func (c *DefaultTslinkClient) ThingPropertyPost(ctx context.Context, data interface{}) error {
	msg, err := NewCommonMessage("event.property.post", data)
	if err != nil {
		return err
	}
	jsonStr, err := msg.ToJSON()
	if err != nil {
		return err
	}
	return c.channel.Send(ctx, c.propertyTopic(), jsonStr)
}

func (c *DefaultTslinkClient) ThingPropertyPostFor(ctx context.Context, productKey, deviceID string, data interface{}) error {
	msg, err := NewCommonMessage("event.property.post", data)
	if err != nil {
		return err
	}
	jsonStr, err := msg.ToJSON()
	if err != nil {
		return err
	}
	return c.channel.Send(ctx, c.propertyTopicFor(productKey, deviceID), jsonStr)
}

func (c *DefaultTslinkClient) ThingEventPost(ctx context.Context, eventType EventType, eventName string, data interface{}) error {
	method := fmt.Sprintf("event.%s.%s", eventName, eventType)
	msg, err := NewCommonMessage(method, data)
	if err != nil {
		return err
	}
	jsonStr, err := msg.ToJSON()
	if err != nil {
		return err
	}
	return c.channel.Send(ctx, c.eventTopic(eventName, eventType), jsonStr)
}

func (c *DefaultTslinkClient) SetPlatformPushUnifiedExecutor(executor ServiceExecutor, _ string, _ string) {
	c.adapter.SetUnifiedExecutor(executor)
}

func (c *DefaultTslinkClient) SetPlatformPushSpecificExecutor(identifier string, executor ServiceExecutor, _ string, _ string) {
	method := fmt.Sprintf("platform.service.%s.post", identifier)
	c.adapter.AddServiceExecutor(method, executor)
}

func (c *DefaultTslinkClient) PlatformServiceInvokeSync(ctx context.Context, request *PlatformServiceRequest, timeoutMs int) (*PlatformServiceResponse, error) {
	pk := c.productKey
	if request.ProductKey != "" {
		pk = request.ProductKey
	}
	did := c.deviceID
	if request.DeviceID != "" {
		did = request.DeviceID
	}

	method := fmt.Sprintf("platform.service.%s.post", request.ServiceIdentifier)
	var dataVal interface{}
	_ = json.Unmarshal(request.ParamData, &dataVal)

	msg, err := NewCommonMessage(method, dataVal)
	if err != nil {
		return nil, err
	}

	// Register reply handler
	ch := make(chan *PlatformServiceResponse, 1)
	c.adapter.AddReplyHandler(msg.TID, &ReplyHandler{
		IsPlatform: true,
		PlatformCb: func(resp *PlatformServiceResponse) {
			select {
			case ch <- resp:
			default:
			}
		},
	})

	jsonStr, err := msg.ToJSON()
	if err != nil {
		return nil, err
	}
	topic := c.platformServiceTopic(request.ServiceIdentifier, pk, did)
	if err := c.channel.Send(ctx, topic, jsonStr); err != nil {
		c.adapter.RemoveReplyHandler(msg.TID)
		return nil, err
	}

	timeout := time.Duration(timeoutMs) * time.Millisecond
	if timeout <= 0 {
		timeout = 5 * time.Second
	}
	select {
	case resp := <-ch:
		return resp, nil
	case <-time.After(timeout):
		c.adapter.RemoveReplyHandler(msg.TID)
		return nil, errTimeout(fmt.Sprintf("platform service invoke timeout after %dms", timeoutMs))
	case <-ctx.Done():
		c.adapter.RemoveReplyHandler(msg.TID)
		return nil, ctx.Err()
	}
}

func (c *DefaultTslinkClient) PlatformServiceInvokeAsync(ctx context.Context, request *PlatformServiceRequest, callback PlatformResponseCallback) error {
	pk := c.productKey
	if request.ProductKey != "" {
		pk = request.ProductKey
	}
	did := c.deviceID
	if request.DeviceID != "" {
		did = request.DeviceID
	}

	method := fmt.Sprintf("platform.service.%s.post", request.ServiceIdentifier)
	var dataVal interface{}
	_ = json.Unmarshal(request.ParamData, &dataVal)

	msg, err := NewCommonMessage(method, dataVal)
	if err != nil {
		return err
	}

	c.adapter.AddReplyHandler(msg.TID, &ReplyHandler{
		IsPlatform: true,
		PlatformCb: callback,
	})

	jsonStr, err := msg.ToJSON()
	if err != nil {
		return err
	}
	topic := c.platformServiceTopic(request.ServiceIdentifier, pk, did)
	return c.channel.Send(ctx, topic, jsonStr)
}

func (c *DefaultTslinkClient) SetServiceUnifiedExecutor(executor ServiceExecutor, _ CommunicationChannel, _ string, _ string) {
	c.adapter.SetUnifiedExecutor(executor)
}

func (c *DefaultTslinkClient) SetServiceSpecificExecutor(identifier string, executor ServiceExecutor, _ CommunicationChannel, _ string, _ string) {
	method := fmt.Sprintf("service.%s.post", identifier)
	c.adapter.AddServiceExecutor(method, executor)
}

func (c *DefaultTslinkClient) DeviceServiceInvokeSync(ctx context.Context, request *DeviceServiceRequest, productKey, deviceID string, timeoutMs int) (*DeviceServiceResponse, error) {
	method := fmt.Sprintf("service.%s.post", request.ServiceIdentifier)
	var dataVal interface{}
	_ = json.Unmarshal(request.ParamData, &dataVal)

	msg, err := NewCommonMessage(method, dataVal)
	if err != nil {
		return nil, err
	}

	ch := make(chan *DeviceServiceResponse, 1)
	c.adapter.AddReplyHandler(msg.TID, &ReplyHandler{
		IsPlatform: false,
		DeviceCb: func(resp *DeviceServiceResponse) {
			select {
			case ch <- resp:
			default:
			}
		},
	})

	jsonStr, err := msg.ToJSON()
	if err != nil {
		return nil, err
	}
	topic := c.deviceServiceTopic(request.ServiceIdentifier, productKey, deviceID)
	if err := c.channel.Send(ctx, topic, jsonStr); err != nil {
		c.adapter.RemoveReplyHandler(msg.TID)
		return nil, err
	}

	timeout := time.Duration(timeoutMs) * time.Millisecond
	if timeout <= 0 {
		timeout = 5 * time.Second
	}
	select {
	case resp := <-ch:
		return resp, nil
	case <-time.After(timeout):
		c.adapter.RemoveReplyHandler(msg.TID)
		return nil, errTimeout(fmt.Sprintf("device service invoke timeout after %dms", timeoutMs))
	case <-ctx.Done():
		c.adapter.RemoveReplyHandler(msg.TID)
		return nil, ctx.Err()
	}
}

func (c *DefaultTslinkClient) DeviceServiceInvokeAsync(ctx context.Context, request *DeviceServiceRequest, productKey, deviceID string, callback ServiceResponseCallback) error {
	method := fmt.Sprintf("service.%s.post", request.ServiceIdentifier)
	var dataVal interface{}
	_ = json.Unmarshal(request.ParamData, &dataVal)

	msg, err := NewCommonMessage(method, dataVal)
	if err != nil {
		return err
	}

	c.adapter.AddReplyHandler(msg.TID, &ReplyHandler{
		IsPlatform: false,
		DeviceCb:   callback,
	})

	jsonStr, err := msg.ToJSON()
	if err != nil {
		return err
	}
	topic := c.deviceServiceTopic(request.ServiceIdentifier, productKey, deviceID)
	return c.channel.Send(ctx, topic, jsonStr)
}

func (c *DefaultTslinkClient) SetPropertySetExecutor(executor ServiceExecutor) {
	c.adapter.AddServiceExecutor("thing.properties.set", executor)
	c.adapter.AddServiceExecutor("service.property.set", executor)
}

func (c *DefaultTslinkClient) Start(ctx context.Context) error {
	return c.channel.Start(ctx)
}

func (c *DefaultTslinkClient) Release(ctx context.Context) error {
	c.adapter.Release()
	return c.channel.Stop(ctx)
}

// ThingPropertyPostWithChannel reports properties via a specified channel.
// In the current MQTT-only build, the channel arg is ignored.
func (c *DefaultTslinkClient) ThingPropertyPostWithChannel(ctx context.Context, data interface{}, _ CommunicationChannel) error {
	return c.ThingPropertyPost(ctx, data)
}

// ThingEventPostWithChannel reports an event via a specified channel.
func (c *DefaultTslinkClient) ThingEventPostWithChannel(ctx context.Context, eventType EventType, eventName string, data interface{}, _ CommunicationChannel) error {
	return c.ThingEventPost(ctx, eventType, eventName, data)
}

// GetChannel returns the current channel type (always Remote for MQTT-only).
func (c *DefaultTslinkClient) GetChannel() CommunicationChannel {
	return ChannelRemote
}

// Ensure DefaultTslinkClient implements TslinkClient.
var _ TslinkClient = (*DefaultTslinkClient)(nil)
