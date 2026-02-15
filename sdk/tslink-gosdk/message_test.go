package tslink

import (
	"encoding/json"
	"testing"
)

func TestCommonMessageNew(t *testing.T) {
	msg, err := NewCommonMessage("event.property.post", map[string]interface{}{"temp": 25})
	if err != nil {
		t.Fatal(err)
	}
	if msg.Method != "event.property.post" {
		t.Errorf("Method = %q, want %q", msg.Method, "event.property.post")
	}
	if msg.Version != "1.0" {
		t.Errorf("Version = %q, want %q", msg.Version, "1.0")
	}
	if msg.TID == "" {
		t.Error("TID should not be empty")
	}
	if msg.BID == "" {
		t.Error("BID should not be empty")
	}
	if msg.Timestamp <= 0 {
		t.Error("Timestamp should be positive")
	}
}

func TestCommonMessageBuilder(t *testing.T) {
	msg := NewCommonMessageBuilder().
		TID("test-tid").
		Method("test.method").
		DataValue(map[string]string{"key": "value"}).
		Build()

	if msg.TID != "test-tid" {
		t.Errorf("TID = %q, want %q", msg.TID, "test-tid")
	}
	if msg.Method != "test.method" {
		t.Errorf("Method = %q, want %q", msg.Method, "test.method")
	}
}

func TestCommonMessageSerialization(t *testing.T) {
	original, _ := NewCommonMessage("test", map[string]string{})
	jsonStr, err := original.ToJSON()
	if err != nil {
		t.Fatal(err)
	}

	parsed, err := CommonMessageFromJSON(jsonStr)
	if err != nil {
		t.Fatal(err)
	}
	if original.TID != parsed.TID {
		t.Errorf("TID mismatch: %q vs %q", original.TID, parsed.TID)
	}
	if original.Method != parsed.Method {
		t.Errorf("Method mismatch: %q vs %q", original.Method, parsed.Method)
	}
}

func TestCommonMessageDataParamsAlias(t *testing.T) {
	// Test that "params" also unmarshals into "data"
	raw := `{"tid":"t1","bid":"b1","version":"1.0","timestamp":123,"method":"test","params":{"x":1}}`
	var msg CommonMessage
	err := json.Unmarshal([]byte(raw), &msg)
	if err != nil {
		t.Fatal(err)
	}
	if msg.TID != "t1" {
		t.Errorf("TID = %q, want %q", msg.TID, "t1")
	}
}
