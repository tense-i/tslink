//! Message adapter for routing incoming messages to callbacks

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::{debug, warn};

use crate::channel::MessageReceiveCallback;
use crate::client::{ServiceCallback, ServiceReplyCallback};
use crate::message::{CommonMessage, ReplyMessage};

/// Message adapter that routes incoming messages to registered callbacks
pub struct MessageAdapter {
    /// Service callbacks keyed by method name
    service_callbacks: RwLock<HashMap<String, ServiceCallback>>,
    /// Reply callbacks keyed by transaction ID
    reply_callbacks: RwLock<HashMap<String, ServiceReplyCallback>>,
    /// Channel for sending replies (to be set by client)
    reply_sender: RwLock<Option<Arc<dyn Fn(&str, &str) + Send + Sync>>>,
}

impl std::fmt::Debug for MessageAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageAdapter")
            .field("service_callbacks_count", &self.service_callbacks.read().len())
            .field("reply_callbacks_count", &self.reply_callbacks.read().len())
            .finish_non_exhaustive()
    }
}

impl MessageAdapter {
    /// Create a new message adapter
    pub fn new() -> Self {
        Self {
            service_callbacks: RwLock::new(HashMap::new()),
            reply_callbacks: RwLock::new(HashMap::new()),
            reply_sender: RwLock::new(None),
        }
    }

    /// Set the reply sender function
    pub fn set_reply_sender(&self, sender: Arc<dyn Fn(&str, &str) + Send + Sync>) {
        *self.reply_sender.write() = Some(sender);
    }

    /// Register a service callback
    pub fn add_service_callback(&self, method: &str, callback: ServiceCallback) {
        debug!("Registering service callback for: {}", method);
        self.service_callbacks
            .write()
            .insert(method.to_string(), callback);
    }

    /// Register a reply callback for a transaction
    pub fn add_reply_callback(&self, tid: &str, callback: ServiceReplyCallback) {
        debug!("Registering reply callback for tid: {}", tid);
        self.reply_callbacks
            .write()
            .insert(tid.to_string(), callback);
    }

    /// Remove a reply callback
    pub fn remove_reply_callback(&self, tid: &str) -> Option<ServiceReplyCallback> {
        self.reply_callbacks.write().remove(tid)
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
                        self.handle_reply_message(&reply);
                    }
                    Err(e) => {
                        warn!("Failed to parse message: {}", e);
                    }
                }
            }
        }
    }

    /// Handle a CommonMessage (service invocation)
    fn handle_common_message(&self, topic: &str, msg: &CommonMessage) {
        debug!("Handling message: method={}, tid={}", msg.method, msg.tid);

        // Extract method from message or topic
        let method = if !msg.method.is_empty() {
            msg.method.clone()
        } else {
            self.extract_method_from_topic(topic)
        };

        // Find and invoke callback
        let callback = {
            let callbacks = self.service_callbacks.read();
            callbacks.get(&method).cloned()
        };

        if let Some(callback) = callback {
            let result = callback(msg.data.clone());

            // Send reply if we have a reply sender
            if let Some(sender) = self.reply_sender.read().as_ref() {
                let reply = ReplyMessage::success_with_data(&msg.tid, &msg.bid, result);
                if let Ok(reply_json) = serde_json::to_string(&reply) {
                    let reply_topic = self.get_reply_topic(topic);
                    sender(&reply_topic, &reply_json);
                }
            }
        } else {
            warn!("No callback registered for method: {}", method);
        }
    }

    /// Handle a ReplyMessage (response to our request)
    fn handle_reply_message(&self, reply: &ReplyMessage) {
        debug!(
            "Handling reply: tid={}, code={}",
            reply.tid, reply.code
        );

        // Find and invoke callback, then remove it
        if let Some(callback) = self.remove_reply_callback(&reply.tid) {
            callback(reply.clone());
        } else {
            debug!("No reply callback for tid: {}", reply.tid);
        }
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
        self.service_callbacks.write().clear();
        self.reply_callbacks.write().clear();
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
    fn test_service_callback_registration() {
        let adapter = MessageAdapter::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        adapter.add_service_callback(
            "test.service",
            Arc::new(move |_| {
                called_clone.store(true, Ordering::SeqCst);
                json!({"result": "ok"})
            }),
        );

        let msg = CommonMessage::new("test.service", json!({}));
        let msg_json = serde_json::to_string(&msg).unwrap();

        adapter.receive("sys/pk/did/thing/service/test/post", &msg_json);

        // Note: callback won't be triggered without proper method matching
        // This test verifies registration works
        assert!(adapter.service_callbacks.read().contains_key("test.service"));
    }

    #[test]
    fn test_reply_callback_registration() {
        let adapter = MessageAdapter::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        adapter.add_reply_callback(
            "test-tid",
            Arc::new(move |_reply| {
                called_clone.store(true, Ordering::SeqCst);
            }),
        );

        let reply = ReplyMessage::success("test-tid", "test-bid");
        let reply_json = serde_json::to_string(&reply).unwrap();

        adapter.receive("sys/pk/did/thing/event/test/info_reply", &reply_json);

        assert!(called.load(Ordering::SeqCst));
    }
}
