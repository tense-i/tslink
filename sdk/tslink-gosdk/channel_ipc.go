package tslink

import (
	"context"
	"errors"
)

// IpcChannel is a placeholder for IPC (iceoryx2-style) zero-copy channel.
// Not yet implemented — all methods return ErrNotImplemented.
type IpcChannel struct{}

// ErrIPCNotImplemented is returned by all IpcChannel methods.
var ErrIPCNotImplemented = errors.New("tslink: IPC channel not implemented")

// NewIpcChannel constructs a placeholder IpcChannel.
func NewIpcChannel() *IpcChannel { return &IpcChannel{} }

func (c *IpcChannel) Send(_ context.Context, _, _ string) error { return ErrIPCNotImplemented }
func (c *IpcChannel) Start(_ context.Context) error             { return ErrIPCNotImplemented }
func (c *IpcChannel) Stop(_ context.Context) error              { return nil }
func (c *IpcChannel) AddTopic(_ context.Context, _ string) error {
	return ErrIPCNotImplemented
}
func (c *IpcChannel) IsConnected() bool { return false }
