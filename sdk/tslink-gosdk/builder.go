package tslink

// TslinkClientBuilder provides a fluent API for constructing a TslinkClient.
type TslinkClientBuilder struct {
	endpoint     string
	productKey   string
	deviceID     string
	deviceSecret string
	username     string
	password     string
	publishQoS   QoS
	subscribeQoS QoS
}

// NewTslinkClientBuilder creates a new builder with default QoS settings.
func NewTslinkClientBuilder() *TslinkClientBuilder {
	return &TslinkClientBuilder{
		publishQoS:   QoSAtMostOnce,
		subscribeQoS: QoSAtMostOnce,
	}
}

func (b *TslinkClientBuilder) Endpoint(v string) *TslinkClientBuilder {
	b.endpoint = v
	return b
}

func (b *TslinkClientBuilder) ProductKey(v string) *TslinkClientBuilder {
	b.productKey = v
	return b
}

func (b *TslinkClientBuilder) DeviceID(v string) *TslinkClientBuilder {
	b.deviceID = v
	return b
}

func (b *TslinkClientBuilder) DeviceSecret(v string) *TslinkClientBuilder {
	b.deviceSecret = v
	return b
}

func (b *TslinkClientBuilder) Username(v string) *TslinkClientBuilder {
	b.username = v
	return b
}

func (b *TslinkClientBuilder) Password(v string) *TslinkClientBuilder {
	b.password = v
	return b
}

func (b *TslinkClientBuilder) PublishQoS(q QoS) *TslinkClientBuilder {
	b.publishQoS = q
	return b
}

func (b *TslinkClientBuilder) SubscribeQoS(q QoS) *TslinkClientBuilder {
	b.subscribeQoS = q
	return b
}

// Build validates configuration and returns a DefaultTslinkClient (as TslinkClient interface).
func (b *TslinkClientBuilder) Build() (TslinkClient, error) {
	if b.endpoint == "" {
		return nil, errConfiguration("endpoint is required")
	}
	if b.productKey == "" {
		return nil, errConfiguration("product_key is required")
	}
	if b.deviceID == "" {
		return nil, errConfiguration("device_id is required")
	}
	if b.username == "" {
		return nil, errConfiguration("username is required")
	}
	if b.password == "" {
		return nil, errConfiguration("password is required")
	}

	client := newDefaultTslinkClient(
		b.endpoint,
		b.productKey,
		b.deviceID,
		b.username,
		b.password,
		b.publishQoS,
		b.subscribeQoS,
	)
	return client, nil
}
