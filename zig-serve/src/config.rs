/// Configuration for the zig API server.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Host/IP to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Bearer token for authentication.
    pub token: String,
}
