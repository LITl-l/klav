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
///
/// Phase 1 additions: yōon (拗音), sokuon (促音), chōon (長音), syllabic ん
pub struct JapaneseTheory {
    /// Layer 1: syllable rules (consonant key → { vowel key → kana })
    syllable_map: HashMap<Option<Consonant>, HashMap<Vowel, String>>,
    /// Voiced consonant mappings
    voiced_map: HashMap<Consonant, Consonant>,
    /// Half-voiced consonant mappings
    half_voiced_map: HashMap<Consonant, Consonant>,
    /// Yōon rules (consonant → { vowel → contracted kana })
    yoon_map: HashMap<Consonant, HashMap<Vowel, String>>,
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
    #[serde(default)]
    yoon_rules: HashMap<String, HashMap<String, String>>,
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

        let mut yoon_map: HashMap<Consonant, HashMap<Vowel, String>> = HashMap::new();
        for (consonant_str, vowel_map) in &file.yoon_rules {
            let consonant = parse_consonant(consonant_str)
                .ok_or_else(|| TheoryError::InvalidConsonant(consonant_str.clone()))?;
            let mut inner = HashMap::new();
            for (vowel_str, kana) in vowel_map {
                let vowel = parse_vowel(vowel_str)
                    .ok_or_else(|| TheoryError::InvalidVowel(vowel_str.clone()))?;
                inner.insert(vowel, kana.clone());
            }
            yoon_map.insert(consonant, inner);
        }

        Ok(Self {
            syllable_map,
            voiced_map,
            half_voiced_map,
            yoon_map,
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

    /// Apply voiced/half-voiced modifier to a consonant.
    fn apply_voicing(&self, consonant: Consonant, stroke: &Stroke) -> Consonant {
        if stroke.contains(StenoKey::Voiced) {
            self.voiced_map.get(&consonant).copied().unwrap_or(consonant)
        } else if stroke.contains(StenoKey::HalfVoiced) {
            self.half_voiced_map.get(&consonant).copied().unwrap_or(consonant)
        } else {
            consonant
        }
    }
}

impl Theory for JapaneseTheory {
    fn translate(&self, stroke: &Stroke) -> Option<String> {
        let vowel = self.extract_vowel(stroke);
        let consonant = self.extract_consonant(stroke);

        let has_sokuon = stroke.contains(StenoKey::F1);
        let has_choon = stroke.contains(StenoKey::S2);
        let has_yoon = stroke.contains(StenoKey::Star);

        // Syllabic ん: consonant N without vowel
        if consonant == Some(Consonant::N) && vowel.is_none() && !has_sokuon && !has_choon {
            return Some("ん".into());
        }

        // Standalone sokuon: っ with no consonant and no vowel
        if has_sokuon && consonant.is_none() && vowel.is_none() {
            return Some("っ".into());
        }

        // Standalone chōon: ー with no consonant and no vowel
        if has_choon && consonant.is_none() && vowel.is_none() {
            return Some("ー".into());
        }

        // Need a vowel for syllable output
        let vowel = vowel?;

        // Apply voicing to consonant
        let consonant = consonant.map(|c| self.apply_voicing(c, stroke));

        // Build the kana syllable
        let kana = if has_yoon {
            // Yōon: look up in yoon_map
            let c = consonant?; // yōon requires a consonant
            let vowel_map = self.yoon_map.get(&c)?;
            vowel_map.get(&vowel)?.clone()
        } else {
            // Normal syllable lookup
            let vowel_map = self.syllable_map.get(&consonant)?;
            vowel_map.get(&vowel)?.clone()
        };

        // Apply sokuon (prepend っ) and chōon (append ー) modifiers
        let mut result = String::new();
        if has_sokuon {
            result.push('っ');
        }
        result.push_str(&kana);
        if has_choon {
            result.push('ー');
        }

        Some(result)
    }

    fn name(&self) -> &str {
        "ja-stenoword"
    }
}

/// English steno theory (Plover-compatible).
///
/// English steno is almost entirely dictionary-driven. This theory provides no
/// algorithmic fallback — all translations come from the dictionary layer.
/// The main value is in identifying the theory for the daemon's language switching.
pub struct EnglishTheory;

impl Theory for EnglishTheory {
    fn translate(&self, _stroke: &Stroke) -> Option<String> {
        // English has no algorithmic syllable→text conversion.
        // All translations are handled by the dictionary layer.
        None
    }

    fn name(&self) -> &str {
        "en-plover"
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
"H" = { "A" = "は", "I" = "ひ", "U" = "ふ", "E" = "へ", "O" = "ほ" }
"B" = { "A" = "ば", "I" = "び", "U" = "ぶ", "E" = "べ", "O" = "ぼ" }
"P" = { "A" = "ぱ", "I" = "ぴ", "U" = "ぷ", "E" = "ぺ", "O" = "ぽ" }

[voiced_rules]
"K" = "G"
"S" = "Z"
"T" = "D"
"H" = "B"

[half_voiced_rules]
"H" = "P"

[yoon_rules]
"K" = { "A" = "きゃ", "U" = "きゅ", "O" = "きょ" }
"S" = { "A" = "しゃ", "U" = "しゅ", "O" = "しょ" }
"T" = { "A" = "ちゃ", "U" = "ちゅ", "O" = "ちょ" }
"N" = { "A" = "にゃ", "U" = "にゅ", "O" = "にょ" }
"H" = { "A" = "ひゃ", "U" = "ひゅ", "O" = "ひょ" }
"G" = { "A" = "ぎゃ", "U" = "ぎゅ", "O" = "ぎょ" }
"Z" = { "A" = "じゃ", "U" = "じゅ", "O" = "じょ" }
"B" = { "A" = "びゃ", "U" = "びゅ", "O" = "びょ" }
"P" = { "A" = "ぴゃ", "U" = "ぴゅ", "O" = "ぴょ" }
"#;
        JapaneseTheory::from_toml(toml).unwrap()
    }

    // === Phase 0 tests (preserved) ===

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
        assert_eq!(theory.translate(&stroke), None);
    }

    #[test]
    fn no_vowel_returns_none() {
        let theory = test_theory();
        // Consonant only, no vowel → None (except N → ん)
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

    // === Phase 1 tests: yōon (拗音) ===

    #[test]
    fn yoon_kya() {
        let theory = test_theory();
        // K + Star + A → きゃ
        let stroke = Stroke::from_keys([StenoKey::K1, StenoKey::Star, StenoKey::A]);
        assert_eq!(theory.translate(&stroke), Some("きゃ".into()));
    }

    #[test]
    fn yoon_shu() {
        let theory = test_theory();
        // S + Star + U → しゅ
        let stroke = Stroke::from_keys([StenoKey::S1, StenoKey::Star, StenoKey::U]);
        assert_eq!(theory.translate(&stroke), Some("しゅ".into()));
    }

    #[test]
    fn yoon_cho() {
        let theory = test_theory();
        // T + Star + O → ちょ
        let stroke = Stroke::from_keys([StenoKey::T1, StenoKey::Star, StenoKey::O]);
        assert_eq!(theory.translate(&stroke), Some("ちょ".into()));
    }

    #[test]
    fn yoon_voiced_gya() {
        let theory = test_theory();
        // Voiced + K + Star + A → ぎゃ (K→G via voicing, then yōon)
        let stroke = Stroke::from_keys([StenoKey::K1, StenoKey::Voiced, StenoKey::Star, StenoKey::A]);
        assert_eq!(theory.translate(&stroke), Some("ぎゃ".into()));
    }

    #[test]
    fn yoon_half_voiced_pyo() {
        let theory = test_theory();
        // HalfVoiced + H + Star + O → ぴょ (H→P via half-voicing, then yōon)
        let stroke = Stroke::from_keys([StenoKey::H1, StenoKey::HalfVoiced, StenoKey::Star, StenoKey::O]);
        assert_eq!(theory.translate(&stroke), Some("ぴょ".into()));
    }

    #[test]
    fn yoon_without_consonant_returns_none() {
        let theory = test_theory();
        // Star + A without consonant → None (yōon requires consonant)
        let stroke = Stroke::from_keys([StenoKey::Star, StenoKey::A]);
        assert_eq!(theory.translate(&stroke), None);
    }

    // === Phase 1 tests: syllabic ん ===

    #[test]
    fn syllabic_n() {
        let theory = test_theory();
        // P1+H1 (N consonant) without vowel → ん
        let stroke = Stroke::from_keys([StenoKey::P1, StenoKey::H1]);
        assert_eq!(theory.translate(&stroke), Some("ん".into()));
    }

    // === Phase 1 tests: sokuon (促音) ===

    #[test]
    fn sokuon_standalone() {
        let theory = test_theory();
        // F1 alone → っ
        let stroke = Stroke::from_keys([StenoKey::F1]);
        assert_eq!(theory.translate(&stroke), Some("っ".into()));
    }

    #[test]
    fn sokuon_with_syllable() {
        let theory = test_theory();
        // F1 + K + A → っか
        let stroke = Stroke::from_keys([StenoKey::F1, StenoKey::K1, StenoKey::A]);
        assert_eq!(theory.translate(&stroke), Some("っか".into()));
    }

    #[test]
    fn sokuon_with_yoon() {
        let theory = test_theory();
        // F1 + T + Star + O → っちょ
        let stroke = Stroke::from_keys([StenoKey::F1, StenoKey::T1, StenoKey::Star, StenoKey::O]);
        assert_eq!(theory.translate(&stroke), Some("っちょ".into()));
    }

    // === Phase 1 tests: chōon (長音) ===

    #[test]
    fn choon_standalone() {
        let theory = test_theory();
        // S2 alone → ー
        let stroke = Stroke::from_keys([StenoKey::S2]);
        assert_eq!(theory.translate(&stroke), Some("ー".into()));
    }

    #[test]
    fn choon_with_syllable() {
        let theory = test_theory();
        // K + A + S2 → かー
        let stroke = Stroke::from_keys([StenoKey::K1, StenoKey::A, StenoKey::S2]);
        assert_eq!(theory.translate(&stroke), Some("かー".into()));
    }

    #[test]
    fn sokuon_and_choon_combined() {
        let theory = test_theory();
        // F1 + K + A + S2 → っかー
        let stroke = Stroke::from_keys([StenoKey::F1, StenoKey::K1, StenoKey::A, StenoKey::S2]);
        assert_eq!(theory.translate(&stroke), Some("っかー".into()));
    }

    // === EnglishTheory tests ===

    #[test]
    fn english_theory_returns_none() {
        let theory = EnglishTheory;
        // EnglishTheory is dictionary-driven, translate() always returns None
        let stroke = Stroke::from_keys([StenoKey::T1, StenoKey::H1, StenoKey::E]);
        assert_eq!(theory.translate(&stroke), None);
    }

    #[test]
    fn english_theory_name() {
        let theory = EnglishTheory;
        assert_eq!(theory.name(), "en-plover");
    }
}
