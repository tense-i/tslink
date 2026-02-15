package tslink

import "testing"

func TestEventTypeString(t *testing.T) {
	tests := []struct {
		et   EventType
		want string
	}{
		{EventTypeInfo, "info"},
		{EventTypeWarning, "warning"},
		{EventTypeError, "error"},
	}
	for _, tt := range tests {
		if got := tt.et.String(); got != tt.want {
			t.Errorf("EventType.String() = %q, want %q", got, tt.want)
		}
	}
}

func TestCommunicationChannelString(t *testing.T) {
	tests := []struct {
		ch   CommunicationChannel
		want string
	}{
		{ChannelAll, "all"},
		{ChannelRemote, "remote"},
		{ChannelIPC, "ipc"},
	}
	for _, tt := range tests {
		if got := tt.ch.String(); got != tt.want {
			t.Errorf("CommunicationChannel.String() = %q, want %q", got, tt.want)
		}
	}
}

func TestCommunicationChannelIncludes(t *testing.T) {
	if !ChannelAll.IncludesMQTT() {
		t.Error("ChannelAll should include MQTT")
	}
	if !ChannelAll.IncludesIPC() {
		t.Error("ChannelAll should include IPC")
	}
	if !ChannelRemote.IncludesMQTT() {
		t.Error("ChannelRemote should include MQTT")
	}
	if ChannelRemote.IncludesIPC() {
		t.Error("ChannelRemote should NOT include IPC")
	}
	if ChannelIPC.IncludesMQTT() {
		t.Error("ChannelIPC should NOT include MQTT")
	}
	if !ChannelIPC.IncludesIPC() {
		t.Error("ChannelIPC should include IPC")
	}
}
