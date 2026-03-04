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
    history: Vec<TranslationEntry>,
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
            history: Vec::new(),
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
        if let Some(entry) = self.history.pop() {
            let char_count = entry.output.chars().count();
            TranslationResult::Undo(char_count)
        } else {
            TranslationResult::Nothing
        }
    }

    fn push_history(&mut self, stroke: Stroke, output: String) {
        self.history.push(TranslationEntry { stroke, output });
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    pub fn history(&self) -> &[TranslationEntry] {
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
}
