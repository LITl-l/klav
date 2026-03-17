/// Abstraction over platform-specific text output.
pub trait OutputBackend {
    /// Type a string of text as keyboard input.
    fn type_text(&mut self, text: &str) -> std::io::Result<()>;

    /// Delete N characters (backspace).
    fn backspace(&mut self, count: usize) -> std::io::Result<()>;
}

#[cfg(target_os = "linux")]
pub mod xdotool;

#[cfg(target_os = "linux")]
pub mod wtype;

#[cfg(target_os = "linux")]
pub mod fcitx5;

#[cfg(target_os = "linux")]
pub mod ibus;

#[cfg(target_os = "windows")]
pub mod sendinput;

#[cfg(target_os = "linux")]
pub fn create_backend(backend_name: &str) -> std::io::Result<Box<dyn OutputBackend>> {
    match backend_name {
        "auto" => auto_detect(),
        "xdotool" => Ok(Box::new(xdotool::XdotoolOutput::new()?)),
        "wtype" => Ok(Box::new(wtype::WtypeOutput::new()?)),
        "fcitx5" => Ok(Box::new(fcitx5::Fcitx5Output::new()?)),
        "ibus" => Ok(Box::new(ibus::IbusOutput::new()?)),
        other => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unknown output backend: {other}"),
        )),
    }
}

#[cfg(target_os = "linux")]
fn auto_detect() -> std::io::Result<Box<dyn OutputBackend>> {
    // Priority: fcitx5 > ibus > wtype (Wayland) > xdotool (X11)
    if fcitx5::Fcitx5Output::is_available() {
        log::info!("auto-detected: fcitx5");
        return Ok(Box::new(fcitx5::Fcitx5Output::new()?));
    }

    if ibus::IbusOutput::is_available() {
        log::info!("auto-detected: ibus");
        return Ok(Box::new(ibus::IbusOutput::new()?));
    }

    // Check if running under Wayland
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v == "wayland");

    if is_wayland {
        match wtype::WtypeOutput::new() {
            Ok(backend) => {
                log::info!("auto-detected: wtype (Wayland)");
                return Ok(Box::new(backend));
            }
            Err(e) => {
                log::warn!("wtype not available on Wayland: {e}; trying xdotool");
            }
        }
    }

    // Fallback to xdotool
    Ok(Box::new(xdotool::XdotoolOutput::new()?))
}

#[cfg(target_os = "windows")]
pub fn create_backend(backend_name: &str) -> std::io::Result<Box<dyn OutputBackend>> {
    match backend_name {
        "auto" | "sendinput" => {
            log::info!("output backend: SendInput (Windows)");
            Ok(Box::new(sendinput::SendInputOutput::new()))
        }
        other => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unknown output backend for Windows: {other}"),
        )),
    }
}
