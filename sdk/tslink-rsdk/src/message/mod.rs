//! Message types for IoT communication

mod common;
mod reply;
pub mod service;

pub use common::CommonMessage;
pub use reply::ReplyMessage;
pub use service::{
    DeviceServiceRequest, DeviceServiceResponse, PlatformServiceRequest,
    PlatformServiceResponse, PlatformResponseCallback, ReplyCallback,
    ServiceExecutor, ServiceResponseCallback,
};
