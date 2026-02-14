//! Device discovery module for IPC-based local device discovery
//!
//! This module provides functionality to discover and track devices
//! on the same machine using IPC pub/sub messaging.

mod device_discovery;

pub use device_discovery::{DeviceDiscovery, DeviceDiscoveryConfig, DeviceInfo, DeviceStatusCallback};
