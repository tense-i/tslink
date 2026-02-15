package tslink

import (
	"context"
	"fmt"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	mqtt "github.com/eclipse/paho.mqtt.golang"
)

// MqttChannel implements MessageChannel over MQTT.
type MqttChannel struct {
	config    MqttConfig
	client    mqtt.Client
	callback  MessageReceiveCallback
	topics    []string
	connected atomic.Bool
	mu        sync.Mutex
	started   bool
}

// NewMqttChannel constructs an MqttChannel.
func NewMqttChannel(config MqttConfig, callback MessageReceiveCallback) *MqttChannel {
	ch := &MqttChannel{
		config:   config,
		callback: callback,
		topics:   defaultTopics(config.ProductKey, config.DeviceID),
	}
	return ch
}

func defaultTopics(pk, did string) []string {
	return []string{
		fmt.Sprintf("sys/%s/%s/thing/properties/set", pk, did),
		fmt.Sprintf("sys/%s/%s/thing/service/+/post", pk, did),
		fmt.Sprintf("sys/%s/%s/thing/service/property/set", pk, did),
		fmt.Sprintf("sys/%s/%s/platform/service/+/post_reply", pk, did),
		fmt.Sprintf("sys/%s/%s/thing/event/+/info_reply", pk, did),
		fmt.Sprintf("sys/%s/%s/thing/event/+/warning_reply", pk, did),
		fmt.Sprintf("sys/%s/%s/thing/event/+/error_reply", pk, did),
	}
}

func (c *MqttChannel) createOptions() *mqtt.ClientOptions {
	endpoint := c.config.Endpoint
	if !strings.HasPrefix(endpoint, "tcp://") && !strings.HasPrefix(endpoint, "ssl://") {
		// mqtt://host:port → tcp://host:port
		endpoint = strings.Replace(endpoint, "mqtt://", "tcp://", 1)
	}

	clientID := fmt.Sprintf("DEVICE:%s", c.config.DeviceID)
	opts := mqtt.NewClientOptions().
		AddBroker(endpoint).
		SetClientID(clientID).
		SetUsername(c.config.Username).
		SetPassword(c.config.Password).
		SetKeepAlive(time.Duration(c.config.KeepAliveSecs) * time.Second).
		SetConnectTimeout(time.Duration(c.config.ConnectionTimeoutSec) * time.Second).
		SetMaxReconnectInterval(5 * time.Second).
		SetCleanSession(true).
		SetAutoReconnect(true).
		SetOrderMatters(false)

	opts.SetOnConnectHandler(func(_ mqtt.Client) {
		c.connected.Store(true)
		// Re-subscribe on reconnect
		c.subscribeAll()
	})
	opts.SetConnectionLostHandler(func(_ mqtt.Client, err error) {
		c.connected.Store(false)
	})
	opts.SetDefaultPublishHandler(func(_ mqtt.Client, msg mqtt.Message) {
		if c.callback != nil {
			c.callback.Receive(msg.Topic(), string(msg.Payload()))
		}
	})
	return opts
}

func (c *MqttChannel) subscribeAll() {
	if c.client == nil || !c.client.IsConnected() {
		return
	}
	subQoS := byte(c.config.SubscribeQoS)
	for _, topic := range c.topics {
		t := c.client.Subscribe(topic, subQoS, nil)
		t.Wait()
	}
}

// Send publishes data to the given topic.
func (c *MqttChannel) Send(_ context.Context, topic, data string) error {
	if c.client == nil || !c.client.IsConnected() {
		return ErrNotStarted
	}
	pubQoS := byte(c.config.PublishQoS)
	t := c.client.Publish(topic, pubQoS, false, data)
	t.Wait()
	if t.Error() != nil {
		return errMqttPublish(t.Error().Error())
	}
	return nil
}

// Start establishes the MQTT connection.
func (c *MqttChannel) Start(_ context.Context) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.started {
		return ErrAlreadyStarted
	}
	opts := c.createOptions()
	c.client = mqtt.NewClient(opts)
	if token := c.client.Connect(); token.Wait() && token.Error() != nil {
		return errMqttConnection(token.Error().Error())
	}
	c.started = true
	return nil
}

// Stop disconnects from the broker.
func (c *MqttChannel) Stop(_ context.Context) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.client != nil {
		c.client.Disconnect(250)
		c.client = nil
	}
	c.started = false
	c.connected.Store(false)
	return nil
}

// AddTopic subscribes to an additional topic at runtime.
func (c *MqttChannel) AddTopic(_ context.Context, topic string) error {
	c.topics = append(c.topics, topic)
	if c.client != nil && c.client.IsConnected() {
		subQoS := byte(c.config.SubscribeQoS)
		t := c.client.Subscribe(topic, subQoS, nil)
		t.Wait()
		if t.Error() != nil {
			return errMqttSubscribe(t.Error().Error())
		}
	}
	return nil
}

// IsConnected returns true when the MQTT client is connected.
func (c *MqttChannel) IsConnected() bool {
	return c.connected.Load()
}
