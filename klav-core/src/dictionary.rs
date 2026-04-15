use std::collections::HashMap;
use std::path::Path;

use crate::stroke::Stroke;

/// A steno dictionary mapping stroke sequences to output text.
///
/// Multiple dictionaries can be layered with priority (user dict > base dict).
#[derive(Debug, Clone)]
pub struct Dictionary {
    /// Maps a canonical stroke string to output text.
    entries: HashMap<String, String>,
    name: String,
}

impl Dictionary {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            entries: HashMap::new(),
            name: name.into(),
        }
    }

    /// Load a dictionary from a JSON file.
    /// Format: `{ "STROKE": "output", "STROKE/STROKE": "output", ... }`
    pub fn load_json(path: &Path) -> Result<Self, DictionaryError> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = std::fs::read_to_string(path)
            .map_err(|e| DictionaryError::Io(path.to_path_buf(), e))?;
        let entries: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| DictionaryError::Parse(path.to_path_buf(), e))?;

        log::info!(
            "loaded dictionary '{}' with {} entries",
            name,
            entries.len()
        );

        Ok(Self { entries, name })
    }

    /// Load a dictionary from a Plover-format JSON file.
    /// Plover stroke notation (e.g. "THAT") is converted to Klav canonical form (e.g. "THA-T").
    /// Entries with "comment" key are skipped.
    pub fn load_plover_json(path: &Path) -> Result<Self, DictionaryError> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = std::fs::read_to_string(path)
            .map_err(|e| DictionaryError::Io(path.to_path_buf(), e))?;
        let raw: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| DictionaryError::Parse(path.to_path_buf(), e))?;

        let mut entries = HashMap::new();
        let mut skipped = 0u32;
        for (plover_key, value) in raw {
            if plover_key == "comment" {
                continue;
            }
            match Stroke::plover_to_canonical(&plover_key) {
                Some(canonical) => {
                    entries.insert(canonical, value);
                }
                None => {
                    log::warn!("skipping unparseable Plover stroke: {plover_key}");
                    skipped += 1;
                }
            }
        }

        log::info!(
            "loaded Plover dictionary '{}' with {} entries ({} skipped)",
            name,
            entries.len(),
            skipped
        );

        Ok(Self { entries, name })
    }

    /// Look up a stroke string in this dictionary.
    pub fn lookup(&self, stroke_str: &str) -> Option<&str> {
        self.entries.get(stroke_str).map(|s| s.as_str())
    }

    /// Insert or update an entry.
    pub fn insert(&mut self, stroke_str: impl Into<String>, output: impl Into<String>) {
        self.entries.insert(stroke_str.into(), output.into());
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A stack of dictionaries with priority-based lookup.
#[derive(Debug)]
pub struct DictionaryStack {
    /// Dictionaries in priority order (first = highest priority).
    dicts: Vec<Dictionary>,
}

impl DictionaryStack {
    pub fn new() -> Self {
        Self { dicts: Vec::new() }
    }

    /// Push a dictionary onto the stack (highest priority).
    pub fn push(&mut self, dict: Dictionary) {
        self.dicts.insert(0, dict);
    }

    /// Push a dictionary at the bottom of the stack (lowest priority).
    pub fn push_back(&mut self, dict: Dictionary) {
        self.dicts.push(dict);
    }

    /// Look up a stroke string across all dictionaries.
    /// Returns the first match found (highest priority).
    pub fn lookup(&self, stroke_str: &str) -> Option<&str> {
        for dict in &self.dicts {
            if let Some(output) = dict.lookup(stroke_str) {
                return Some(output);
            }
        }
        None
    }

    pub fn total_entries(&self) -> usize {
        self.dicts.iter().map(|d| d.len()).sum()
    }
}

impl Default for DictionaryStack {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    #[error("failed to read dictionary file {0}: {1}")]
    Io(std::path::PathBuf, std::io::Error),
    #[error("failed to parse dictionary JSON {0}: {1}")]
    Parse(std::path::PathBuf, serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_lookup() {
        let mut dict = Dictionary::new("test");
        dict.insert("SA", "さ");
        dict.insert("KA", "か");
        assert_eq!(dict.lookup("SA"), Some("さ"));
        assert_eq!(dict.lookup("XX"), None);
    }

    #[test]
    fn stack_priority() {
        let mut base = Dictionary::new("base");
        base.insert("SA", "さ_base");

        let mut user = Dictionary::new("user");
        user.insert("SA", "さ_user");

        let mut stack = DictionaryStack::new();
        stack.push_back(base);
        stack.push(user); // higher priority

        assert_eq!(stack.lookup("SA"), Some("さ_user"));
    }
}
