use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::stroke::{Stroke, StenoKey};

/// A steno theory translates strokes into text.
pub trait Theory {
    /// Translate a stroke into output text.
    /// Returns `None` if the stroke has no mapping in this theory.
    fn translate(&self, stroke: &Stroke) -> Option<String>;

    /// The name of this theory.
    fn name(&self) -> &str;
}

/// The Japanese syllable-based theory (Layer 1 + Layer 2).
///
/// Layer 1: Rule-based consonant × vowel → kana syllable
/// Layer 2: Dictionary lookup for words/phrases
pub struct JapaneseTheory {
    /// Layer 1: syllable rules (consonant key → { vowel key → kana })
    syllable_map: HashMap<Option<Consonant>, HashMap<Vowel, String>>,
    /// Voiced consonant mappings
    voiced_map: HashMap<Consonant, Consonant>,
    /// Half-voiced consonant mappings
    half_voiced_map: HashMap<Consonant, Consonant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Consonant {
    K, S, T, N, H, M, Y, R, W, G, Z, D, B, P,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Vowel {
    A, I, U, E, O,
}

/// Raw TOML format for syllable rules.
#[derive(Debug, Deserialize)]
struct RulesFile {
    syllable_rules: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    voiced_rules: HashMap<String, String>,
    #[serde(default)]
    half_voiced_rules: HashMap<String, String>,
}

impl JapaneseTheory {
    pub fn load(rules_path: &Path) -> Result<Self, TheoryError> {
        let content = std::fs::read_to_string(rules_path)
            .map_err(|e| TheoryError::Io(rules_path.to_path_buf(), e))?;
        Self::from_toml(&content)
    }

    pub fn from_toml(content: &str) -> Result<Self, TheoryError> {
        let file: RulesFile = toml::from_str(content)
            .map_err(TheoryError::Parse)?;

        let mut syllable_map: HashMap<Option<Consonant>, HashMap<Vowel, String>> = HashMap::new();

        for (consonant_str, vowel_map) in &file.syllable_rules {
            let consonant = if consonant_str.is_empty() {
                None
            } else {
                Some(parse_consonant(consonant_str)
                    .ok_or_else(|| TheoryError::InvalidConsonant(consonant_str.clone()))?)
            };

            let mut inner = HashMap::new();
            for (vowel_str, kana) in vowel_map {
                let vowel = parse_vowel(vowel_str)
                    .ok_or_else(|| TheoryError::InvalidVowel(vowel_str.clone()))?;
                inner.insert(vowel, kana.clone());
            }
            syllable_map.insert(consonant, inner);
        }

        let mut voiced_map = HashMap::new();
        for (from, to) in &file.voiced_rules {
            let from_c = parse_consonant(from)
                .ok_or_else(|| TheoryError::InvalidConsonant(from.clone()))?;
            let to_c = parse_consonant(to)
                .ok_or_else(|| TheoryError::InvalidConsonant(to.clone()))?;
            voiced_map.insert(from_c, to_c);
        }

        let mut half_voiced_map = HashMap::new();
        for (from, to) in &file.half_voiced_rules {
            let from_c = parse_consonant(from)
                .ok_or_else(|| TheoryError::InvalidConsonant(from.clone()))?;
            let to_c = parse_consonant(to)
                .ok_or_else(|| TheoryError::InvalidConsonant(to.clone()))?;
            half_voiced_map.insert(from_c, to_c);
        }

        Ok(Self {
            syllable_map,
            voiced_map,
            half_voiced_map,
        })
    }

    /// Extract consonant from stroke's left-hand keys.
    ///
    /// Mappings follow StenoWord conventions:
    /// - K1 → K, S1 → S, T1 → T
    /// - P1+H1 → N (combo), H1 → H, P1 → M
    /// - W1 → Y, R1 → R
    /// - S1+W1 → W (combo)
    fn extract_consonant(&self, stroke: &Stroke) -> Option<Consonant> {
        // Check combos first (more specific matches)
        if stroke.contains(StenoKey::P1) && stroke.contains(StenoKey::H1) {
            return Some(Consonant::N);
        }
        if stroke.contains(StenoKey::S1) && stroke.contains(StenoKey::W1) {
            return Some(Consonant::W);
        }
        // Single key consonants
        if stroke.contains(StenoKey::K1) { return Some(Consonant::K); }
        if stroke.contains(StenoKey::S1) { return Some(Consonant::S); }
        if stroke.contains(StenoKey::T1) { return Some(Consonant::T); }
        if stroke.contains(StenoKey::H1) { return Some(Consonant::H); }
        if stroke.contains(StenoKey::P1) { return Some(Consonant::M); }
        if stroke.contains(StenoKey::W1) { return Some(Consonant::Y); }
        if stroke.contains(StenoKey::R1) { return Some(Consonant::R); }
        None
    }

    /// Extract vowel from stroke's thumb keys.
    ///
    /// StenoWord vowel encoding:
    /// - A alone → A
    /// - O alone → O
    /// - E alone → E
    /// - U alone → U
    /// - A+E → I (combo)
    fn extract_vowel(&self, stroke: &Stroke) -> Option<Vowel> {
        let a = stroke.contains(StenoKey::A);
        let o = stroke.contains(StenoKey::O);
        let e = stroke.contains(StenoKey::E);
        let u = stroke.contains(StenoKey::U);

        match (a, o, e, u) {
            // Combo: I = A+E
            (true, false, true, false) => Some(Vowel::I),
            // Singles
            (true, false, false, false) => Some(Vowel::A),
            (false, true, false, false) => Some(Vowel::O),
            (false, false, true, false) => Some(Vowel::E),
            (false, false, false, true) => Some(Vowel::U),
            _ => None,
        }
    }
}

impl Theory for JapaneseTheory {
    fn translate(&self, stroke: &Stroke) -> Option<String> {
        let vowel = self.extract_vowel(stroke)?;
        let mut consonant = self.extract_consonant(stroke);

        // Apply voiced/half-voiced modifiers
        if let Some(c) = consonant {
            if stroke.contains(StenoKey::Voiced) {
                consonant = self.voiced_map.get(&c).copied().or(consonant);
            } else if stroke.contains(StenoKey::HalfVoiced) {
                consonant = self.half_voiced_map.get(&c).copied().or(consonant);
            }
        }

        let vowel_map = self.syllable_map.get(&consonant)?;
        vowel_map.get(&vowel).cloned()
    }

    fn name(&self) -> &str {
        "ja-stenoword"
    }
}

fn parse_consonant(s: &str) -> Option<Consonant> {
    match s {
        "K" => Some(Consonant::K),
        "S" => Some(Consonant::S),
        "T" => Some(Consonant::T),
        "N" => Some(Consonant::N),
        "H" => Some(Consonant::H),
        "M" => Some(Consonant::M),
        "Y" => Some(Consonant::Y),
        "R" => Some(Consonant::R),
        "W" => Some(Consonant::W),
        "G" => Some(Consonant::G),
        "Z" => Some(Consonant::Z),
        "D" => Some(Consonant::D),
        "B" => Some(Consonant::B),
        "P" => Some(Consonant::P),
        _ => None,
    }
}

fn parse_vowel(s: &str) -> Option<Vowel> {
    match s {
        "A" => Some(Vowel::A),
        "I" => Some(Vowel::I),
        "U" => Some(Vowel::U),
        "E" => Some(Vowel::E),
        "O" => Some(Vowel::O),
        _ => None,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TheoryError {
    #[error("failed to read theory file {0}: {1}")]
    Io(std::path::PathBuf, std::io::Error),
    #[error("failed to parse theory TOML: {0}")]
    Parse(toml::de::Error),
    #[error("invalid consonant: {0}")]
    InvalidConsonant(String),
    #[error("invalid vowel: {0}")]
    InvalidVowel(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theory() -> JapaneseTheory {
        let toml = r#"
[syllable_rules]
"" = { "A" = "あ", "I" = "い", "U" = "う", "E" = "え", "O" = "お" }
"K" = { "A" = "か", "I" = "き", "U" = "く", "E" = "け", "O" = "こ" }
"S" = { "A" = "さ", "I" = "し", "U" = "す", "E" = "せ", "O" = "そ" }
"T" = { "A" = "た", "I" = "ち", "U" = "つ", "E" = "て", "O" = "と" }
"N" = { "A" = "な", "I" = "に", "U" = "ぬ", "E" = "ね", "O" = "の" }
"G" = { "A" = "が", "I" = "ぎ", "U" = "ぐ", "E" = "げ", "O" = "ご" }
"Z" = { "A" = "ざ", "I" = "じ", "U" = "ず", "E" = "ぜ", "O" = "ぞ" }
"D" = { "A" = "だ", "I" = "ぢ", "U" = "づ", "E" = "で", "O" = "ど" }

[voiced_rules]
"K" = "G"
"S" = "Z"
"T" = "D"

[half_voiced_rules]
"#;
        JapaneseTheory::from_toml(toml).unwrap()
    }

    #[test]
    fn translate_vowel_only() {
        let theory = test_theory();
        let stroke = Stroke::from_keys([StenoKey::A]);
        assert_eq!(theory.translate(&stroke), Some("あ".into()));
    }

    #[test]
    fn translate_vowel_i_combo() {
        let theory = test_theory();
        // I = A+E
        let stroke = Stroke::from_keys([StenoKey::A, StenoKey::E]);
        assert_eq!(theory.translate(&stroke), Some("い".into()));
    }

    #[test]
    fn translate_consonant_vowel() {
        let theory = test_theory();
        // K + A → か
        let stroke = Stroke::from_keys([StenoKey::K1, StenoKey::A]);
        assert_eq!(theory.translate(&stroke), Some("か".into()));
    }

    #[test]
    fn translate_voiced() {
        let theory = test_theory();
        // Voiced + K + A → が (K→G via voiced_rules)
        let stroke = Stroke::from_keys([StenoKey::K1, StenoKey::A, StenoKey::Voiced]);
        // G+A should produce が if G row exists in syllable_rules
        // Our test theory has voiced_rules K→G, but also has G row in syllable_rules
        assert_eq!(theory.translate(&stroke), Some("が".into()));
    }

    #[test]
    fn translate_n_combo() {
        let theory = test_theory();
        // P1+H1 → N consonant, + A → な
        let stroke = Stroke::from_keys([StenoKey::P1, StenoKey::H1, StenoKey::A]);
        assert_eq!(theory.translate(&stroke), Some("な".into()));
    }

    #[test]
    fn translate_w_combo() {
        let theory = test_theory();
        // S1+W1 → W consonant, + A → わ
        let stroke = Stroke::from_keys([StenoKey::S1, StenoKey::W1, StenoKey::A]);
        // W row doesn't exist in test_theory, so this should be None
        // unless we add it
        assert_eq!(theory.translate(&stroke), None);
    }

    #[test]
    fn no_vowel_returns_none() {
        let theory = test_theory();
        // Consonant only, no vowel → None
        let stroke = Stroke::from_keys([StenoKey::K1]);
        assert_eq!(theory.translate(&stroke), None);
    }

    #[test]
    fn ambiguous_vowel_returns_none() {
        let theory = test_theory();
        // A+O pressed together → ambiguous, returns None
        let stroke = Stroke::from_keys([StenoKey::A, StenoKey::O]);
        assert_eq!(theory.translate(&stroke), None);
    }
}
