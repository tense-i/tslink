package tslink

import "encoding/json"

// ReplyMessage represents a platform reply to a request.
type ReplyMessage struct {
	TID     string          `json:"tid"`
	BID     string          `json:"bid"`
	Code    int             `json:"code"`
	Message string          `json:"message"`
	Data    json.RawMessage `json:"data,omitempty"`
}

// NewReplySuccess creates a success reply (code=0).
func NewReplySuccess(tid, bid string) *ReplyMessage {
	return &ReplyMessage{TID: tid, BID: bid, Code: 0, Message: "success"}
}

// NewReplySuccessWithData creates a success reply carrying data.
func NewReplySuccessWithData(tid, bid string, data interface{}) *ReplyMessage {
	raw, _ := json.Marshal(data)
	return &ReplyMessage{TID: tid, BID: bid, Code: 0, Message: "success", Data: raw}
}

// NewReplyError creates an error reply.
func NewReplyError(tid, bid string, code int, message string) *ReplyMessage {
	return &ReplyMessage{TID: tid, BID: bid, Code: code, Message: message}
}

// IsSuccess returns true when code == 0.
func (r *ReplyMessage) IsSuccess() bool { return r.Code == 0 }

// ToJSON serialises the reply to JSON.
func (r *ReplyMessage) ToJSON() (string, error) {
	b, err := json.Marshal(r)
	return string(b), err
}

// ReplyMessageFromJSON deserialises a ReplyMessage from JSON.
func ReplyMessageFromJSON(data string) (*ReplyMessage, error) {
	var msg ReplyMessage
	err := json.Unmarshal([]byte(data), &msg)
	return &msg, err
}
