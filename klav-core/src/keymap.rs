use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::stroke::StenoKey;

/// Maps physical key codes (evdev KEY_*) to logical steno keys.
#[derive(Debug, Clone)]
pub struct KeyMap {
    mapping: HashMap<u16, StenoKey>,
}

/// Raw TOML representation.
#[derive(Debug, Deserialize)]
struct KeyMapFile {
    keymap: HashMap<String, String>,
}

impl KeyMap {
    pub fn load(path: &Path) -> Result<Self, KeyMapError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| KeyMapError::Io(path.to_path_buf(), e))?;
        Self::from_toml(&content)
    }

    pub fn from_toml(content: &str) -> Result<Self, KeyMapError> {
        let file: KeyMapFile = toml::from_str(content)
            .map_err(KeyMapError::Parse)?;

        let mut mapping = HashMap::new();
        for (evdev_name, steno_name) in &file.keymap {
            let code = evdev_name_to_code(evdev_name)
                .ok_or_else(|| KeyMapError::UnknownEvdevKey(evdev_name.clone()))?;
            let key = steno_name_to_key(steno_name)
                .ok_or_else(|| KeyMapError::UnknownStenoKey(steno_name.clone()))?;
            mapping.insert(code, key);
        }

        Ok(Self { mapping })
    }

    /// Look up the steno key for a given evdev key code.
    pub fn get(&self, evdev_code: u16) -> Option<StenoKey> {
        self.mapping.get(&evdev_code).copied()
    }

    /// All evdev codes that are mapped (for grab filtering).
    pub fn mapped_codes(&self) -> impl Iterator<Item = u16> + '_ {
        self.mapping.keys().copied()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KeyMapError {
    #[error("failed to read keymap file {0}: {1}")]
    Io(std::path::PathBuf, std::io::Error),
    #[error("failed to parse keymap TOML: {0}")]
    Parse(toml::de::Error),
    #[error("unknown evdev key name: {0}")]
    UnknownEvdevKey(String),
    #[error("unknown steno key name: {0}")]
    UnknownStenoKey(String),
}

/// Convert an evdev key name like "KEY_Q" to its numeric code.
fn evdev_name_to_code(name: &str) -> Option<u16> {
    // Common keycodes from <linux/input-event-codes.h>
    let code = match name {
        "KEY_Q" => 16,
        "KEY_W" => 17,
        "KEY_E" => 18,
        "KEY_R" => 19,
        "KEY_T" => 20,
        "KEY_Y" => 21,
        "KEY_U" => 22,
        "KEY_I" => 23,
        "KEY_O" => 24,
        "KEY_P" => 25,
        "KEY_A" => 30,
        "KEY_S" => 31,
        "KEY_D" => 32,
        "KEY_F" => 33,
        "KEY_G" => 34,
        "KEY_H" => 35,
        "KEY_J" => 36,
        "KEY_K" => 37,
        "KEY_L" => 38,
        "KEY_SEMICOLON" => 39,
        "KEY_Z" => 44,
        "KEY_X" => 45,
        "KEY_C" => 46,
        "KEY_V" => 47,
        "KEY_B" => 48,
        "KEY_N" => 49,
        "KEY_M" => 50,
        "KEY_COMMA" => 51,
        "KEY_DOT" => 52,
        "KEY_SPACE" => 57,
        "KEY_BACKSPACE" => 14,
        "KEY_TAB" => 15,
        "KEY_LEFTSHIFT" => 42,
        "KEY_RIGHTSHIFT" => 54,
        "KEY_LEFTCTRL" => 29,
        "KEY_RIGHTCTRL" => 97,
        "KEY_1" => 2,
        "KEY_2" => 3,
        "KEY_3" => 4,
        "KEY_4" => 5,
        "KEY_5" => 6,
        "KEY_6" => 7,
        "KEY_7" => 8,
        "KEY_8" => 9,
        "KEY_9" => 10,
        "KEY_0" => 11,
        _ => return None,
    };
    Some(code)
}

/// Convert a steno key name like "S1" to a `StenoKey`.
fn steno_name_to_key(name: &str) -> Option<StenoKey> {
    let key = match name {
        "S1" => StenoKey::S1,
        "T1" => StenoKey::T1,
        "K1" => StenoKey::K1,
        "P1" => StenoKey::P1,
        "W1" => StenoKey::W1,
        "H1" => StenoKey::H1,
        "R1" => StenoKey::R1,
        "A" => StenoKey::A,
        "O" => StenoKey::O,
        "E" => StenoKey::E,
        "U" => StenoKey::U,
        "F1" => StenoKey::F1,
        "P2" => StenoKey::P2,
        "L1" => StenoKey::L1,
        "T2" => StenoKey::T2,
        "D1" => StenoKey::D1,
        "R2" => StenoKey::R2,
        "B1" => StenoKey::B1,
        "G1" => StenoKey::G1,
        "S2" => StenoKey::S2,
        "Z1" => StenoKey::Z1,
        "*" | "STAR" => StenoKey::Star,
        "VOICED" | "#V" => StenoKey::Voiced,
        "HALF_VOICED" | "#H" => StenoKey::HalfVoiced,
        "LANG" => StenoKey::Lang,
        "UNDO" => StenoKey::Undo,
        _ => return None,
    };
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keymap_toml() {
        let toml = r#"
[keymap]
"KEY_Q" = "S1"
"KEY_C" = "A"
"KEY_V" = "O"
"KEY_SPACE" = "LANG"
"#;
        let km = KeyMap::from_toml(toml).unwrap();
        assert_eq!(km.get(16), Some(StenoKey::S1)); // KEY_Q
        assert_eq!(km.get(46), Some(StenoKey::A));  // KEY_C
        assert_eq!(km.get(57), Some(StenoKey::Lang)); // KEY_SPACE
        assert_eq!(km.get(99), None);
    }
}
