use std::collections::VecDeque;

use crate::dictionary::DictionaryStack;
use crate::stroke::Stroke;
use crate::theory::Theory;

/// Translates strokes into text output using a theory and dictionary stack.
///
/// Lookup order:
/// 1. Dictionary (Layer 2) — exact stroke match
/// 2. Theory rules (Layer 1) — algorithmic syllable conversion
///
/// Also maintains an undo buffer for stroke reversal.
pub struct Translator {
    theory: Box<dyn Theory>,
    dictionaries: DictionaryStack,
    /// History of (stroke, output_text) for undo support.
    history: VecDeque<TranslationEntry>,
    /// Maximum history size.
    max_history: usize,
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

        // Layer 2: Dictionary lookup (higher priority)
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
            } else {
                None
            }
        }
        fn name(&self) -> &str { "dummy" }
    }

    fn make_translator() -> Translator {
        let mut dict = Dictionary::new("test");
        dict.insert("KA", "か");
        let mut stack = DictionaryStack::new();
        stack.push_back(dict);
        Translator::new(Box::new(DummyTheory), stack)
    }

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
        assert!(matches!(t.translate(&stroke), TranslationResult::LangSwitch));
    }

    #[test]
    fn unmapped_stroke_is_nothing() {
        let mut t = make_translator();
        // Right-hand only stroke with no dictionary match
        let stroke = Stroke::from_keys([StenoKey::F1, StenoKey::P2]);
        assert!(matches!(t.translate(&stroke), TranslationResult::Nothing));
    }
}
