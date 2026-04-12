use std::time::Duration;

/// Configuration for the zig API server.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Host/IP to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Bearer token for authentication.
    pub token: String,
    /// Maximum time to wait for in-flight requests during shutdown.
    pub shutdown_timeout: Duration,
    /// Enable TLS with auto-generated self-signed certificates.
    pub tls: bool,
    /// Path to a TLS certificate PEM file.
    pub tls_cert: Option<String>,
    /// Path to a TLS private key PEM file.
    pub tls_key: Option<String>,
    /// Rate limit in requests per second (None = no limit).
    pub rate_limit: Option<u64>,
}
