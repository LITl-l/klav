use std::collections::BTreeSet;
use std::fmt;
use std::time::{Duration, Instant};

/// A logical steno key, independent of physical keyboard layout.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum StenoKey {
    // Left-hand consonants
    S1,
    T1,
    K1,
    P1,
    W1,
    H1,
    R1,
    // Thumbs (vowels)
    A,
    O,
    E,
    U,
    // Right-hand consonants
    F1,
    P2,
    L1,
    T2,
    D1,
    R2,
    B1,
    G1,
    S2,
    Z1,
    // Modifiers
    Star,       // * — used for disambiguation
    Voiced,     // 濁音 modifier
    HalfVoiced, // 半濁音 modifier
    // Special
    Lang, // Language switch
    Undo, // Undo last stroke
}

impl fmt::Display for StenoKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::S1 => "S",
            Self::T1 => "T",
            Self::K1 => "K",
            Self::P1 => "P",
            Self::W1 => "W",
            Self::H1 => "H",
            Self::R1 => "R",
            Self::A => "A",
            Self::O => "O",
            Self::E => "E",
            Self::U => "U",
            Self::F1 => "-F",
            Self::P2 => "-P",
            Self::L1 => "-L",
            Self::T2 => "-T",
            Self::D1 => "-D",
            Self::R2 => "-R",
            Self::B1 => "-B",
            Self::G1 => "-G",
            Self::S2 => "-S",
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
        self.keys
            .iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
            .join("")
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Self::new()
    }
}

/// Plover steno order: #STKPWHR*AOEUFRPBLGTSDZ
/// This parser converts Plover-format stroke strings to Klav's internal Stroke representation.
impl Stroke {
    /// Parse a Plover-format stroke string (e.g. "THAT", "STKPW", "-S", "KAT").
    ///
    /// Plover notation rules:
    /// - Left consonants come before vowels: STKPWHR
    /// - Vowels: AOEU
    /// - Right consonants come after vowels: FRPBLGTSDZ
    /// - Star (*) can appear between left consonants and vowels
    /// - A hyphen (-) explicitly marks the start of right-hand keys when no vowels are present
    /// - Number bar (#) at the start enables number mode
    pub fn from_plover(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }

        let mut stroke = Stroke::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        // Number bar
        if i < chars.len() && chars[i] == '#' {
            // Number bar — for now, just skip it (no StenoKey for it yet)
            i += 1;
        }

        // Determine if we have vowels or star to know the left/right boundary
        let has_vowel_or_star = s.contains(['A', 'O', 'E', 'U', '*']);
        let has_hyphen = s.contains('-');

        // If no vowel/star and no hyphen, entire stroke is left-hand
        // If has hyphen, everything before '-' is left, after is right
        // If has vowel/star, before first vowel/* is left, after last vowel/* is right

        if !has_vowel_or_star && !has_hyphen {
            // All left-hand
            while i < chars.len() {
                match chars[i] {
                    'S' => stroke.add(StenoKey::S1),
                    'T' => stroke.add(StenoKey::T1),
                    'K' => stroke.add(StenoKey::K1),
                    'P' => stroke.add(StenoKey::P1),
                    'W' => stroke.add(StenoKey::W1),
                    'H' => stroke.add(StenoKey::H1),
                    'R' => stroke.add(StenoKey::R1),
                    _ => return None,
                }
                i += 1;
            }
        } else if !has_vowel_or_star && has_hyphen {
            // Left-hand before hyphen, right-hand after
            while i < chars.len() && chars[i] != '-' {
                match chars[i] {
                    'S' => stroke.add(StenoKey::S1),
                    'T' => stroke.add(StenoKey::T1),
                    'K' => stroke.add(StenoKey::K1),
                    'P' => stroke.add(StenoKey::P1),
                    'W' => stroke.add(StenoKey::W1),
                    'H' => stroke.add(StenoKey::H1),
                    'R' => stroke.add(StenoKey::R1),
                    _ => return None,
                }
                i += 1;
            }
            if i < chars.len() && chars[i] == '-' {
                i += 1; // skip hyphen
            }
            while i < chars.len() {
                match chars[i] {
                    'F' => stroke.add(StenoKey::F1),
                    'R' => stroke.add(StenoKey::R2),
                    'P' => stroke.add(StenoKey::P2),
                    'B' => stroke.add(StenoKey::B1),
                    'L' => stroke.add(StenoKey::L1),
                    'G' => stroke.add(StenoKey::G1),
                    'T' => stroke.add(StenoKey::T2),
                    'S' => stroke.add(StenoKey::S2),
                    'D' => stroke.add(StenoKey::D1),
                    'Z' => stroke.add(StenoKey::Z1),
                    _ => return None,
                }
                i += 1;
            }
        } else {
            // Has vowels or star — parse left, vowels/star, right
            // Left-hand consonants
            while i < chars.len() && !matches!(chars[i], 'A' | 'O' | 'E' | 'U' | '*' | '-') {
                match chars[i] {
                    'S' => stroke.add(StenoKey::S1),
                    'T' => stroke.add(StenoKey::T1),
                    'K' => stroke.add(StenoKey::K1),
                    'P' => stroke.add(StenoKey::P1),
                    'W' => stroke.add(StenoKey::W1),
                    'H' => stroke.add(StenoKey::H1),
                    'R' => stroke.add(StenoKey::R1),
                    _ => return None,
                }
                i += 1;
            }

            // Vowels and star
            while i < chars.len() && matches!(chars[i], 'A' | 'O' | 'E' | 'U' | '*') {
                match chars[i] {
                    'A' => stroke.add(StenoKey::A),
                    'O' => stroke.add(StenoKey::O),
                    'E' => stroke.add(StenoKey::E),
                    'U' => stroke.add(StenoKey::U),
                    '*' => stroke.add(StenoKey::Star),
                    _ => unreachable!(),
                }
                i += 1;
            }

            // Right-hand consonants
            while i < chars.len() {
                match chars[i] {
                    'F' => stroke.add(StenoKey::F1),
                    'R' => stroke.add(StenoKey::R2),
                    'P' => stroke.add(StenoKey::P2),
                    'B' => stroke.add(StenoKey::B1),
                    'L' => stroke.add(StenoKey::L1),
                    'G' => stroke.add(StenoKey::G1),
                    'T' => stroke.add(StenoKey::T2),
                    'S' => stroke.add(StenoKey::S2),
                    'D' => stroke.add(StenoKey::D1),
                    'Z' => stroke.add(StenoKey::Z1),
                    _ => return None,
                }
                i += 1;
            }
        }

        if stroke.is_empty() {
            None
        } else {
            Some(stroke)
        }
    }

    /// Convert a Plover-format stroke string to Klav canonical steno string.
    /// Multi-stroke strings (with "/") are handled by converting each stroke individually.
    pub fn plover_to_canonical(plover: &str) -> Option<String> {
        let parts: Vec<&str> = plover.split('/').collect();
        let mut canonical_parts = Vec::with_capacity(parts.len());
        for part in parts {
            let stroke = Stroke::from_plover(part)?;
            canonical_parts.push(stroke.to_steno_string());
        }
        Some(canonical_parts.join("/"))
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

    // === Plover stroke parsing tests ===

    #[test]
    fn plover_left_only() {
        let stroke = Stroke::from_plover("STKPWHR").unwrap();
        assert!(stroke.contains(StenoKey::S1));
        assert!(stroke.contains(StenoKey::T1));
        assert!(stroke.contains(StenoKey::K1));
        assert!(stroke.contains(StenoKey::P1));
        assert!(stroke.contains(StenoKey::W1));
        assert!(stroke.contains(StenoKey::H1));
        assert!(stroke.contains(StenoKey::R1));
    }

    #[test]
    fn plover_vowels_only() {
        let stroke = Stroke::from_plover("AOEU").unwrap();
        assert!(stroke.contains(StenoKey::A));
        assert!(stroke.contains(StenoKey::O));
        assert!(stroke.contains(StenoKey::E));
        assert!(stroke.contains(StenoKey::U));
    }

    #[test]
    fn plover_right_with_hyphen() {
        let stroke = Stroke::from_plover("-FRPBLGTSDZ").unwrap();
        assert!(stroke.contains(StenoKey::F1));
        assert!(stroke.contains(StenoKey::R2));
        assert!(stroke.contains(StenoKey::P2));
        assert!(stroke.contains(StenoKey::B1));
        assert!(stroke.contains(StenoKey::L1));
        assert!(stroke.contains(StenoKey::G1));
        assert!(stroke.contains(StenoKey::T2));
        assert!(stroke.contains(StenoKey::S2));
        assert!(stroke.contains(StenoKey::D1));
        assert!(stroke.contains(StenoKey::Z1));
    }

    #[test]
    fn plover_that() {
        // THAT = T(left) + H(left) + A(vowel) + T(right)
        let stroke = Stroke::from_plover("THAT").unwrap();
        assert!(stroke.contains(StenoKey::T1));
        assert!(stroke.contains(StenoKey::H1));
        assert!(stroke.contains(StenoKey::A));
        assert!(stroke.contains(StenoKey::T2));
    }

    #[test]
    fn plover_that_canonical() {
        // THAT in Plover → "THA-T" in Klav canonical
        let canonical = Stroke::plover_to_canonical("THAT").unwrap();
        assert_eq!(canonical, "THA-T");
    }

    #[test]
    fn plover_the() {
        let stroke = Stroke::from_plover("THE").unwrap();
        assert!(stroke.contains(StenoKey::T1));
        assert!(stroke.contains(StenoKey::H1));
        assert!(stroke.contains(StenoKey::E));
    }

    #[test]
    fn plover_star() {
        let stroke = Stroke::from_plover("KA*T").unwrap();
        assert!(stroke.contains(StenoKey::K1));
        assert!(stroke.contains(StenoKey::A));
        assert!(stroke.contains(StenoKey::Star));
        assert!(stroke.contains(StenoKey::T2));
    }

    #[test]
    fn plover_right_s_with_hyphen() {
        let stroke = Stroke::from_plover("-S").unwrap();
        assert!(stroke.contains(StenoKey::S2));
        assert!(!stroke.contains(StenoKey::S1));
    }

    #[test]
    fn plover_multi_stroke_canonical() {
        let canonical = Stroke::plover_to_canonical("KAT/ER").unwrap();
        // KAT → K1, A, T2 → "KA-T"
        // ER → E, R2 → "E-R"
        assert_eq!(canonical, "KA-T/E-R");
    }

    #[test]
    fn plover_empty_returns_none() {
        assert!(Stroke::from_plover("").is_none());
    }

    #[test]
    fn plover_invalid_char_returns_none() {
        assert!(Stroke::from_plover("XYZ").is_none());
    }

    #[test]
    fn plover_number_bar() {
        // #S should parse (number bar + S left)
        let stroke = Stroke::from_plover("#S").unwrap();
        assert!(stroke.contains(StenoKey::S1));
    }

    #[test]
    fn plover_skp_and() {
        // SKP = S(left) + K(left) + P(left)
        let stroke = Stroke::from_plover("SKP").unwrap();
        assert!(stroke.contains(StenoKey::S1));
        assert!(stroke.contains(StenoKey::K1));
        assert!(stroke.contains(StenoKey::P1));
    }
}
