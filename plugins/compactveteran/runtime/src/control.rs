use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    os::unix::net::UnixStream,
    path::Path,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RestartRequest {
    pub map: String,
    pub cwd: String,
    pub model: String,
}
pub fn handoff_prompt(map: &str) -> String {
    format!("Read {map}. Use its Objective, Cursor, and Next action. Continue immediately from local HEAD. Open a referenced raw log only if a specific ambiguity blocks the next action.")
}

pub fn notify(path: &Path, request: &RestartRequest) -> io::Result<()> {
    let mut s = UnixStream::connect(path)?;
    serde_json::to_writer(&mut s, request).map_err(io::Error::other)?;
    s.write_all(b"\n")?;
    s.flush()
}

pub struct Listener {
    pub socket: std::os::unix::net::UnixListener,
    path: std::path::PathBuf,
}
impl Listener {
    pub fn bind(path: std::path::PathBuf) -> io::Result<Self> {
        if path.exists() {
            fs::remove_file(&path)?;
        }
        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?;
        }
        let socket = std::os::unix::net::UnixListener::bind(&path)?;
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket, path })
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
