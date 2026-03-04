use super::OutputBackend;

/// Linux uinput virtual keyboard output backend.
///
/// Creates a virtual keyboard device via /dev/uinput and emits key events
/// to type text. For characters outside basic ASCII/Latin, we use
/// the XDG approach of writing to a temporary file and using xdotool,
/// or alternatively, use IBus/Fcitx commit string.
///
/// Phase 0 implementation: uses xdotool for simplicity.
/// Future: direct uinput key event sequences or IBus integration.
pub struct UinputOutput {
    _marker: (),
}

impl UinputOutput {
    pub fn new() -> std::io::Result<Self> {
        // Verify xdotool is available for Phase 0
        let status = std::process::Command::new("which")
            .arg("xdotool")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => {
                log::info!("uinput output initialized (using xdotool backend)");
            }
            _ => {
                log::warn!("xdotool not found; output may not work. Install xdotool or use ydotool for Wayland.");
            }
        }

        Ok(Self { _marker: () })
    }
}

impl OutputBackend for UinputOutput {
    fn type_text(&mut self, text: &str) -> std::io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        log::debug!("typing: {text}");

        // Phase 0: use xdotool for Unicode support
        let status = std::process::Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg("--delay")
            .arg("0")
            .arg(text)
            .status()?;

        if !status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("xdotool exited with {status}"),
            ));
        }

        Ok(())
    }

    fn backspace(&mut self, count: usize) -> std::io::Result<()> {
        if count == 0 {
            return Ok(());
        }

        log::debug!("backspace ×{count}");

        for _ in 0..count {
            let status = std::process::Command::new("xdotool")
                .arg("key")
                .arg("BackSpace")
                .status()?;

            if !status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "xdotool backspace failed",
                ));
            }
        }

        Ok(())
    }
}
