package tslink

// EventType represents IoT thing event types.
type EventType string

const (
	EventTypeInfo    EventType = "info"
	EventTypeWarning EventType = "warning"
	EventTypeError   EventType = "error"
)

// String returns the string representation of EventType.
func (e EventType) String() string { return string(e) }

// QoS represents MQTT quality-of-service levels.
type QoS byte

const (
	QoSAtMostOnce  QoS = 0
	QoSAtLeastOnce QoS = 1
	QoSExactlyOnce QoS = 2
)

// CommunicationChannel selects which transport channel(s) to use.
type CommunicationChannel int

const (
	// ChannelAll sends via all available channels.
	ChannelAll CommunicationChannel = iota
	// ChannelRemote sends via MQTT (remote) channel only.
	ChannelRemote
	// ChannelIPC sends via IPC channel only.
	ChannelIPC
)

// String returns human-readable channel name.
func (c CommunicationChannel) String() string {
	switch c {
	case ChannelAll:
		return "all"
	case ChannelRemote:
		return "remote"
	case ChannelIPC:
		return "ipc"
	default:
		return "unknown"
	}
}

// IncludesMQTT returns true if this channel includes MQTT transport.
func (c CommunicationChannel) IncludesMQTT() bool {
	return c == ChannelAll || c == ChannelRemote
}

// IncludesIPC returns true if this channel includes IPC transport.
func (c CommunicationChannel) IncludesIPC() bool {
	return c == ChannelAll || c == ChannelIPC
}
