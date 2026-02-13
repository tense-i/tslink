//! Smart client interfaces and implementations

mod builder;
mod default_client;
mod smart_client;

pub use builder::TslinkClientBuilder;
pub use default_client::DefaultTslinkClient;
pub use smart_client::{ServiceCallback, ServiceReplyCallback, TslinkClient};
