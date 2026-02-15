package tslink

import (
	"context"
	"log"
)

// MultiChannel routes messages to MQTT and/or IPC channels based on
// the specified CommunicationChannel.
type MultiChannel struct {
	mqtt           *MqttChannel
	ipc            *IpcChannel
	defaultChannel CommunicationChannel
}

// NewMultiChannel creates a MultiChannel. Either channel may be nil.
func NewMultiChannel(mqttCh *MqttChannel, ipcCh *IpcChannel, defaultCh CommunicationChannel) *MultiChannel {
	return &MultiChannel{mqtt: mqttCh, ipc: ipcCh, defaultChannel: defaultCh}
}

// DefaultChannel returns the default routing channel.
func (m *MultiChannel) DefaultChannel() CommunicationChannel { return m.defaultChannel }

// MQTTAvailable returns true if the MQTT channel is up.
func (m *MultiChannel) MQTTAvailable() bool {
	return m.mqtt != nil && m.mqtt.IsConnected()
}

// IPCAvailable returns true if the IPC channel is up.
func (m *MultiChannel) IPCAvailable() bool {
	return m.ipc != nil && m.ipc.IsConnected()
}

// SendWithChannel sends via the specified channel type.
func (m *MultiChannel) SendWithChannel(ctx context.Context, topic, data string, ch CommunicationChannel) error {
	sent := false
	if ch.IncludesMQTT() && m.mqtt != nil && m.mqtt.IsConnected() {
		if err := m.mqtt.Send(ctx, topic, data); err != nil {
			return err
		}
		sent = true
	}
	if ch.IncludesIPC() && m.ipc != nil && m.ipc.IsConnected() {
		if err := m.ipc.Send(ctx, topic, data); err != nil {
			return err
		}
		sent = true
	}
	if !sent {
		log.Printf("[tslink] no channel available for topic=%s channel=%s", topic, ch)
	}
	return nil
}

// Implement MessageChannel on the default channel -------------------------

func (m *MultiChannel) Send(ctx context.Context, topic, data string) error {
	return m.SendWithChannel(ctx, topic, data, m.defaultChannel)
}

func (m *MultiChannel) Start(ctx context.Context) error {
	if m.mqtt != nil {
		if err := m.mqtt.Start(ctx); err != nil {
			return err
		}
	}
	if m.ipc != nil {
		if err := m.ipc.Start(ctx); err != nil {
			return err
		}
	}
	return nil
}

func (m *MultiChannel) Stop(ctx context.Context) error {
	if m.mqtt != nil {
		_ = m.mqtt.Stop(ctx)
	}
	if m.ipc != nil {
		_ = m.ipc.Stop(ctx)
	}
	return nil
}

func (m *MultiChannel) AddTopic(ctx context.Context, topic string) error {
	if m.mqtt != nil {
		if err := m.mqtt.AddTopic(ctx, topic); err != nil {
			return err
		}
	}
	return nil
}

func (m *MultiChannel) IsConnected() bool {
	if m.mqtt != nil && m.mqtt.IsConnected() {
		return true
	}
	if m.ipc != nil && m.ipc.IsConnected() {
		return true
	}
	return false
}
