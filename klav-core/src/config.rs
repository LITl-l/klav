use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Top-level Klav configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub keymap: String,
    pub languages: LanguagesConfig,
    #[serde(default)]
    pub stroke: StrokeConfig,
}

#[derive(Debug, Deserialize)]
pub struct LanguagesConfig {
    pub default: String,
    pub switch_stroke: String,
    #[serde(flatten)]
    pub languages: HashMap<String, LanguageConfig>,
}

#[derive(Debug, Deserialize)]
pub struct LanguageConfig {
    pub theory: String,
    pub dictionary: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct StrokeConfig {
    /// Stroke timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

impl Default for StrokeConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout_ms(),
        }
    }
}

fn default_timeout_ms() -> u64 {
    200
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(path.to_path_buf(), e))?;
        let config: Config = toml::from_str(&content)
            .map_err(ConfigError::Parse)?;
        Ok(config)
    }

    /// Resolve a relative path against the config file's directory.
    pub fn resolve_path(config_dir: &Path, relative: &str) -> PathBuf {
        config_dir.join(relative)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("failed to parse config TOML: {0}")]
    Parse(toml::de::Error),
}
