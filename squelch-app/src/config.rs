//! Application configuration — loaded from a TOML file or defaults.

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Application configuration persisted between sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Matrix homeserver URL (e.g. `https://matrix.org`).
    pub homeserver: String,
    /// Matrix username (local part, without `@` and domain).
    pub username: String,
    /// Configured PTT key string (e.g. `"CapsLock"`, `"F1"`, `"ctrl+shift+Space"`).
    /// `None` means not yet configured.
    pub ptt_key: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            homeserver: "https://matrix.org".into(),
            username: String::new(),
            ptt_key: None,
        }
    }
}

impl AppConfig {
    /// Path to the config file: `~/.config/squelch/config.toml`.
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("squelch")
            .join("config.toml")
    }

    /// Load config from disk, falling back to defaults if not found.
    pub fn load() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)?;
        let cfg = toml::from_str(&raw).unwrap_or_else(|e| {
            warn!("config parse error ({e}), using defaults");
            Self::default()
        });
        Ok(cfg)
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(&path, raw)?;
        Ok(())
    }
}
