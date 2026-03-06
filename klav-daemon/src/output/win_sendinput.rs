use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_BACK,
};

use super::OutputBackend;

/// Windows output backend using the `SendInput` API.
///
/// For ASCII characters, we use virtual key codes.
/// For Unicode characters (e.g. Japanese), we use `KEYEVENTF_UNICODE`
/// which sends the character directly regardless of the active keyboard layout.
pub struct WinSendInputOutput {
    _marker: (),
}

impl WinSendInputOutput {
    pub fn new() -> std::io::Result<Self> {
        log::info!("Windows SendInput output initialized");
        Ok(Self { _marker: () })
    }
}

impl OutputBackend for WinSendInputOutput {
    fn type_text(&mut self, text: &str) -> std::io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        log::debug!("typing: {text}");

        // Use KEYEVENTF_UNICODE for each character — this works for all
        // Unicode code points (including Japanese) regardless of keyboard layout.
        let mut inputs = Vec::new();

        for ch in text.chars() {
            // UTF-16 encode the character (may produce surrogate pairs)
            let mut buf = [0u16; 2];
            let encoded = ch.encode_utf16(&mut buf);

            for &unit in encoded {
                // Key down
                inputs.push(make_unicode_input(unit, KEYBD_EVENT_FLAGS(0)));
                // Key up
                inputs.push(make_unicode_input(unit, KEYEVENTF_KEYUP));
            }
        }

        send_inputs(&inputs)
    }

    fn backspace(&mut self, count: usize) -> std::io::Result<()> {
        if count == 0 {
            return Ok(());
        }

        log::debug!("backspace x{count}");

        let mut inputs = Vec::with_capacity(count * 2);

        for _ in 0..count {
            // Key down
            inputs.push(make_vk_input(VK_BACK, KEYBD_EVENT_FLAGS(0)));
            // Key up
            inputs.push(make_vk_input(VK_BACK, KEYEVENTF_KEYUP));
        }

        send_inputs(&inputs)
    }
}

/// Create an INPUT structure for a Unicode character.
fn make_unicode_input(scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: scan,
                dwFlags: KEYEVENTF_UNICODE | flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Create an INPUT structure for a virtual key code.
fn make_vk_input(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Send a batch of input events via SendInput.
fn send_inputs(inputs: &[INPUT]) -> std::io::Result<()> {
    let sent = unsafe {
        SendInput(inputs, std::mem::size_of::<INPUT>() as i32)
    };
    if sent != inputs.len() as u32 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("SendInput: sent {sent}/{} events", inputs.len()),
        ));
    }
    Ok(())
}
