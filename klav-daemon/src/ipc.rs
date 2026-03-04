use std::io;
use std::path::{Path, PathBuf};

/// IPC server for communication between klav-daemon and klav-gui.
///
/// Phase 0: Stub implementation. Full IPC will be implemented in Phase 3.
pub struct IpcServer {
    socket_path: PathBuf,
}

impl IpcServer {
    pub fn socket_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(runtime_dir).join("klav.sock")
    }

    pub fn bind() -> io::Result<Self> {
        let socket_path = Self::socket_path();

        // Remove stale socket
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        log::info!("IPC socket: {}", socket_path.display());
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
