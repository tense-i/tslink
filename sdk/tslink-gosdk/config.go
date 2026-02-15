package tslink

// MqttConfig holds MQTT channel configuration.
type MqttConfig struct {
	// Endpoint is the MQTT broker address (e.g. "mqtt://broker:1883" or "tcp://broker:1883").
	Endpoint string

	// ProductKey identifies the product (used in topic construction).
	ProductKey string

	// DeviceID identifies the device (used in topic construction and client ID).
	DeviceID string

	// Username for MQTT authentication.
	Username string

	// Password for MQTT authentication.
	Password string

	// KeepAliveSecs is the MQTT keep-alive interval in seconds. Default: 20.
	KeepAliveSecs uint16

	// ConnectionTimeoutSec is the connection attempt timeout in seconds. Default: 10.
	ConnectionTimeoutSec uint16

	// MaxInflight is the maximum number of in-flight QoS 1/2 messages. Default: 1000.
	MaxInflight int

	// PublishQoS is the default QoS for publishing. Default: QoSAtMostOnce.
	PublishQoS QoS

	// SubscribeQoS is the default QoS for subscriptions. Default: QoSAtMostOnce.
	SubscribeQoS QoS
}

// DefaultMqttConfig returns a MqttConfig with sensible defaults.
func DefaultMqttConfig() MqttConfig {
	return MqttConfig{
		KeepAliveSecs:        20,
		ConnectionTimeoutSec: 10,
		MaxInflight:          1000,
		PublishQoS:           QoSAtMostOnce,
		SubscribeQoS:         QoSAtMostOnce,
	}
}
