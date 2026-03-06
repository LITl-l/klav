/// Key event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    Press,
    Release,
}

/// A raw key event from the input backend.
#[derive(Debug, Clone)]
pub struct RawKeyEvent {
    /// The evdev key code (or platform-equivalent).
    pub code: u16,
    pub kind: KeyEventKind,
}

/// Abstraction over platform-specific keyboard input.
pub trait InputBackend {
    /// Block until the next key event is available.
    fn next_event(&mut self) -> std::io::Result<RawKeyEvent>;

    /// Grab the keyboard device (prevent events from reaching other applications).
    fn grab(&mut self) -> std::io::Result<()>;

    /// Release the keyboard device.
    fn ungrab(&mut self) -> std::io::Result<()>;
}

#[cfg(target_os = "linux")]
pub mod evdev;

#[cfg(target_os = "windows")]
pub mod win_hook;
