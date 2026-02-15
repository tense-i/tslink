package tslink

import (
	"encoding/json"
	"testing"
)

func TestDeviceInfoNew(t *testing.T) {
	info := NewDeviceInfo("pk1", "dev1")
	if info.ProductKey != "pk1" {
		t.Errorf("ProductKey = %q, want %q", info.ProductKey, "pk1")
	}
	if info.DeviceID != "dev1" {
		t.Errorf("DeviceID = %q, want %q", info.DeviceID, "dev1")
	}
	if info.Key() != "pk1:dev1" {
		t.Errorf("Key() = %q, want %q", info.Key(), "pk1:dev1")
	}
}

func TestDeviceDiscoveryConfigDefault(t *testing.T) {
	config := DefaultDeviceDiscoveryConfig()
	if config.BroadcastIntervalSec != 5 {
		t.Errorf("BroadcastIntervalSec = %d, want 5", config.BroadcastIntervalSec)
	}
	if config.DeviceTimeoutSec != 15 {
		t.Errorf("DeviceTimeoutSec = %d, want 15", config.DeviceTimeoutSec)
	}
}

func TestDeviceDiscoveryHandleMessage(t *testing.T) {
	config := DeviceDiscoveryConfig{
		ProductKey:           "test_pk",
		DeviceID:             "test_dev",
		BroadcastIntervalSec: 5,
		DeviceTimeoutSec:     15,
	}
	discovery := NewDeviceDiscovery(config)

	info := NewDeviceInfo("remote_pk", "remote_dev")
	payload, _ := json.Marshal(info)
	discovery.HandleDiscoveryMessage(string(payload))

	devices := discovery.GetDevices()
	if len(devices) != 1 {
		t.Fatalf("len(devices) = %d, want 1", len(devices))
	}
	if devices[0].ProductKey != "remote_pk" {
		t.Errorf("ProductKey = %q, want %q", devices[0].ProductKey, "remote_pk")
	}
}

func TestDeviceDiscoveryIsDeviceOnline(t *testing.T) {
	config := DefaultDeviceDiscoveryConfig()
	discovery := NewDeviceDiscovery(config)

	if discovery.IsDeviceOnline("pk", "did") {
		t.Error("device should not be online")
	}

	info := NewDeviceInfo("pk", "did")
	payload, _ := json.Marshal(info)
	discovery.HandleDiscoveryMessage(string(payload))

	if !discovery.IsDeviceOnline("pk", "did") {
		t.Error("device should be online after discovery message")
	}
}

func TestDeviceDiscoveryCreateBroadcastMessage(t *testing.T) {
	config := DeviceDiscoveryConfig{
		ProductKey: "my_pk",
		DeviceID:   "my_did",
	}
	discovery := NewDeviceDiscovery(config)
	msg := discovery.CreateBroadcastMessage()

	var info DeviceInfo
	if err := json.Unmarshal([]byte(msg), &info); err != nil {
		t.Fatal(err)
	}
	if info.ProductKey != "my_pk" {
		t.Errorf("ProductKey = %q, want %q", info.ProductKey, "my_pk")
	}
}
