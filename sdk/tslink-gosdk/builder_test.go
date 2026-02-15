package tslink

import "testing"

func TestBuilderMissingEndpoint(t *testing.T) {
	_, err := NewTslinkClientBuilder().
		ProductKey("pk").
		DeviceID("did").
		Username("user").
		Password("pass").
		Build()
	if err == nil {
		t.Fatal("expected error for missing endpoint")
	}
	if e, ok := err.(*Error); !ok || e.Category != "configuration" {
		t.Errorf("unexpected error: %v", err)
	}
}

func TestBuilderMissingProductKey(t *testing.T) {
	_, err := NewTslinkClientBuilder().
		Endpoint("mqtt://localhost:1883").
		DeviceID("did").
		Username("user").
		Password("pass").
		Build()
	if err == nil {
		t.Fatal("expected error for missing product_key")
	}
}

func TestBuilderSuccess(t *testing.T) {
	client, err := NewTslinkClientBuilder().
		Endpoint("mqtt://localhost:1883").
		ProductKey("pk").
		DeviceID("did").
		Username("user").
		Password("pass").
		Build()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if client == nil {
		t.Fatal("client should not be nil")
	}
}

func TestBuilderWithQoS(t *testing.T) {
	client, err := NewTslinkClientBuilder().
		Endpoint("mqtt://localhost:1883").
		ProductKey("pk").
		DeviceID("did").
		Username("user").
		Password("pass").
		PublishQoS(QoSAtLeastOnce).
		SubscribeQoS(QoSExactlyOnce).
		Build()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if client == nil {
		t.Fatal("client should not be nil")
	}
}
