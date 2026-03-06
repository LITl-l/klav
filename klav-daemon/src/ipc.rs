use std::io;
use std::path::{Path, PathBuf};

/// IPC server for communication between klav-daemon and klav-gui.
///
/// Phase 0: Stub implementation. Full IPC will be implemented in Phase 3.
/// On Linux, uses a Unix domain socket.
/// On Windows, uses a file-based marker in %LOCALAPPDATA% (Named Pipes in Phase 3).
pub struct IpcServer {
    socket_path: PathBuf,
}

impl IpcServer {
    pub fn socket_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            // On Windows, use %LOCALAPPDATA%\klav\ for IPC marker.
            // Full Named Pipe IPC (\\.\pipe\klav) will come in Phase 3.
            let local_app_data = std::env::var("LOCALAPPDATA")
                .unwrap_or_else(|_| {
                    let home = std::env::var("USERPROFILE")
                        .unwrap_or_else(|_| "C:\\Users\\Default".to_string());
                    format!("{home}\\AppData\\Local")
                });
            PathBuf::from(local_app_data).join("klav").join("klav.lock")
        }
        #[cfg(not(target_os = "windows"))]
        {
            let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                .unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(runtime_dir).join("klav.sock")
        }
    }

    pub fn bind() -> io::Result<Self> {
        let socket_path = Self::socket_path();

        // Ensure parent directory exists (relevant on Windows)
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Remove stale socket/lock
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        log::info!("IPC path: {}", socket_path.display());
        Ok(Self { socket_path })
    }

    pub fn path(&self) -> &Path {
        &self.socket_path
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
