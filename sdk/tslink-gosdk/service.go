package tslink

import "time"

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

// PlatformServiceRequest describes a request to invoke a platform service.
type PlatformServiceRequest struct {
	Channel           CommunicationChannel `json:"channel"`
	ServiceIdentifier string               `json:"service_identifier"`
	ParamData         []byte               `json:"param_data"`
	ProductKey        string               `json:"product_key,omitempty"`
	DeviceID          string               `json:"device_id,omitempty"`
}

// NewPlatformServiceRequest creates a PlatformServiceRequest.
func NewPlatformServiceRequest(serviceIdentifier string, paramData []byte) *PlatformServiceRequest {
	return &PlatformServiceRequest{
		Channel:           ChannelAll,
		ServiceIdentifier: serviceIdentifier,
		ParamData:         paramData,
	}
}

// WithDevice sets the target product key and device ID.
func (r *PlatformServiceRequest) WithDevice(productKey, deviceID string) *PlatformServiceRequest {
	r.ProductKey = productKey
	r.DeviceID = deviceID
	return r
}

// WithChannel sets the communication channel.
func (r *PlatformServiceRequest) WithChannel(ch CommunicationChannel) *PlatformServiceRequest {
	r.Channel = ch
	return r
}

// DeviceServiceRequest describes a request to invoke a device service.
type DeviceServiceRequest struct {
	Channel            CommunicationChannel `json:"channel"`
	ServiceIdentifier  string               `json:"service_identifier"`
	ParamData          []byte               `json:"param_data"`
	ServiceTimestampMs int64                `json:"service_timestamp_ms"`
}

// NewDeviceServiceRequest creates a DeviceServiceRequest.
func NewDeviceServiceRequest(serviceIdentifier string, paramData []byte) *DeviceServiceRequest {
	return &DeviceServiceRequest{
		Channel:           ChannelAll,
		ServiceIdentifier: serviceIdentifier,
		ParamData:         paramData,
	}
}

// WithChannel sets the communication channel.
func (r *DeviceServiceRequest) WithChannel(ch CommunicationChannel) *DeviceServiceRequest {
	r.Channel = ch
	return r
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

// PlatformServiceResponse encapsulates the result of a platform service call.
type PlatformServiceResponse struct {
	Channel            CommunicationChannel `json:"channel"`
	ServiceIdentifier  string               `json:"service_identifier"`
	Result             int                  `json:"result"`
	ParamData          []byte               `json:"param_data"`
	ServiceTimestampMs int64                `json:"service_timestamp_ms"`
}

// NewPlatformServiceResponseSuccess creates a success response.
func NewPlatformServiceResponseSuccess(serviceIdentifier string, paramData []byte) *PlatformServiceResponse {
	return &PlatformServiceResponse{
		ServiceIdentifier:  serviceIdentifier,
		Result:             0,
		ParamData:          paramData,
		ServiceTimestampMs: time.Now().UnixMilli(),
	}
}

// NewPlatformServiceResponseError creates an error response.
func NewPlatformServiceResponseError(serviceIdentifier string, result int) *PlatformServiceResponse {
	return &PlatformServiceResponse{
		ServiceIdentifier:  serviceIdentifier,
		Result:             result,
		ServiceTimestampMs: time.Now().UnixMilli(),
	}
}

// DeviceServiceResponse encapsulates the result of a device service call.
type DeviceServiceResponse struct {
	Channel            CommunicationChannel `json:"channel"`
	ServiceIdentifier  string               `json:"service_identifier"`
	Result             int                  `json:"result"`
	ParamData          []byte               `json:"param_data"`
	ServiceTimestampMs int64                `json:"service_timestamp_ms"`
}

// NewDeviceServiceResponseSuccess creates a success response.
func NewDeviceServiceResponseSuccess(serviceIdentifier string, paramData []byte) *DeviceServiceResponse {
	return &DeviceServiceResponse{
		ServiceIdentifier:  serviceIdentifier,
		Result:             0,
		ParamData:          paramData,
		ServiceTimestampMs: time.Now().UnixMilli(),
	}
}

// NewDeviceServiceResponseError creates an error response.
func NewDeviceServiceResponseError(serviceIdentifier string, result int) *DeviceServiceResponse {
	return &DeviceServiceResponse{
		ServiceIdentifier:  serviceIdentifier,
		Result:             result,
		ServiceTimestampMs: time.Now().UnixMilli(),
	}
}

// ---------------------------------------------------------------------------
// Callback / executor function types
// ---------------------------------------------------------------------------

// ReplyCallback is called to send a reply: (resultCode, data).
type ReplyCallback func(resultCode int, data []byte)

// ServiceExecutor handles an incoming service invocation.
// The executor receives the request and a ReplyCallback to send the response.
type ServiceExecutor func(request *DeviceServiceRequest, reply ReplyCallback)

// PlatformResponseCallback is invoked when a platform service response arrives.
type PlatformResponseCallback func(response *PlatformServiceResponse)

// ServiceResponseCallback is invoked when a device service response arrives.
type ServiceResponseCallback func(response *DeviceServiceResponse)
