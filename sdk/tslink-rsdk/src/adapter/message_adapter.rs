//! Message adapter for routing incoming messages to callbacks

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;
use tracing::{debug, warn};

use crate::channel::MessageReceiveCallback;
use crate::message::service::{
    DeviceServiceRequest, DeviceServiceResponse, PlatformServiceResponse, ReplyCallback,
    ServiceExecutor,
};
use crate::message::{CommonMessage, ReplyMessage};

/// Reply handler: either a platform response callback or a device response callback
#[derive(Clone)]
pub enum ReplyHandler {
    Platform(Arc<dyn Fn(PlatformServiceResponse) + Send + Sync>),
    Device(Arc<dyn Fn(DeviceServiceResponse) + Send + Sync>),
}

/// Message adapter that routes incoming messages to registered callbacks
pub struct MessageAdapter {
    /// Service executors keyed by method name (e.g. "service.{id}.post")
    service_executors: RwLock<HashMap<String, ServiceExecutor>>,
    /// Unified service executor (fallback when no specific executor matches)
    unified_executor: RwLock<Option<ServiceExecutor>>,
    /// Reply handlers keyed by transaction ID
    reply_handlers: RwLock<HashMap<String, ReplyHandler>>,
    /// Channel for sending replies (to be set by client)
    reply_sender: RwLock<Option<Arc<dyn Fn(&str, &str) + Send + Sync>>>,
}

impl std::fmt::Debug for MessageAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageAdapter")
            .field("service_executors_count", &self.service_executors.read().len())
            .field("reply_handlers_count", &self.reply_handlers.read().len())
            .finish_non_exhaustive()
    }
}

impl MessageAdapter {
    /// Create a new message adapter
    pub fn new() -> Self {
        Self {
            service_executors: RwLock::new(HashMap::new()),
            unified_executor: RwLock::new(None),
            reply_handlers: RwLock::new(HashMap::new()),
            reply_sender: RwLock::new(None),
        }
    }

    /// Set the reply sender function
    pub fn set_reply_sender(&self, sender: Arc<dyn Fn(&str, &str) + Send + Sync>) {
        *self.reply_sender.write() = Some(sender);
    }

    /// Register a service executor for a specific method
    pub fn add_service_executor(&self, method: &str, executor: ServiceExecutor) {
        debug!("Registering service executor for: {}", method);
        self.service_executors
            .write()
            .insert(method.to_string(), executor);
    }

    /// Register a unified (fallback) service executor
    pub fn set_unified_executor(&self, executor: ServiceExecutor) {
        debug!("Registering unified service executor");
        *self.unified_executor.write() = Some(executor);
    }

    /// Register a reply handler for a transaction
    pub fn add_reply_handler(&self, tid: &str, handler: ReplyHandler) {
        debug!("Registering reply handler for tid: {}", tid);
        self.reply_handlers
            .write()
            .insert(tid.to_string(), handler);
    }

    /// Remove a reply handler
    pub fn remove_reply_handler(&self, tid: &str) -> Option<ReplyHandler> {
        self.reply_handlers.write().remove(tid)
    }

    /// Handle incoming message and route to appropriate callback
    fn handle_message(&self, topic: &str, data: &str) {
        // Try to parse as CommonMessage first
        match serde_json::from_str::<CommonMessage>(data) {
            Ok(msg) => {
                self.handle_common_message(topic, &msg);
            }
            Err(_) => {
                // Try to parse as ReplyMessage
                match serde_json::from_str::<ReplyMessage>(data) {
                    Ok(reply) => {
                        self.handle_reply_message(topic, &reply);
                    }
                    Err(e) => {
                        warn!("Failed to parse message: {}", e);
                    }
                }
            }
        }
    }

    /// Handle a CommonMessage (service invocation) using ServiceExecutor
    fn handle_common_message(&self, topic: &str, msg: &CommonMessage) {
        debug!("Handling message: method={}, tid={}", msg.method, msg.tid);

        // Extract method from message or topic
        let method = if !msg.method.is_empty() {
            msg.method.clone()
        } else {
            self.extract_method_from_topic(topic)
        };

        // Extract service identifier from method (e.g. "service.xxx.post" -> "xxx")
        let service_identifier = self.extract_service_identifier(&method);

        // Build DeviceServiceRequest from CommonMessage
        let param_data = self.value_to_bytes(&msg.data);
        let request = DeviceServiceRequest {
            channel: crate::enums::CommunicationChannel::default(),
            service_identifier: service_identifier.clone(),
            param_data,
            service_timestamp_ms: msg.timestamp as i64,
        };

        // Find specific executor: try exact method, then "service.{method}.post" format
        let executor = {
            let executors = self.service_executors.read();
            executors.get(&method).cloned().or_else(|| {
                let alt_key = format!("service.{}.post", method);
                executors.get(&alt_key).cloned()
            }).or_else(|| {
                // Also try matching by service_identifier directly
                let alt_key2 = format!("service.{}.post", service_identifier);
                executors.get(&alt_key2).cloned()
            })
        };
        let executor = executor.or_else(|| self.unified_executor.read().clone());

        if let Some(executor) = executor {
            // Build reply callback that sends reply via MQTT
            let reply_sender = self.reply_sender.read().clone();
            let reply_topic = self.get_reply_topic(topic);
            let tid = msg.tid.clone();
            let bid = msg.bid.clone();

            let reply_cb: ReplyCallback = Arc::new(move |result_code, data| {
                if let Some(sender) = reply_sender.as_ref() {
                    let reply = ReplyMessage {
                        tid: tid.clone(),
                        bid: bid.clone(),
                        code: result_code,
                        message: if result_code == 0 {
                            "success".to_string()
                        } else {
                            "error".to_string()
                        },
                        data: Some(serde_json::Value::String(
                            String::from_utf8_lossy(&data).to_string(),
                        )),
                    };
                    if let Ok(reply_json) = serde_json::to_string(&reply) {
                        sender(&reply_topic, &reply_json);
                    }
                }
            });

            executor(request, reply_cb);
        } else {
            warn!("No executor registered for method: {}", method);
        }
    }

    /// Handle a ReplyMessage (response to our request)
    fn handle_reply_message(&self, topic: &str, reply: &ReplyMessage) {
        debug!(
            "Handling reply: tid={}, code={}",
            reply.tid, reply.code
        );

        if let Some(handler) = self.remove_reply_handler(&reply.tid) {
            let param_data = reply
                .data
                .as_ref()
                .map(|v| self.value_to_bytes(v))
                .unwrap_or_default();
            let identifier = self.extract_service_identifier_from_topic(topic);

            match handler {
                ReplyHandler::Platform(cb) => {
                    let resp = PlatformServiceResponse {
                        channel: crate::enums::CommunicationChannel::default(),
                        service_identifier: identifier,
                        result: reply.code,
                        param_data,
                        service_timestamp_ms: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64,
                    };
                    cb(resp);
                }
                ReplyHandler::Device(cb) => {
                    let resp = DeviceServiceResponse {
                        channel: crate::enums::CommunicationChannel::default(),
                        service_identifier: identifier,
                        result: reply.code,
                        param_data,
                        service_timestamp_ms: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64,
                    };
                    cb(resp);
                }
            }
        } else {
            debug!("No reply handler for tid: {}", reply.tid);
        }
    }

    /// Convert serde_json::Value to bytes
    fn value_to_bytes(&self, value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap_or_default()
    }

    /// Extract method name from topic
    fn extract_method_from_topic(&self, topic: &str) -> String {
        // Topic format: sys/{pk}/{did}/thing/service/{identity}/post
        // Extract: service.{identity}.post
        let parts: Vec<&str> = topic.split('/').collect();

        if parts.len() >= 7 && parts[4] == "service" {
            format!("service.{}.post", parts[5])
        } else if topic.contains("properties/set") {
            "thing.properties.set".to_string()
        } else {
            topic.to_string()
        }
    }

    /// Extract service identifier from method string
    fn extract_service_identifier(&self, method: &str) -> String {
        // method format: "service.{identity}.post" or "platform.service.{identity}.post"
        let parts: Vec<&str> = method.split('.').collect();
        if parts.len() >= 3 && parts[0] == "platform" {
            parts[2].to_string()
        } else if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            method.to_string()
        }
    }

    /// Extract service identifier from topic
    fn extract_service_identifier_from_topic(&self, topic: &str) -> String {
        // Topic: sys/{pk}/{did}/platform/service/{identity}/post_reply
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() >= 6 {
            parts[5].to_string()
        } else {
            String::new()
        }
    }

    /// Get reply topic from request topic
    fn get_reply_topic(&self, topic: &str) -> String {
        if topic.ends_with("/post") {
            format!("{}_reply", topic)
        } else {
            format!("{}/reply", topic)
        }
    }

    /// Release all resources
    pub fn release(&self) {
        self.service_executors.write().clear();
        *self.unified_executor.write() = None;
        self.reply_handlers.write().clear();
        *self.reply_sender.write() = None;
    }
}

impl Default for MessageAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageReceiveCallback for MessageAdapter {
    fn receive(&self, topic: &str, data: &str) {
        self.handle_message(topic, data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_service_executor_registration() {
        let adapter = MessageAdapter::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let executor: ServiceExecutor = Arc::new(move |req, reply_cb| {
            called_clone.store(true, Ordering::SeqCst);
            assert_eq!(req.service_identifier, "test");
            reply_cb(0, b"ok".to_vec());
        });

        adapter.add_service_executor("service.test.post", executor);

        assert!(adapter.service_executors.read().contains_key("service.test.post"));
    }

    #[test]
    fn test_unified_executor_fallback() {
        let adapter = MessageAdapter::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let executor: ServiceExecutor = Arc::new(move |_req, _reply_cb| {
            called_clone.store(true, Ordering::SeqCst);
        });

        adapter.set_unified_executor(executor);

        let msg = CommonMessage::new("service.unknown.post", json!({}));
        let msg_json = serde_json::to_string(&msg).unwrap();

        adapter.receive("sys/pk/did/thing/service/unknown/post", &msg_json);

        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_reply_handler_platform() {
        let adapter = MessageAdapter::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        adapter.add_reply_handler(
            "test-tid",
            ReplyHandler::Platform(Arc::new(move |resp| {
                called_clone.store(true, Ordering::SeqCst);
                assert_eq!(resp.result, 0);
            })),
        );

        let reply = ReplyMessage::success("test-tid", "test-bid");
        let reply_json = serde_json::to_string(&reply).unwrap();

        adapter.receive("sys/pk/did/platform/service/svc1/post_reply", &reply_json);

        assert!(called.load(Ordering::SeqCst));
    }
}
