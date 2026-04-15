use super::OutputBackend;

/// X11 output backend using xdotool.
///
/// Types Unicode text via `xdotool type` and sends backspace keys
/// via `xdotool key`. Works on X11; for Wayland use the wtype backend.
pub struct XdotoolOutput {
    _marker: (),
}

impl XdotoolOutput {
    pub fn new() -> std::io::Result<Self> {
        let status = std::process::Command::new("which")
            .arg("xdotool")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => {
                log::info!("output backend: xdotool (X11)");
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "xdotool not found; install xdotool or use a different backend",
                ));
            }
        }

        Ok(Self { _marker: () })
    }
}

impl OutputBackend for XdotoolOutput {
    fn type_text(&mut self, text: &str) -> std::io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        log::debug!("typing: {text}");

        let status = std::process::Command::new("xdotool")
            .arg("type")
            .arg("--clearmodifiers")
            .arg("--delay")
            .arg("0")
            .arg(text)
            .status()?;

        if !status.success() {
            return Err(std::io::Error::other(format!(
                "xdotool exited with {status}"
            )));
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
                "xdotool backspace failed ({count}x): {status}"
            )));
        }

        Ok(())
    }
}
