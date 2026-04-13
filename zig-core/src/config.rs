//! Global zig configuration loaded from `~/.zig/config.toml`.
//!
//! Every field is optional so partial files are valid. Missing or unreadable
//! files fall back to built-in defaults.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level config structure backing `~/.zig/config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZigConfig {
    #[serde(default)]
    pub memory: MemorySection,
}

/// `[memory]` section of the global config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySection {
    /// Whether project-local memory (`<git-root>/.zig/memory/`) is enabled.
    /// When `false`, only the global tiers are used.
    #[serde(default = "default_true")]
    pub local: bool,
}

fn default_true() -> bool {
    true
}

impl Default for MemorySection {
    fn default() -> Self {
        Self { local: true }
    }
}

impl ZigConfig {
    /// Returns the absolute path to the global config file (`~/.zig/config.toml`).
    pub fn config_path() -> PathBuf {
        crate::paths::global_base_dir()
            .unwrap_or_else(|| PathBuf::from(".zig"))
            .join("config.toml")
    }

    /// Load the global config file. Returns a default (empty) config if the
    /// file is missing or unreadable, so callers can treat it as opt-in.
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::config_path()) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
