package tslink

import (
	"errors"
	"fmt"
)

// Sentinel errors for common SDK error conditions.
var (
	ErrNotStarted     = errors.New("tslink: client not started")
	ErrAlreadyStarted = errors.New("tslink: client already started")
)

// Error wraps a category + detail string. It satisfies the error interface.
type Error struct {
	Category string
	Detail   string
}

func (e *Error) Error() string {
	return fmt.Sprintf("tslink [%s]: %s", e.Category, e.Detail)
}

// Convenience constructors ------------------------------------------------

func errMqttConnection(detail string) error {
	return &Error{Category: "mqtt_connection", Detail: detail}
}

func errMqttPublish(detail string) error {
	return &Error{Category: "mqtt_publish", Detail: detail}
}

func errMqttSubscribe(detail string) error {
	return &Error{Category: "mqtt_subscribe", Detail: detail}
}

func errConfiguration(detail string) error {
	return &Error{Category: "configuration", Detail: detail}
}

func errChannel(detail string) error {
	return &Error{Category: "channel", Detail: detail}
}

func errTimeout(detail string) error {
	return &Error{Category: "timeout", Detail: detail}
}

func errCallbackNotFound(detail string) error {
	return &Error{Category: "callback_not_found", Detail: detail}
}

func errInternal(detail string) error {
	return &Error{Category: "internal", Detail: detail}
}
