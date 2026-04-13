use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Runtime configuration for the zig API server.
///
/// This is the struct consumed by `start_server()`. It is assembled by
/// `zig-cli` from CLI flags, environment variables, and the optional global
/// config file (`FileConfig`) with the precedence:
/// CLI flag > env var > config file > built-in default.
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
    /// Serve the embedded React web UI from `/`.
    pub web: bool,
}

/// Global config file backing `~/.zig/serve.toml`.
///
/// Every field is optional so partial files are valid. Used by `zig-cli` to
/// pre-populate `ServeConfig` before applying CLI and environment overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileConfig {
    #[serde(default)]
    pub server: ServerSection,
}

/// `[server]` section of the global config file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerSection {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub token: Option<String>,
    /// Shutdown drain timeout in whole seconds.
    pub shutdown_timeout: Option<u64>,
    #[serde(default)]
    pub tls: bool,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
    pub rate_limit: Option<u64>,
    /// Serve the embedded React web UI from `/`.
    #[serde(default)]
    pub web: bool,
}

impl FileConfig {
    /// Returns the absolute path to the global config file
    /// (`~/.zig/serve.toml`). Falls back to a relative `.zig/serve.toml`
    /// if `HOME` is unset, mirroring `zig_core::paths::global_base_dir`.
    pub fn config_path() -> PathBuf {
        zig_core::paths::global_base_dir()
            .unwrap_or_else(|| PathBuf::from(".zig"))
            .join("serve.toml")
    }

    /// Load the global config file. Returns a default (empty) config if the
    /// file is missing or unreadable, so callers can treat it as opt-in.
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::config_path()) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Serialize to TOML and write atomically. Creates the parent directory
    /// if it doesn't already exist.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(path, serialized)
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
