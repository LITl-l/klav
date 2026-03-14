use super::OutputBackend;

/// Wayland output backend using wtype.
///
/// Types Unicode text via `wtype` and sends backspace keys.
/// Requires wtype to be installed and a Wayland compositor running.
pub struct WtypeOutput {
    _marker: (),
}

impl WtypeOutput {
    pub fn new() -> std::io::Result<Self> {
        let status = std::process::Command::new("which")
            .arg("wtype")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => {
                log::info!("output backend: wtype (Wayland)");
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "wtype not found; install wtype or use a different backend",
                ));
            }
        }

        Ok(Self { _marker: () })
    }
}

impl OutputBackend for WtypeOutput {
    fn type_text(&mut self, text: &str) -> std::io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        log::debug!("typing: {text}");

        let status = std::process::Command::new("wtype")
            .arg("--")
            .arg(text)
            .status()?;

        if !status.success() {
            return Err(std::io::Error::other(format!("wtype exited with {status}"),
            ));
        }

        Ok(())
    }

    fn backspace(&mut self, count: usize) -> std::io::Result<()> {
        if count == 0 {
            return Ok(());
        }

        log::debug!("backspace x{count}");

        // wtype -k sends a key event; repeat for each backspace
        let mut cmd = std::process::Command::new("wtype");
        for _ in 0..count {
            cmd.arg("-k").arg("BackSpace");
        }

        let status = cmd.status()?;

        if !status.success() {
            return Err(std::io::Error::other(format!("wtype backspace failed ({count}x): {status}"),
            ));
        }

        Ok(())
    }
}
