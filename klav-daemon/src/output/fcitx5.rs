use super::OutputBackend;

/// Fcitx5 output backend using D-Bus.
///
/// Commits text directly through the Fcitx5 input method framework via
/// `dbus-send`. This integrates with Fcitx5's input pipeline, allowing
/// text to be properly handled by applications using Fcitx5.
///
/// Falls back to xdotool for backspace since Fcitx5 doesn't provide
/// a direct backspace API.
pub struct Fcitx5Output {
    _marker: (),
}

impl Fcitx5Output {
    pub fn new() -> std::io::Result<Self> {
        // Verify Fcitx5 is running by checking its D-Bus presence
        let status = std::process::Command::new("dbus-send")
            .arg("--session")
            .arg("--print-reply")
            .arg("--dest=org.fcitx.Fcitx5")
            .arg("/controller")
            .arg("org.freedesktop.DBus.Peer.Ping")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => {
                log::info!("output backend: fcitx5 (D-Bus)");
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Fcitx5 is not running or not reachable via D-Bus",
                ));
            }
        }

        Ok(Self { _marker: () })
    }

    /// Check if Fcitx5 is available (for auto-detection).
    pub fn is_available() -> bool {
        std::process::Command::new("dbus-send")
            .arg("--session")
            .arg("--print-reply")
            .arg("--dest=org.fcitx.Fcitx5")
            .arg("/controller")
            .arg("org.freedesktop.DBus.Peer.Ping")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
}

impl OutputBackend for Fcitx5Output {
    fn type_text(&mut self, text: &str) -> std::io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        log::debug!("fcitx5 commit: {text}");

        // Use Fcitx5's CommitString via D-Bus to inject text
        let status = std::process::Command::new("dbus-send")
            .arg("--session")
            .arg("--type=method_call")
            .arg("--dest=org.fcitx.Fcitx5")
            .arg("/org/freedesktop/portal/inputmethod")
            .arg("org.fcitx.Fcitx.InputMethod1.CommitString")
            .arg(format!("string:{text}"))
            .status()?;

        if !status.success() {
            log::warn!("fcitx5 CommitString failed, falling back to xdotool");
            return self.xdotool_type(text);
        }

        Ok(())
    }

    fn backspace(&mut self, count: usize) -> std::io::Result<()> {
        if count == 0 {
            return Ok(());
        }

        log::debug!("backspace x{count}");

        // Fcitx5 doesn't have a direct backspace API, so forward key events
        // through Fcitx5's ForwardKey or fall back to xdotool
        let keys: Vec<&str> = (0..count).map(|_| "BackSpace").collect();

        let status = std::process::Command::new("xdotool")
            .arg("key")
            .arg("--delay")
            .arg("0")
            .args(&keys)
            .status()?;

        if !status.success() {
            return Err(std::io::Error::other(format!("backspace failed ({count}x): {status}"),
            ));
        }

        Ok(())
    }
}

impl Fcitx5Output {
    fn xdotool_type(&self, text: &str) -> std::io::Result<()> {
        let status = std::process::Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg("--delay")
            .arg("0")
            .arg(text)
            .status()?;

        if !status.success() {
            return Err(std::io::Error::other(format!("xdotool fallback exited with {status}"),
            ));
        }

        Ok(())
    }
}
