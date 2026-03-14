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
    #[serde(default)]
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputConfig {
    /// Output backend: "auto", "xdotool", "wtype", "fcitx5", "ibus"
    #[serde(default = "default_backend")]
    pub backend: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
        }
    }
}

fn default_backend() -> String {
    "auto".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_toml() {
        let toml = r#"
keymap = "keymaps/qwerty.toml"

[stroke]
timeout_ms = 200

[languages]
default = "japanese"
switch_stroke = "LANG"

[languages.japanese]
theory = "ja-stenoword"
dictionary = ["theories/ja-stenoword/dict_base.json"]

[languages.english]
theory = "en-plover"
dictionary = ["theories/en-plover/dict_base.json"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.keymap, "keymaps/qwerty.toml");
        assert_eq!(config.stroke.timeout_ms, 200);
        assert_eq!(config.languages.default, "japanese");
        assert_eq!(config.languages.switch_stroke, "LANG");
        assert!(config.languages.languages.contains_key("japanese"));
        assert!(config.languages.languages.contains_key("english"));

        let ja = &config.languages.languages["japanese"];
        assert_eq!(ja.theory, "ja-stenoword");
        assert_eq!(ja.dictionary, vec!["theories/ja-stenoword/dict_base.json"]);
    }

    #[test]
    fn parse_output_config() {
        let toml = r#"
keymap = "keymaps/qwerty.toml"

[output]
backend = "fcitx5"

[languages]
default = "japanese"
switch_stroke = "LANG"

[languages.japanese]
theory = "ja-stenoword"
dictionary = []
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.output.backend, "fcitx5");
    }

    #[test]
    fn default_output_backend() {
        let toml = r#"
keymap = "keymaps/qwerty.toml"

[languages]
default = "japanese"
switch_stroke = "LANG"

[languages.japanese]
theory = "ja-stenoword"
dictionary = []
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.output.backend, "auto");
    }

    #[test]
    fn default_stroke_timeout() {
        let toml = r#"
keymap = "keymaps/qwerty.toml"

[languages]
default = "japanese"
switch_stroke = "LANG"

[languages.japanese]
theory = "ja-stenoword"
dictionary = []
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.stroke.timeout_ms, 200);
    }
}
