package tslink

import "context"

// MessageChannel is the transport-layer abstraction (MQTT, IPC, etc.).
type MessageChannel interface {
	// Send publishes data to the given topic.
	Send(ctx context.Context, topic, data string) error
	// Start establishes the underlying connection.
	Start(ctx context.Context) error
	// Stop closes the connection and releases resources.
	Stop(ctx context.Context) error
	// AddTopic subscribes to an additional topic at runtime.
	AddTopic(ctx context.Context, topic string) error
	// IsConnected returns the current connectivity status.
	IsConnected() bool
}

// MessageReceiveCallback is invoked when a message is received on a channel.
type MessageReceiveCallback interface {
	Receive(topic, data string)
}

// MessageReceiveFunc adapts a plain function to MessageReceiveCallback.
type MessageReceiveFunc func(topic, data string)

func (f MessageReceiveFunc) Receive(topic, data string) { f(topic, data) }
