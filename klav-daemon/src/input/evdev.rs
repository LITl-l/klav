use std::fs;
use std::path::PathBuf;

use evdev::{Device, InputEventKind, Key};

use super::{InputBackend, KeyEventKind, RawKeyEvent};

/// Linux evdev input backend.
pub struct EvdevInput {
    device: Device,
}

impl EvdevInput {
    /// Open a specific evdev device by path.
    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        let device = Device::open(&path)?;
        log::info!(
            "opened input device: {} ({})",
            device.name().unwrap_or("unknown"),
            path.display()
        );
        Ok(Self { device })
    }

    /// Auto-detect the first keyboard device.
    pub fn auto_detect() -> std::io::Result<Self> {
        let input_dir = PathBuf::from("/dev/input");
        let mut entries: Vec<_> = fs::read_dir(&input_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("event"))
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            if let Ok(device) = Device::open(&path) {
                // Check if this device has key events and looks like a keyboard
                let has_keys = device
                    .supported_keys()
                    .is_some_and(|keys| keys.contains(Key::KEY_A) && keys.contains(Key::KEY_Z));
                if has_keys {
                    log::info!(
                        "auto-detected keyboard: {} ({})",
                        device.name().unwrap_or("unknown"),
                        path.display()
                    );
                    return Ok(Self { device });
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no keyboard device found in /dev/input/",
        ))
    }

    pub fn device_name(&self) -> &str {
        self.device.name().unwrap_or("unknown")
    }
}

impl InputBackend for EvdevInput {
    fn next_event(&mut self) -> std::io::Result<RawKeyEvent> {
        loop {
            let events = self.device.fetch_events()?;
            for event in events {
                if let InputEventKind::Key(key) = event.kind() {
                    let kind = match event.value() {
                        0 => KeyEventKind::Release,
                        1 => KeyEventKind::Press,
                        2 => continue, // repeat — ignore
                        _ => continue,
                    };
                    return Ok(RawKeyEvent { code: key.0, kind });
                }
            }
        }
    }

    fn grab(&mut self) -> std::io::Result<()> {
        self.device
            .grab()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))
    }

    fn ungrab(&mut self) -> std::io::Result<()> {
        self.device.ungrab().map_err(std::io::Error::other)
    }
}
