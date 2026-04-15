use super::OutputBackend;

/// IBus output backend using D-Bus.
///
/// Commits text directly through the IBus input method framework.
/// Uses `busctl` to call IBus D-Bus methods, which provides proper
/// type serialization for IBus's variant-based API.
///
/// Falls back to xdotool for backspace since IBus doesn't provide
/// a direct backspace API for external clients.
pub struct IbusOutput {
    _marker: (),
}

impl IbusOutput {
    pub fn new() -> std::io::Result<Self> {
        // Verify IBus is running by checking D-Bus name
        let status = std::process::Command::new("busctl")
            .arg("--user")
            .arg("status")
            .arg("org.freedesktop.IBus")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => {
                log::info!("output backend: ibus (D-Bus)");
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "IBus is not running or not reachable via D-Bus",
                ));
            }
        }

        Ok(Self { _marker: () })
    }

    /// Check if IBus is available (for auto-detection).
    pub fn is_available() -> bool {
        std::process::Command::new("busctl")
            .arg("--user")
            .arg("status")
            .arg("org.freedesktop.IBus")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
}

impl OutputBackend for IbusOutput {
    fn type_text(&mut self, text: &str) -> std::io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        log::debug!("ibus commit: {text}");

        // Get the focused input context path from IBus
        let context_output = std::process::Command::new("busctl")
            .arg("--user")
            .arg("call")
            .arg("org.freedesktop.IBus")
            .arg("/org/freedesktop/IBus")
            .arg("org.freedesktop.IBus")
            .arg("CurrentInputContext")
            .output()?;

        if !context_output.status.success() {
            log::warn!("ibus: failed to get input context, falling back to xdotool");
            return self.xdotool_type(text);
        }

        let output_str = String::from_utf8_lossy(&context_output.stdout);
        // busctl output format: "s "/org/freedesktop/IBus/InputContext_N""
        let context_path = output_str.split('"').nth(1).unwrap_or("").trim();

        if context_path.is_empty() {
            log::warn!("ibus: no focused input context, falling back to xdotool");
            return self.xdotool_type(text);
        }

        // CommitText takes an IBusText variant: (sa{sv}sv)
        // IBusText struct: type_name="IBusText", attrs={}, text, variant(empty)
        // busctl call format: CommitText "v" "(sa{sv}sv)" "IBusText" 0 "text" "v" "i" 0
        let status = std::process::Command::new("busctl")
            .arg("--user")
            .arg("call")
            .arg("org.freedesktop.IBus")
            .arg(context_path)
            .arg("org.freedesktop.IBus.InputContext")
            .arg("CommitText")
            .arg("v")
            .arg("(sa{sv}sv)")
            .arg("IBusText")
            .arg("0")
            .arg(text)
            .arg("v")
            .arg("i")
            .arg("0")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;

        if !status.success() {
            log::warn!("ibus CommitText failed, falling back to xdotool");
            return self.xdotool_type(text);
        }

        Ok(())
    }

    fn backspace(&mut self, count: usize) -> std::io::Result<()> {
        if count == 0 {
            return Ok(());
        }

        log::debug!("backspace x{count}");

        let keys: Vec<&str> = (0..count).map(|_| "BackSpace").collect();

        let status = std::process::Command::new("xdotool")
            .arg("key")
            .arg("--delay")
            .arg("0")
            .args(&keys)
            .status()?;

        if !status.success() {
            return Err(std::io::Error::other(format!(
                "backspace failed ({count}x): {status}"
            )));
        }

        Ok(())
    }
}

impl IbusOutput {
    fn xdotool_type(&self, text: &str) -> std::io::Result<()> {
        let status = std::process::Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg("--delay")
            .arg("0")
            .arg(text)
            .status()?;

        if !status.success() {
            return Err(std::io::Error::other(format!(
                "xdotool fallback exited with {status}"
            )));
        }

        Ok(())
    }
}
