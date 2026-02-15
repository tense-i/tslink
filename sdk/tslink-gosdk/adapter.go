package tslink

import (
	"encoding/json"
	"fmt"
	"log"
	"strings"
	"sync"
	"time"
)

// ReplyHandler wraps a callback waiting for a service reply.
type ReplyHandler struct {
	IsPlatform bool
	PlatformCb PlatformResponseCallback
	DeviceCb   ServiceResponseCallback
}

// MessageAdapter routes incoming messages to registered executors / reply handlers.
// It implements MessageReceiveCallback.
type MessageAdapter struct {
	mu               sync.RWMutex
	serviceExecutors map[string]ServiceExecutor
	unifiedExecutor  ServiceExecutor

	replyMu       sync.Mutex
	replyHandlers map[string]*ReplyHandler

	replySenderMu sync.RWMutex
	replySender   func(topic, data string)
}

// NewMessageAdapter creates a new MessageAdapter.
func NewMessageAdapter() *MessageAdapter {
	return &MessageAdapter{
		serviceExecutors: make(map[string]ServiceExecutor),
		replyHandlers:    make(map[string]*ReplyHandler),
	}
}

// SetReplySender sets the function used to send reply messages back.
func (a *MessageAdapter) SetReplySender(sender func(topic, data string)) {
	a.replySenderMu.Lock()
	defer a.replySenderMu.Unlock()
	a.replySender = sender
}

// AddServiceExecutor registers an executor for the given method key.
func (a *MessageAdapter) AddServiceExecutor(method string, executor ServiceExecutor) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.serviceExecutors[method] = executor
}

// SetUnifiedExecutor registers a fallback executor for all methods.
func (a *MessageAdapter) SetUnifiedExecutor(executor ServiceExecutor) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.unifiedExecutor = executor
}

// AddReplyHandler registers a handler waiting for the given tid.
func (a *MessageAdapter) AddReplyHandler(tid string, handler *ReplyHandler) {
	a.replyMu.Lock()
	defer a.replyMu.Unlock()
	a.replyHandlers[tid] = handler
}

// RemoveReplyHandler removes and returns a reply handler for tid.
func (a *MessageAdapter) RemoveReplyHandler(tid string) *ReplyHandler {
	a.replyMu.Lock()
	defer a.replyMu.Unlock()
	h, ok := a.replyHandlers[tid]
	if ok {
		delete(a.replyHandlers, tid)
	}
	return h
}

// Receive implements MessageReceiveCallback.
func (a *MessageAdapter) Receive(topic, data string) {
	a.handleMessage(topic, data)
}

// Release clears all state.
func (a *MessageAdapter) Release() {
	a.mu.Lock()
	a.serviceExecutors = make(map[string]ServiceExecutor)
	a.unifiedExecutor = nil
	a.mu.Unlock()

	a.replyMu.Lock()
	a.replyHandlers = make(map[string]*ReplyHandler)
	a.replyMu.Unlock()

	a.replySenderMu.Lock()
	a.replySender = nil
	a.replySenderMu.Unlock()
}

// ---------------------------------------------------------------------------
// Internal message routing
// ---------------------------------------------------------------------------

func (a *MessageAdapter) handleMessage(topic, data string) {
	// Try parsing as CommonMessage first
	var msg CommonMessage
	if err := json.Unmarshal([]byte(data), &msg); err == nil && msg.Method != "" {
		a.handleCommonMessage(topic, &msg)
		return
	}

	// Try parsing as ReplyMessage
	var reply ReplyMessage
	if err := json.Unmarshal([]byte(data), &reply); err == nil && reply.TID != "" {
		a.handleReplyMessage(topic, &reply)
		return
	}

	log.Printf("[tslink] failed to parse message on topic=%s", topic)
}

func (a *MessageAdapter) handleCommonMessage(topic string, msg *CommonMessage) {
	method := msg.Method
	if method == "" {
		method = a.extractMethodFromTopic(topic)
	}
	serviceIdentifier := a.extractServiceIdentifier(method)

	paramData := msg.Data
	if paramData == nil {
		paramData = json.RawMessage("null")
	}

	request := &DeviceServiceRequest{
		Channel:            ChannelAll,
		ServiceIdentifier:  serviceIdentifier,
		ParamData:          []byte(paramData),
		ServiceTimestampMs: msg.Timestamp,
	}

	executor := a.findExecutor(method, serviceIdentifier)
	if executor == nil {
		log.Printf("[tslink] no executor registered for method=%s", method)
		return
	}

	a.replySenderMu.RLock()
	sender := a.replySender
	a.replySenderMu.RUnlock()

	replyTopic := a.getReplyTopic(topic)
	tid := msg.TID
	bid := msg.BID

	replyCb := func(resultCode int, data []byte) {
		if sender != nil {
			reply := &ReplyMessage{
				TID:     tid,
				BID:     bid,
				Code:    resultCode,
				Message: "success",
				Data:    data,
			}
			if resultCode != 0 {
				reply.Message = "error"
			}
			replyJSON, err := json.Marshal(reply)
			if err == nil {
				sender(replyTopic, string(replyJSON))
			}
		}
	}

	executor(request, replyCb)
}

func (a *MessageAdapter) handleReplyMessage(topic string, reply *ReplyMessage) {
	handler := a.RemoveReplyHandler(reply.TID)
	if handler == nil {
		return
	}

	paramData := reply.Data
	if paramData == nil {
		paramData = json.RawMessage("null")
	}
	identifier := a.extractServiceIdentifierFromTopic(topic)
	ts := time.Now().UnixMilli()

	if handler.IsPlatform && handler.PlatformCb != nil {
		handler.PlatformCb(&PlatformServiceResponse{
			ServiceIdentifier:  identifier,
			Result:             reply.Code,
			ParamData:          []byte(paramData),
			ServiceTimestampMs: ts,
		})
	} else if handler.DeviceCb != nil {
		handler.DeviceCb(&DeviceServiceResponse{
			ServiceIdentifier:  identifier,
			Result:             reply.Code,
			ParamData:          []byte(paramData),
			ServiceTimestampMs: ts,
		})
	}
}

func (a *MessageAdapter) findExecutor(method, serviceIdentifier string) ServiceExecutor {
	a.mu.RLock()
	defer a.mu.RUnlock()

	if ex, ok := a.serviceExecutors[method]; ok {
		return ex
	}
	alt := fmt.Sprintf("service.%s.post", method)
	if ex, ok := a.serviceExecutors[alt]; ok {
		return ex
	}
	alt2 := fmt.Sprintf("service.%s.post", serviceIdentifier)
	if ex, ok := a.serviceExecutors[alt2]; ok {
		return ex
	}
	return a.unifiedExecutor
}

func (a *MessageAdapter) extractMethodFromTopic(topic string) string {
	parts := strings.Split(topic, "/")
	// sys/{pk}/{did}/thing/service/{identifier}/post
	if len(parts) >= 7 && parts[4] == "service" {
		return fmt.Sprintf("service.%s.post", parts[5])
	}
	if strings.Contains(topic, "properties/set") {
		return "thing.properties.set"
	}
	return topic
}

func (a *MessageAdapter) extractServiceIdentifier(method string) string {
	parts := strings.Split(method, ".")
	if len(parts) >= 3 && parts[0] == "platform" {
		return parts[2]
	}
	if len(parts) >= 2 {
		return parts[1]
	}
	return method
}

func (a *MessageAdapter) extractServiceIdentifierFromTopic(topic string) string {
	parts := strings.Split(topic, "/")
	if len(parts) >= 6 {
		return parts[5]
	}
	return ""
}

func (a *MessageAdapter) getReplyTopic(topic string) string {
	if strings.HasSuffix(topic, "/post") {
		return topic + "_reply"
	}
	return topic + "/reply"
}
