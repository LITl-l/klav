/// Abstraction over platform-specific text output.
pub trait OutputBackend {
    /// Type a string of text as keyboard input.
    fn type_text(&mut self, text: &str) -> std::io::Result<()>;

    /// Delete N characters (backspace).
    fn backspace(&mut self, count: usize) -> std::io::Result<()>;
}

#[cfg(target_os = "linux")]
pub mod uinput;

#[cfg(target_os = "windows")]
pub mod win_sendinput;
