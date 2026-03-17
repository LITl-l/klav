use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, OnceLock};
use std::thread;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::{InputBackend, KeyEventKind, RawKeyEvent};

struct HookState {
    sender: mpsc::Sender<RawKeyEvent>,
    grabbed: AtomicBool,
}

static HOOK_STATE: OnceLock<HookState> = OnceLock::new();

/// Windows low-level keyboard hook input backend.
pub struct Win32HookInput {
    receiver: mpsc::Receiver<RawKeyEvent>,
    _hook_thread: thread::JoinHandle<()>,
}

impl Win32HookInput {
    pub fn new() -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel();

        HOOK_STATE
            .set(HookState {
                sender,
                grabbed: AtomicBool::new(false),
            })
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "only one Win32HookInput instance can exist at a time",
                )
            })?;

        let hook_thread = thread::spawn(|| {
            unsafe {
                let hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(keyboard_hook_proc),
                    GetModuleHandleW(None).unwrap_or_default(),
                    0,
                )
                .expect("failed to install keyboard hook");

                // Message pump — required to keep the hook alive.
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    // WM_QUIT will cause GetMessageW to return FALSE, ending the loop.
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                let _ = UnhookWindowsHookEx(hook);
            }
        });

        Ok(Self {
            receiver,
            _hook_thread: hook_thread,
        })
    }
}

impl InputBackend for Win32HookInput {
    fn next_event(&mut self) -> std::io::Result<RawKeyEvent> {
        self.receiver.recv().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "keyboard hook thread has stopped",
            )
        })
    }

    fn grab(&mut self) -> std::io::Result<()> {
        if let Some(state) = HOOK_STATE.get() {
            state.grabbed.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    fn ungrab(&mut self) -> std::io::Result<()> {
        if let Some(state) = HOOK_STATE.get() {
            state.grabbed.store(false, Ordering::Relaxed);
        }
        Ok(())
    }
}

unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kb = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };

        // Skip injected events (e.g. from our own SendInput) to avoid feedback loops.
        if kb.flags.contains(LLKHF_INJECTED) {
            return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
        }

        let kind = match w_param.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => KeyEventKind::Press,
            WM_KEYUP | WM_SYSKEYUP => KeyEventKind::Release,
            _ => return unsafe { CallNextHookEx(None, n_code, w_param, l_param) },
        };

        let event = RawKeyEvent {
            code: kb.vkCode as u16,
            kind,
        };

        if let Some(state) = HOOK_STATE.get() {
            let _ = state.sender.send(event);

            if state.grabbed.load(Ordering::Relaxed) {
                return LRESULT(1);
            }
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}
