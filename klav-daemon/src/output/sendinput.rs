use windows::Win32::UI::Input::KeyboardAndMouse::*;

use super::OutputBackend;

/// Windows output backend using the SendInput API.
pub struct SendInputOutput;

impl SendInputOutput {
    pub fn new() -> Self {
        Self
    }
}

impl OutputBackend for SendInputOutput {
    fn type_text(&mut self, text: &str) -> std::io::Result<()> {
        let mut inputs: Vec<INPUT> = Vec::new();

        for ch in text.encode_utf16() {
            inputs.push(unicode_input(ch, KEYEVENTF_UNICODE));
            inputs.push(unicode_input(ch, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
        }

        send(&inputs)
    }

    fn backspace(&mut self, count: usize) -> std::io::Result<()> {
        let mut inputs: Vec<INPUT> = Vec::new();

        for _ in 0..count {
            inputs.push(vk_input(VK_BACK, KEYBD_EVENT_FLAGS(0)));
            inputs.push(vk_input(VK_BACK, KEYEVENTF_KEYUP));
        }

        send(&inputs)
    }
}

fn unicode_input(scan_code: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: scan_code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn vk_input(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
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

fn send(inputs: &[INPUT]) -> std::io::Result<()> {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
