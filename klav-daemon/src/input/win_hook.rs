use std::sync::mpsc;
use std::thread;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    KBDLLHOOKSTRUCT, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT_FLAGS,
    LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::{InputBackend, KeyEventKind, RawKeyEvent};

/// Windows low-level keyboard hook input backend.
///
/// Uses `SetWindowsHookExW(WH_KEYBOARD_LL)` to intercept all keyboard input
/// at a low level, then sends events through a channel. A background thread
/// runs the Windows message pump required by the hook.
pub struct WinHookInput {
    rx: mpsc::Receiver<RawKeyEvent>,
    /// Whether we are "grabbing" (suppressing events from reaching other apps).
    grabbing: bool,
}

/// Thread-local channel sender used by the hook callback.
///
/// The hook callback is a bare `extern "system"` function, so we use
/// thread-local storage to access the sender and grab state.
thread_local! {
    static HOOK_TX: std::cell::RefCell<Option<mpsc::SyncSender<RawKeyEvent>>> =
        const { std::cell::RefCell::new(None) };
    static HOOK_GRAB: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

impl WinHookInput {
    /// Install a low-level keyboard hook and start the message pump thread.
    pub fn install() -> std::io::Result<Self> {
        let (tx, rx) = mpsc::sync_channel::<RawKeyEvent>(256);

        // The hook + message pump must live on a dedicated thread because
        // GetMessageW blocks, and the hook callback is called on the
        // thread that installed the hook.
        thread::Builder::new()
            .name("klav-kb-hook".into())
            .spawn(move || {
                HOOK_TX.with(|cell| {
                    *cell.borrow_mut() = Some(tx);
                });

                let hook = unsafe {
                    SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_keyboard_proc), None, 0)
                };

                match hook {
                    Ok(h) => {
                        log::info!("low-level keyboard hook installed");
                        // Run message pump — required for the hook to work.
                        run_message_pump();
                        let _ = unsafe { UnhookWindowsHookEx(h) };
                    }
                    Err(e) => {
                        log::error!("failed to install keyboard hook: {e}");
                    }
                }
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(Self {
            rx,
            grabbing: false,
        })
    }

    pub fn device_name(&self) -> &str {
        "Windows keyboard hook"
    }
}

impl InputBackend for WinHookInput {
    fn next_event(&mut self) -> std::io::Result<RawKeyEvent> {
        self.rx.recv().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "keyboard hook thread terminated",
            )
        })
    }

    fn grab(&mut self) -> std::io::Result<()> {
        self.grabbing = true;
        // We communicate the grab state via a global; the hook callback
        // will suppress events when grab is active.
        // NOTE: in this design the grab flag is on the main thread, not
        // the hook thread. Since WinHookInput is consumed linearly (the
        // hook callback checks its own TLS), we set the flag on the hook
        // thread via a special event. For simplicity in Phase 0, we don't
        // actually suppress — the hook still passes events through.
        // Full grab (suppression) can be added later.
        log::info!("keyboard grab requested (note: suppression not yet implemented on Windows)");
        Ok(())
    }

    fn ungrab(&mut self) -> std::io::Result<()> {
        self.grabbing = false;
        log::info!("keyboard ungrab requested");
        Ok(())
    }
}

/// Run the Windows message pump on the current thread.
fn run_message_pump() {
    let mut msg = MSG::default();
    loop {
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if !ret.as_bool() {
            break; // WM_QUIT or error
        }
        unsafe {
            let _ = DispatchMessageW(&msg);
        }
    }
}

/// Low-level keyboard hook callback.
///
/// Called by Windows on the hook thread whenever a keyboard event occurs.
unsafe extern "system" fn ll_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

        // Skip injected events (our own SendInput calls)
        if kb.flags.contains(KBDLLHOOKSTRUCT_FLAGS(LLKHF_INJECTED.0)) {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let kind = match wparam.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => Some(KeyEventKind::Press),
            WM_KEYUP | WM_SYSKEYUP => Some(KeyEventKind::Release),
            _ => None,
        };

        if let Some(kind) = kind {
            let vk = kb.vkCode as u16;
            let event = RawKeyEvent { code: vk, kind };

            HOOK_TX.with(|cell| {
                if let Some(ref tx) = *cell.borrow() {
                    // Non-blocking send — drop events if buffer is full
                    let _ = tx.try_send(event);
                }
            });
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}
