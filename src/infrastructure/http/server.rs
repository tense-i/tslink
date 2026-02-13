use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::config::HttpConfig;
use crate::error::Result;

/// Start the HTTP server with the given router.
///
/// Binds to the configured host:port and serves axum routes
/// with CORS and tracing middleware.
pub async fn start_http_server(config: &HttpConfig, router: Router) -> Result<()> {
    let app = router
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|e| {
            crate::error::TsLinkError::Internal(format!("invalid http bind address: {}", e))
        })?;

    info!(addr = %addr, "HTTP server starting");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| crate::error::TsLinkError::Internal(format!("failed to bind HTTP: {}", e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::TsLinkError::Internal(format!("HTTP server error: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_addr_parse() {
        let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
        assert_eq!(addr.port(), 8080);
    }
}
