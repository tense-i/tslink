use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

/// Top-level application configuration.
///
/// Loaded from TOML files with environment variable overrides:
/// - `config/default.toml` (always loaded)
/// - `config/{RUN_ENV}.toml` (loaded if `RUN_ENV` is set, e.g. `dev`, `staging`, `production`)
/// - Environment variables with `TSLINK__` prefix (double underscore as separator)
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct AppConfig {
    pub app: AppSettings,
    pub mqtt: MqttConfig,
    pub redis: RedisConfig,
    pub kafka: KafkaConfig,
    pub database: DatabaseConfig,
    pub http: HttpConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    pub name: String,
    pub env: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_keep_alive")]
    pub keep_alive_secs: u64,
    #[serde(default = "default_true")]
    pub clean_session: bool,
    #[serde(default = "default_max_packet_size")]
    pub max_packet_size: usize,
    #[serde(default = "default_inflight")]
    pub inflight: u16,
    #[serde(default)]
    pub subscribe_topics: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct RedisConfig {
    pub url: String,
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,
    #[serde(default = "default_key_prefix")]
    pub key_prefix: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KafkaConfig {
    pub brokers: String,
    #[serde(default = "default_topic_prefix")]
    pub topic_prefix: String,
    #[serde(default = "default_event_topic")]
    pub event_topic: String,
    #[serde(default = "default_property_topic")]
    pub property_topic: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct HttpConfig {
    #[serde(default = "default_http_host")]
    pub host: String,
    #[serde(default = "default_http_port")]
    pub port: u16,
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,
}

// ── Default value functions ──────────────────────────────────────

fn default_keep_alive() -> u64 {
    30
}
fn default_true() -> bool {
    true
}
fn default_max_packet_size() -> usize {
    65536
}
fn default_inflight() -> u16 {
    100
}
fn default_pool_size() -> usize {
    8
}
fn default_key_prefix() -> String {
    "tslink:".to_string()
}
fn default_topic_prefix() -> String {
    "tslink.".to_string()
}
fn default_event_topic() -> String {
    "tslink.device.event".to_string()
}
fn default_property_topic() -> String {
    "tslink.device.property".to_string()
}
fn default_max_connections() -> u32 {
    10
}
fn default_min_connections() -> u32 {
    2
}
fn default_connect_timeout() -> u64 {
    10
}
fn default_http_host() -> String {
    "0.0.0.0".to_string()
}
fn default_http_port() -> u16 {
    8080
}
fn default_request_timeout() -> u64 {
    30
}

/// Thread-safe shared config handle for hot reload.
pub type SharedConfig = std::sync::Arc<tokio::sync::RwLock<AppConfig>>;

// ── Config loading ───────────────────────────────────────────────

impl AppConfig {
    /// Load configuration from files and environment variables.
    ///
    /// Priority (highest to lowest):
    /// 1. Environment variables (`TSLINK__SECTION__KEY`)
    /// 2. Environment-specific TOML (`config/{RUN_ENV}.toml`)
    /// 3. Default TOML (`config/default.toml`)
    pub fn load() -> Result<Self, ConfigError> {
        let run_env = std::env::var("RUN_ENV").unwrap_or_else(|_| "dev".to_string());

        let config = Config::builder()
            // Start with default config
            .add_source(File::with_name("config/default").required(true))
            // Layer on environment-specific config
            .add_source(File::with_name(&format!("config/{}", run_env)).required(false))
            // Override with environment variables: TSLINK__MQTT__HOST → mqtt.host
            .add_source(
                Environment::with_prefix("TSLINK")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    /// Create a shared config handle for hot reload.
    pub fn into_shared(self) -> SharedConfig {
        std::sync::Arc::new(tokio::sync::RwLock::new(self))
    }

    /// Reload configuration from disk. Returns the new config or an error.
    ///
    /// Usage: call on SIGHUP to hot-reload TOML files + env overrides.
    pub fn reload() -> Result<Self, ConfigError> {
        Self::load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        assert_eq!(default_keep_alive(), 30);
        assert_eq!(default_http_port(), 8080);
        assert_eq!(default_pool_size(), 8);
        assert_eq!(default_key_prefix(), "tslink:");
    }
}
