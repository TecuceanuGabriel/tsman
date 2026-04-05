//! User configuration loaded from `~/.config/tsman/config.toml`.
//!
//! Precedence: CLI flag > env var > config file > default.
use std::{fs, path::PathBuf};

use anyhow::Result;
use dirs::home_dir;
use serde::Deserialize;

const CONFIG_PATH: &str = ".config/tsman/config.toml";

/// Top-level config struct, mirroring `config.toml` sections.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub menu: MenuConfig,
    pub storage: StorageConfig,
}

/// `[menu]` section - persistent UI preferences.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct MenuConfig {
    pub preview: bool,
    pub ask_for_confirmation: bool,
}

impl Default for MenuConfig {
    fn default() -> Self {
        Self {
            preview: false,
            ask_for_confirmation: false,
        }
    }
}

/// `[storage]` section - override default storage directories.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub sessions_dir: Option<PathBuf>,
    pub layouts_dir: Option<PathBuf>,
}

impl Config {
    /// Load config from `~/.config/tsman/config.toml`.
    ///
    /// Returns `Config::default()` if the file does not exist.
    /// Returns an error only if the file exists but cannot be parsed.
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)?;
        let config: Self = toml::from_str(&raw)?;
        Ok(config)
    }
}

fn config_path() -> Result<PathBuf> {
    let home = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine HOME directory"))?;
    Ok(home.join(CONFIG_PATH))
}
