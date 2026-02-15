package tslink

import (
	"sync/atomic"
	"testing"
)

func TestPlatformServiceRequestNew(t *testing.T) {
	req := NewPlatformServiceRequest("test_service", []byte("hello")).
		WithDevice("pk1", "did1").
		WithChannel(ChannelRemote)

	if req.ServiceIdentifier != "test_service" {
		t.Errorf("ServiceIdentifier = %q, want %q", req.ServiceIdentifier, "test_service")
	}
	if string(req.ParamData) != "hello" {
		t.Errorf("ParamData = %q, want %q", req.ParamData, "hello")
	}
	if req.ProductKey != "pk1" {
		t.Errorf("ProductKey = %q, want %q", req.ProductKey, "pk1")
	}
	if req.DeviceID != "did1" {
		t.Errorf("DeviceID = %q, want %q", req.DeviceID, "did1")
	}
	if req.Channel != ChannelRemote {
		t.Errorf("Channel = %v, want ChannelRemote", req.Channel)
	}
}

func TestDeviceServiceRequestNew(t *testing.T) {
	req := NewDeviceServiceRequest("dev_svc", []byte("data"))
	if req.ServiceIdentifier != "dev_svc" {
		t.Errorf("ServiceIdentifier = %q, want %q", req.ServiceIdentifier, "dev_svc")
	}
	if string(req.ParamData) != "data" {
		t.Errorf("ParamData = %q, want %q", req.ParamData, "data")
	}
	if req.ServiceTimestampMs != 0 {
		t.Errorf("ServiceTimestampMs = %d, want 0", req.ServiceTimestampMs)
	}
}

func TestPlatformServiceResponseSuccess(t *testing.T) {
	resp := NewPlatformServiceResponseSuccess("svc1", []byte("ok"))
	if resp.Result != 0 {
		t.Errorf("Result = %d, want 0", resp.Result)
	}
	if resp.ServiceIdentifier != "svc1" {
		t.Errorf("ServiceIdentifier = %q, want %q", resp.ServiceIdentifier, "svc1")
	}
	if string(resp.ParamData) != "ok" {
		t.Errorf("ParamData = %q, want %q", resp.ParamData, "ok")
	}
}

func TestDeviceServiceResponseError(t *testing.T) {
	resp := NewDeviceServiceResponseError("svc2", -1)
	if resp.Result != -1 {
		t.Errorf("Result = %d, want -1", resp.Result)
	}
	if len(resp.ParamData) != 0 {
		t.Error("ParamData should be empty")
	}
}

func TestReplyCallback(t *testing.T) {
	var called atomic.Bool
	cb := ReplyCallback(func(code int, data []byte) {
		if code != 0 {
			t.Errorf("code = %d, want 0", code)
		}
		called.Store(true)
	})
	cb(0, []byte("ok"))
	if !called.Load() {
		t.Error("callback was not called")
	}
}
