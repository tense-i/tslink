package tslink

import "testing"

func TestReplyMessageSuccess(t *testing.T) {
	reply := NewReplySuccess("tid-1", "bid-1")
	if !reply.IsSuccess() {
		t.Error("expected IsSuccess() == true")
	}
	if reply.Code != 0 {
		t.Errorf("Code = %d, want 0", reply.Code)
	}
}

func TestReplyMessageError(t *testing.T) {
	reply := NewReplyError("tid-1", "bid-1", 500, "Internal error")
	if reply.IsSuccess() {
		t.Error("expected IsSuccess() == false")
	}
	if reply.Code != 500 {
		t.Errorf("Code = %d, want 500", reply.Code)
	}
}

func TestReplyMessageSerialization(t *testing.T) {
	reply := NewReplySuccessWithData("tid-1", "bid-1", map[string]bool{"result": true})
	jsonStr, err := reply.ToJSON()
	if err != nil {
		t.Fatal(err)
	}
	parsed, err := ReplyMessageFromJSON(jsonStr)
	if err != nil {
		t.Fatal(err)
	}
	if reply.TID != parsed.TID {
		t.Errorf("TID mismatch")
	}
	if parsed.Data == nil {
		t.Error("Data should not be nil")
	}
}
