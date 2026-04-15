use std::collections::VecDeque;

use crate::dictionary::DictionaryStack;
use crate::stroke::Stroke;
use crate::theory::Theory;

/// Translates strokes into text output using a theory and dictionary stack.
///
/// Lookup order:
/// 1. Multi-stroke dictionary lookup (current stroke + buffered history)
/// 2. Single-stroke dictionary lookup (Layer 2)
/// 3. Theory rules (Layer 1) — algorithmic syllable conversion
///
/// Also maintains an undo buffer for stroke reversal.
pub struct Translator {
    theory: Box<dyn Theory>,
    dictionaries: DictionaryStack,
    /// History of (stroke, output_text) for undo support.
    history: VecDeque<TranslationEntry>,
    /// Maximum history size.
    max_history: usize,
    /// Maximum number of strokes to buffer for multi-stroke lookup.
    max_multi_stroke: usize,
}

#[derive(Debug, Clone)]
pub struct TranslationEntry {
    pub stroke: Stroke,
    pub output: String,
}

/// The result of processing a stroke.
#[derive(Debug)]
pub enum TranslationResult {
    /// Emit this text.
    Output(String),
    /// Undo the last N characters, then emit new text.
    /// Used for multi-stroke replacements: delete previous partial output, emit full match.
    Replace { backspace: usize, text: String },
    /// Undo the last N characters.
    Undo(usize),
    /// Switch language (the daemon handles this).
    LangSwitch,
    /// No translation found for this stroke.
    Nothing,
}

impl Translator {
    pub fn new(theory: Box<dyn Theory>, dictionaries: DictionaryStack) -> Self {
        Self {
            theory,
            dictionaries,
            history: VecDeque::new(),
            max_history: 1000,
            max_multi_stroke: 6,
        }
    }

    /// Process a stroke and return the translation result.
    pub fn translate(&mut self, stroke: &Stroke) -> TranslationResult {
        // Special strokes
        if stroke.is_undo() {
            return self.undo();
        }
        if stroke.is_lang_switch() {
            return TranslationResult::LangSwitch;
        }

        // Try multi-stroke dictionary lookup (longest match first)
        if let Some(result) = self.try_multi_stroke(stroke) {
            return result;
        }

        // Layer 2: Single-stroke dictionary lookup
        let steno_str = stroke.to_steno_string();
        if let Some(output) = self.dictionaries.lookup(&steno_str) {
            let output = output.to_string();
            self.push_history(stroke.clone(), output.clone());
            return TranslationResult::Output(output);
        }

        // Layer 1: Theory rules
        if let Some(output) = self.theory.translate(stroke) {
            self.push_history(stroke.clone(), output.clone());
            return TranslationResult::Output(output);
        }

        TranslationResult::Nothing
    }

    /// Try to match the current stroke combined with recent history as a multi-stroke
    /// dictionary entry. Returns the longest match found.
    ///
    /// Multi-stroke dictionary keys use "/" as separator, e.g. "KA/TA" for a two-stroke word.
    fn try_multi_stroke(&mut self, stroke: &Stroke) -> Option<TranslationResult> {
        let current_steno = stroke.to_steno_string();
        let history_len = self.history.len();
        let max_lookback = self.max_multi_stroke.min(history_len);

        // Try longest sequences first (most specific match)
        for lookback in (1..=max_lookback).rev() {
            let start = history_len - lookback;
            let mut multi_key = String::new();
            for i in start..history_len {
                multi_key.push_str(&self.history[i].stroke.to_steno_string());
                multi_key.push('/');
            }
            multi_key.push_str(&current_steno);

            if let Some(output) = self.dictionaries.lookup(&multi_key) {
                let output = output.to_string();

                // Calculate how many characters to backspace (undo previous partial outputs)
                let backspace: usize = (start..history_len)
                    .map(|i| self.history[i].output.chars().count())
                    .sum();

                // Remove the history entries we're replacing
                for _ in 0..lookback {
                    self.history.pop_back();
                }

                // Push the combined entry
                self.push_history(stroke.clone(), output.clone());

                return Some(TranslationResult::Replace {
                    backspace,
                    text: output,
                });
            }
        }

        None
    }

    fn undo(&mut self) -> TranslationResult {
        if let Some(entry) = self.history.pop_back() {
            let char_count = entry.output.chars().count();
            TranslationResult::Undo(char_count)
        } else {
            TranslationResult::Nothing
        }
    }

    fn push_history(&mut self, stroke: Stroke, output: String) {
        self.history.push_back(TranslationEntry { stroke, output });
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    pub fn history(&self) -> &VecDeque<TranslationEntry> {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Replace the active theory (for language switching).
    pub fn set_theory(&mut self, theory: Box<dyn Theory>) {
        self.theory = theory;
        self.clear_history();
    }

    /// Replace the dictionary stack (for language switching).
    pub fn set_dictionaries(&mut self, dictionaries: DictionaryStack) {
        self.dictionaries = dictionaries;
    }

    /// The name of the active theory.
    pub fn theory_name(&self) -> &str {
        self.theory.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::Dictionary;
    use crate::stroke::StenoKey;

    struct DummyTheory;
    impl Theory for DummyTheory {
        fn translate(&self, stroke: &Stroke) -> Option<String> {
            if stroke.contains(StenoKey::A) && stroke.keys().len() == 1 {
                Some("あ".into())
            } else if stroke.contains(StenoKey::K1)
                && stroke.contains(StenoKey::A)
                && stroke.keys().len() == 2
            {
                Some("か".into())
            } else if stroke.contains(StenoKey::T1)
                && stroke.contains(StenoKey::A)
                && stroke.keys().len() == 2
            {
                Some("た".into())
            } else {
                None
            }
        }
        fn name(&self) -> &str {
            "dummy"
        }
    }

    fn make_translator() -> Translator {
        let mut dict = Dictionary::new("test");
        dict.insert("KA", "か");
        let mut stack = DictionaryStack::new();
        stack.push_back(dict);
        Translator::new(Box::new(DummyTheory), stack)
    }

    fn make_multi_stroke_translator() -> Translator {
        let mut dict = Dictionary::new("test");
        dict.insert("KA", "か");
        dict.insert("TA", "た");
        // Multi-stroke entries
        dict.insert("KA/TA", "肩"); // かた → 肩 (shoulder)
        dict.insert("KA/KA/TA", "味方"); // かかた → 味方 (ally) -- contrived example
        let mut stack = DictionaryStack::new();
        stack.push_back(dict);
        Translator::new(Box::new(DummyTheory), stack)
    }

    // === Phase 0 tests (preserved) ===

    #[test]
    fn dict_lookup_has_priority() {
        let mut t = make_translator();
        let stroke = Stroke::from_keys([StenoKey::K1, StenoKey::A]);
        match t.translate(&stroke) {
            TranslationResult::Output(text) => assert_eq!(text, "か"),
            other => panic!("expected Output, got {other:?}"),
        }
    }

    #[test]
    fn theory_fallback() {
        let mut t = make_translator();
        let stroke = Stroke::from_keys([StenoKey::A]);
        match t.translate(&stroke) {
            TranslationResult::Output(text) => assert_eq!(text, "あ"),
            other => panic!("expected Output, got {other:?}"),
        }
    }

    #[test]
    fn undo_reverses_last() {
        let mut t = make_translator();
        let stroke = Stroke::from_keys([StenoKey::A]);
        t.translate(&stroke);
        assert_eq!(t.history().len(), 1);

        let undo = Stroke::from_keys([StenoKey::Undo]);
        match t.translate(&undo) {
            TranslationResult::Undo(count) => assert_eq!(count, 1), // "あ" is 1 char
            other => panic!("expected Undo, got {other:?}"),
        }
        assert!(t.history().is_empty());
    }

    #[test]
    fn undo_on_empty_is_nothing() {
        let mut t = make_translator();
        let undo = Stroke::from_keys([StenoKey::Undo]);
        assert!(matches!(t.translate(&undo), TranslationResult::Nothing));
    }

    #[test]
    fn lang_switch() {
        let mut t = make_translator();
        let stroke = Stroke::from_keys([StenoKey::Lang]);
        assert!(matches!(
            t.translate(&stroke),
            TranslationResult::LangSwitch
        ));
    }

    #[test]
    fn unmapped_stroke_is_nothing() {
        let mut t = make_translator();
        // Right-hand only stroke with no dictionary match
        let stroke = Stroke::from_keys([StenoKey::F1, StenoKey::P2]);
        assert!(matches!(t.translate(&stroke), TranslationResult::Nothing));
    }

    // === Phase 1 tests: multi-stroke ===

    #[test]
    fn multi_stroke_two_stroke_match() {
        let mut t = make_multi_stroke_translator();

        // First stroke: KA → か (single match)
        let ka = Stroke::from_keys([StenoKey::K1, StenoKey::A]);
        match t.translate(&ka) {
            TranslationResult::Output(text) => assert_eq!(text, "か"),
            other => panic!("expected Output, got {other:?}"),
        }

        // Second stroke: TA → should match "KA/TA" → 肩, replacing "か"
        let ta = Stroke::from_keys([StenoKey::T1, StenoKey::A]);
        match t.translate(&ta) {
            TranslationResult::Replace { backspace, text } => {
                assert_eq!(backspace, 1); // undo "か" (1 char)
                assert_eq!(text, "肩");
            }
            other => panic!("expected Replace, got {other:?}"),
        }
    }

    #[test]
    fn multi_stroke_three_stroke_match() {
        let mut t = make_multi_stroke_translator();

        let ka = Stroke::from_keys([StenoKey::K1, StenoKey::A]);
        t.translate(&ka); // か
        t.translate(&ka); // か

        // Third stroke: TA → should match "KA/KA/TA" → 味方
        let ta = Stroke::from_keys([StenoKey::T1, StenoKey::A]);
        match t.translate(&ta) {
            TranslationResult::Replace { backspace, text } => {
                assert_eq!(backspace, 2); // undo "か" + "か" (2 chars)
                assert_eq!(text, "味方");
            }
            other => panic!("expected Replace, got {other:?}"),
        }
    }

    #[test]
    fn multi_stroke_no_match_falls_through() {
        let mut t = make_multi_stroke_translator();

        // Stroke A → あ (theory fallback, no multi-stroke possible)
        let a = Stroke::from_keys([StenoKey::A]);
        t.translate(&a);

        // Stroke KA → か (single dict match, no "A/KA" in dict)
        let ka = Stroke::from_keys([StenoKey::K1, StenoKey::A]);
        match t.translate(&ka) {
            TranslationResult::Output(text) => assert_eq!(text, "か"),
            other => panic!("expected Output, got {other:?}"),
        }
    }
}
