use std::collections::BTreeSet;
use std::fmt;
use std::time::{Duration, Instant};

/// A logical steno key, independent of physical keyboard layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum StenoKey {
    // Left-hand consonants
    S1, T1, K1, P1, W1, H1, R1,
    // Thumbs (vowels)
    A, O, E, U,
    // Right-hand consonants
    F1, P2, L1, T2, D1, R2, B1, G1, S2, Z1,
    // Modifiers
    Star,   // * — used for disambiguation
    Voiced, // 濁音 modifier
    HalfVoiced, // 半濁音 modifier
    // Special
    Lang, // Language switch
    Undo, // Undo last stroke
}

impl fmt::Display for StenoKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::S1 => "S",  Self::T1 => "T",  Self::K1 => "K",
            Self::P1 => "P",  Self::W1 => "W",  Self::H1 => "H",
            Self::R1 => "R",
            Self::A => "A",   Self::O => "O",
            Self::E => "E",   Self::U => "U",
            Self::F1 => "-F", Self::P2 => "-P", Self::L1 => "-L",
            Self::T2 => "-T", Self::D1 => "-D", Self::R2 => "-R",
            Self::B1 => "-B", Self::G1 => "-G", Self::S2 => "-S",
            Self::Z1 => "-Z",
            Self::Star => "*",
            Self::Voiced => "#V",
            Self::HalfVoiced => "#H",
            Self::Lang => "LANG",
            Self::Undo => "UNDO",
        };
        write!(f, "{s}")
    }
}

/// A single steno stroke — a set of keys pressed simultaneously.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stroke {
    keys: BTreeSet<StenoKey>,
}

impl Stroke {
    pub fn new() -> Self {
        Self {
            keys: BTreeSet::new(),
        }
    }

    pub fn from_keys(keys: impl IntoIterator<Item = StenoKey>) -> Self {
        Self {
            keys: keys.into_iter().collect(),
        }
    }

    pub fn add(&mut self, key: StenoKey) {
        self.keys.insert(key);
    }

    pub fn contains(&self, key: StenoKey) -> bool {
        self.keys.contains(&key)
    }

    pub fn keys(&self) -> &BTreeSet<StenoKey> {
        &self.keys
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn is_lang_switch(&self) -> bool {
        self.keys.contains(&StenoKey::Lang)
    }

    pub fn is_undo(&self) -> bool {
        self.keys.contains(&StenoKey::Undo)
    }

    /// Produce a canonical string representation (steno order).
    pub fn to_steno_string(&self) -> String {
        self.keys.iter().map(|k| k.to_string()).collect::<Vec<_>>().join("")
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Stroke {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_steno_string())
    }
}

/// Detects complete strokes from a stream of key press/release events.
///
/// A stroke is defined as: one or more keys are pressed, then all pressed keys
/// are released. The set of keys that were held simultaneously forms the stroke.
pub struct StrokeDetector {
    /// Keys currently held down.
    held: BTreeSet<StenoKey>,
    /// All keys that were part of the current chord (including already-released).
    chord: BTreeSet<StenoKey>,
    /// When the first key of the current chord was pressed.
    chord_start: Option<Instant>,
    /// Maximum time from first press to stroke finalization.
    timeout: Duration,
}

impl StrokeDetector {
    pub fn new(timeout: Duration) -> Self {
        Self {
            held: BTreeSet::new(),
            chord: BTreeSet::new(),
            chord_start: None,
            timeout,
        }
    }

    /// Register a key press. Returns `None` — strokes are only emitted on release.
    pub fn key_down(&mut self, key: StenoKey) {
        if self.chord_start.is_none() {
            self.chord_start = Some(Instant::now());
        }
        self.held.insert(key);
        self.chord.insert(key);
    }

    /// Register a key release. Returns `Some(Stroke)` when all keys have been released.
    pub fn key_up(&mut self, key: StenoKey) -> Option<Stroke> {
        self.held.remove(&key);

        if self.held.is_empty() && !self.chord.is_empty() {
            let stroke = Stroke::from_keys(std::mem::take(&mut self.chord));
            self.chord_start = None;
            Some(stroke)
        } else {
            None
        }
    }

    /// Check for timeout. Returns `Some(Stroke)` if the chord has timed out.
    pub fn check_timeout(&mut self) -> Option<Stroke> {
        if let Some(start) = self.chord_start
            && start.elapsed() >= self.timeout
            && !self.chord.is_empty()
        {
            let stroke = Stroke::from_keys(std::mem::take(&mut self.chord));
            self.held.clear();
            self.chord_start = None;
            return Some(stroke);
        }
        None
    }

    pub fn reset(&mut self) {
        self.held.clear();
        self.chord.clear();
        self.chord_start = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_key_stroke() {
        let mut det = StrokeDetector::new(Duration::from_secs(5));
        det.key_down(StenoKey::A);
        assert!(det.key_up(StenoKey::A).is_some());
    }

    #[test]
    fn chord_stroke() {
        let mut det = StrokeDetector::new(Duration::from_secs(5));
        det.key_down(StenoKey::S1);
        det.key_down(StenoKey::A);
        assert!(det.key_up(StenoKey::S1).is_none());
        let stroke = det.key_up(StenoKey::A).unwrap();
        assert!(stroke.contains(StenoKey::S1));
        assert!(stroke.contains(StenoKey::A));
    }

    #[test]
    fn stroke_display() {
        let stroke = Stroke::from_keys([StenoKey::S1, StenoKey::A, StenoKey::T2]);
        let s = stroke.to_steno_string();
        assert!(s.contains('S'));
        assert!(s.contains('A'));
        assert!(s.contains("-T"));
    }
}
