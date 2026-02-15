package tslink

import (
	"encoding/json"
	"log"
	"sync"
	"sync/atomic"
	"time"
)

// DeviceInfo represents a discovered device on the local network.
type DeviceInfo struct {
	ProductKey string            `json:"product_key"`
	DeviceID   string            `json:"device_id"`
	DeviceName string            `json:"device_name,omitempty"`
	LastSeenMs uint64            `json:"last_seen_ms"`
	Metadata   map[string]string `json:"metadata,omitempty"`
}

// NewDeviceInfo creates a DeviceInfo with the current timestamp.
func NewDeviceInfo(productKey, deviceID string) *DeviceInfo {
	return &DeviceInfo{
		ProductKey: productKey,
		DeviceID:   deviceID,
		LastSeenMs: uint64(time.Now().UnixMilli()),
		Metadata:   make(map[string]string),
	}
}

// Key returns a unique device key.
func (d *DeviceInfo) Key() string { return d.ProductKey + ":" + d.DeviceID }

// Touch updates LastSeenMs to now.
func (d *DeviceInfo) Touch() { d.LastSeenMs = uint64(time.Now().UnixMilli()) }

// DeviceDiscoveryConfig configures the discovery service.
type DeviceDiscoveryConfig struct {
	ProductKey           string
	DeviceID             string
	BroadcastIntervalSec uint64
	DeviceTimeoutSec     uint64
}

// DefaultDeviceDiscoveryConfig returns sensible defaults.
func DefaultDeviceDiscoveryConfig() DeviceDiscoveryConfig {
	return DeviceDiscoveryConfig{
		BroadcastIntervalSec: 5,
		DeviceTimeoutSec:     15,
	}
}

// DeviceStatusCallback is called when a device comes online/offline.
type DeviceStatusCallback func(info *DeviceInfo, online bool)

type deviceEntry struct {
	info     *DeviceInfo
	lastSeen time.Time
}

// DeviceDiscovery provides IPC-based local device discovery.
// NOTE: The actual IPC broadcast is not implemented (placeholder).
// Only the device table + cleanup logic is provided.
type DeviceDiscovery struct {
	config   DeviceDiscoveryConfig
	mu       sync.RWMutex
	devices  map[string]*deviceEntry
	callback DeviceStatusCallback
	running  atomic.Bool
	stopCh   chan struct{}
}

// NewDeviceDiscovery creates a new discovery service.
func NewDeviceDiscovery(config DeviceDiscoveryConfig) *DeviceDiscovery {
	return &DeviceDiscovery{
		config:  config,
		devices: make(map[string]*deviceEntry),
	}
}

// SetStatusCallback sets the online/offline callback.
func (d *DeviceDiscovery) SetStatusCallback(cb DeviceStatusCallback) {
	d.mu.Lock()
	defer d.mu.Unlock()
	d.callback = cb
}

// GetDevices returns a snapshot of online devices.
func (d *DeviceDiscovery) GetDevices() []*DeviceInfo {
	d.mu.RLock()
	defer d.mu.RUnlock()
	out := make([]*DeviceInfo, 0, len(d.devices))
	for _, e := range d.devices {
		clone := *e.info
		out = append(out, &clone)
	}
	return out
}

// GetDevice returns a specific device, or nil if not found.
func (d *DeviceDiscovery) GetDevice(productKey, deviceID string) *DeviceInfo {
	key := productKey + ":" + deviceID
	d.mu.RLock()
	defer d.mu.RUnlock()
	e, ok := d.devices[key]
	if !ok {
		return nil
	}
	clone := *e.info
	return &clone
}

// IsDeviceOnline checks if a device is in the table.
func (d *DeviceDiscovery) IsDeviceOnline(productKey, deviceID string) bool {
	key := productKey + ":" + deviceID
	d.mu.RLock()
	defer d.mu.RUnlock()
	_, ok := d.devices[key]
	return ok
}

// HandleDiscoveryMessage should be called when a discovery payload arrives.
func (d *DeviceDiscovery) HandleDiscoveryMessage(payload string) {
	var info DeviceInfo
	if err := json.Unmarshal([]byte(payload), &info); err != nil {
		log.Printf("[tslink] failed to parse discovery message: %v", err)
		return
	}
	info.Touch()
	key := info.Key()

	d.mu.Lock()
	_, exists := d.devices[key]
	d.devices[key] = &deviceEntry{info: &info, lastSeen: time.Now()}
	cb := d.callback
	d.mu.Unlock()

	if !exists {
		log.Printf("[tslink] new device discovered: %s", key)
		if cb != nil {
			cb(&info, true)
		}
	}
}

// CreateBroadcastMessage creates the JSON payload for this device.
func (d *DeviceDiscovery) CreateBroadcastMessage() string {
	info := NewDeviceInfo(d.config.ProductKey, d.config.DeviceID)
	b, _ := json.Marshal(info)
	return string(b)
}

// Start begins the cleanup goroutine.
func (d *DeviceDiscovery) Start() error {
	if d.running.Swap(true) {
		return nil
	}
	d.stopCh = make(chan struct{})
	go d.cleanupLoop()
	return nil
}

// Stop halts the cleanup goroutine.
func (d *DeviceDiscovery) Stop() {
	if !d.running.Swap(false) {
		return
	}
	close(d.stopCh)
}

// IsRunning returns whether the discovery service is active.
func (d *DeviceDiscovery) IsRunning() bool { return d.running.Load() }

func (d *DeviceDiscovery) cleanupLoop() {
	ticker := time.NewTicker(5 * time.Second)
	defer ticker.Stop()
	timeout := time.Duration(d.config.DeviceTimeoutSec) * time.Second

	for {
		select {
		case <-d.stopCh:
			return
		case <-ticker.C:
			d.mu.Lock()
			var expired []struct {
				key  string
				info *DeviceInfo
			}
			for key, entry := range d.devices {
				if time.Since(entry.lastSeen) > timeout {
					expired = append(expired, struct {
						key  string
						info *DeviceInfo
					}{key, entry.info})
				}
			}
			cb := d.callback
			for _, e := range expired {
				delete(d.devices, e.key)
			}
			d.mu.Unlock()

			for _, e := range expired {
				log.Printf("[tslink] device offline (timeout): %s", e.key)
				if cb != nil {
					cb(e.info, false)
				}
			}
		}
	}
}
