package tslink

import (
	"encoding/json"
	"time"

	"github.com/google/uuid"
)

// CommonMessage is the standard IoT protocol message envelope.
// It corresponds to the Rust CommonMessage struct.
type CommonMessage struct {
	TID       string          `json:"tid"`
	BID       string          `json:"bid"`
	Version   string          `json:"version"`
	Timestamp int64           `json:"timestamp"`
	Method    string          `json:"method"`
	Data      json.RawMessage `json:"data"`
}

// NewCommonMessage creates a CommonMessage with auto-generated tid/bid.
func NewCommonMessage(method string, data interface{}) (*CommonMessage, error) {
	raw, err := json.Marshal(data)
	if err != nil {
		return nil, err
	}
	return &CommonMessage{
		TID:       uuid.New().String(),
		BID:       uuid.New().String(),
		Version:   "1.0",
		Timestamp: nowMillis(),
		Method:    method,
		Data:      raw,
	}, nil
}

// NewCommonMessageWithTID creates a CommonMessage with a specified tid.
func NewCommonMessageWithTID(tid, method string, data interface{}) (*CommonMessage, error) {
	raw, err := json.Marshal(data)
	if err != nil {
		return nil, err
	}
	return &CommonMessage{
		TID:       tid,
		BID:       uuid.New().String(),
		Version:   "1.0",
		Timestamp: nowMillis(),
		Method:    method,
		Data:      raw,
	}, nil
}

// ToJSON serialises the message to JSON string.
func (m *CommonMessage) ToJSON() (string, error) {
	b, err := json.Marshal(m)
	return string(b), err
}

// CommonMessageFromJSON deserialises a CommonMessage from JSON string.
func CommonMessageFromJSON(data string) (*CommonMessage, error) {
	var msg CommonMessage
	err := json.Unmarshal([]byte(data), &msg)
	return &msg, err
}

// CommonMessageBuilder provides a fluent API for constructing CommonMessage.
type CommonMessageBuilder struct {
	tid       string
	bid       string
	version   string
	timestamp int64
	method    string
	data      json.RawMessage
}

// NewCommonMessageBuilder creates a new builder.
func NewCommonMessageBuilder() *CommonMessageBuilder {
	return &CommonMessageBuilder{}
}

func (b *CommonMessageBuilder) TID(tid string) *CommonMessageBuilder {
	b.tid = tid
	return b
}

func (b *CommonMessageBuilder) BID(bid string) *CommonMessageBuilder {
	b.bid = bid
	return b
}

func (b *CommonMessageBuilder) Version(v string) *CommonMessageBuilder {
	b.version = v
	return b
}

func (b *CommonMessageBuilder) Timestamp(ts int64) *CommonMessageBuilder {
	b.timestamp = ts
	return b
}

func (b *CommonMessageBuilder) Method(m string) *CommonMessageBuilder {
	b.method = m
	return b
}

func (b *CommonMessageBuilder) DataRaw(raw json.RawMessage) *CommonMessageBuilder {
	b.data = raw
	return b
}

func (b *CommonMessageBuilder) DataValue(v interface{}) *CommonMessageBuilder {
	raw, _ := json.Marshal(v)
	b.data = raw
	return b
}

// Build constructs the CommonMessage, filling defaults for unset fields.
func (b *CommonMessageBuilder) Build() *CommonMessage {
	msg := &CommonMessage{}
	if b.tid != "" {
		msg.TID = b.tid
	} else {
		msg.TID = uuid.New().String()
	}
	if b.bid != "" {
		msg.BID = b.bid
	} else {
		msg.BID = uuid.New().String()
	}
	if b.version != "" {
		msg.Version = b.version
	} else {
		msg.Version = "1.0"
	}
	if b.timestamp != 0 {
		msg.Timestamp = b.timestamp
	} else {
		msg.Timestamp = nowMillis()
	}
	msg.Method = b.method
	if b.data != nil {
		msg.Data = b.data
	} else {
		msg.Data = json.RawMessage("null")
	}
	return msg
}

func nowMillis() int64 {
	return time.Now().UnixMilli()
}
