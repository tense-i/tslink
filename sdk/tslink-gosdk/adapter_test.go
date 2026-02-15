package tslink

import (
	"encoding/json"
	"sync/atomic"
	"testing"
)

func TestAdapterServiceExecutorRegistration(t *testing.T) {
	adapter := NewMessageAdapter()
	var called atomic.Bool
	executor := ServiceExecutor(func(req *DeviceServiceRequest, reply ReplyCallback) {
		called.Store(true)
		if req.ServiceIdentifier != "test" {
			t.Errorf("ServiceIdentifier = %q, want %q", req.ServiceIdentifier, "test")
		}
		reply(0, []byte("ok"))
	})
	adapter.AddServiceExecutor("service.test.post", executor)

	// Simulate incoming message
	msg, _ := NewCommonMessage("service.test.post", map[string]interface{}{})
	jsonStr, _ := msg.ToJSON()
	adapter.Receive("sys/pk/did/thing/service/test/post", jsonStr)

	if !called.Load() {
		t.Error("executor was not called")
	}
}

func TestAdapterUnifiedExecutorFallback(t *testing.T) {
	adapter := NewMessageAdapter()
	var called atomic.Bool
	executor := ServiceExecutor(func(req *DeviceServiceRequest, reply ReplyCallback) {
		called.Store(true)
	})
	adapter.SetUnifiedExecutor(executor)

	msg, _ := NewCommonMessage("service.unknown.post", map[string]interface{}{})
	jsonStr, _ := msg.ToJSON()
	adapter.Receive("sys/pk/did/thing/service/unknown/post", jsonStr)

	if !called.Load() {
		t.Error("unified executor was not called")
	}
}

func TestAdapterReplyHandlerPlatform(t *testing.T) {
	adapter := NewMessageAdapter()
	var called atomic.Bool
	adapter.AddReplyHandler("test-tid", &ReplyHandler{
		IsPlatform: true,
		PlatformCb: func(resp *PlatformServiceResponse) {
			called.Store(true)
			if resp.Result != 0 {
				t.Errorf("Result = %d, want 0", resp.Result)
			}
		},
	})

	reply := NewReplySuccess("test-tid", "test-bid")
	replyJSON, _ := json.Marshal(reply)
	adapter.Receive("sys/pk/did/platform/service/svc1/post_reply", string(replyJSON))

	if !called.Load() {
		t.Error("platform reply handler was not called")
	}
}

func TestAdapterReplyHandlerDevice(t *testing.T) {
	adapter := NewMessageAdapter()
	var called atomic.Bool
	adapter.AddReplyHandler("test-tid-2", &ReplyHandler{
		IsPlatform: false,
		DeviceCb: func(resp *DeviceServiceResponse) {
			called.Store(true)
			if resp.Result != 0 {
				t.Errorf("Result = %d, want 0", resp.Result)
			}
		},
	})

	reply := NewReplySuccess("test-tid-2", "test-bid-2")
	replyJSON, _ := json.Marshal(reply)
	adapter.Receive("sys/pk/did/thing/service/svc2/post_reply", string(replyJSON))

	if !called.Load() {
		t.Error("device reply handler was not called")
	}
}

func TestAdapterRelease(t *testing.T) {
	adapter := NewMessageAdapter()
	adapter.AddServiceExecutor("service.test.post", func(req *DeviceServiceRequest, reply ReplyCallback) {})
	adapter.AddReplyHandler("tid", &ReplyHandler{IsPlatform: true})
	adapter.Release()

	// After release, unified should be nil and maps empty
	if adapter.unifiedExecutor != nil {
		t.Error("unifiedExecutor should be nil after Release")
	}
	if len(adapter.serviceExecutors) != 0 {
		t.Error("serviceExecutors should be empty after Release")
	}
	if len(adapter.replyHandlers) != 0 {
		t.Error("replyHandlers should be empty after Release")
	}
}
