use crate::state;
use serde_json::Value;
use std::{
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
pub struct ProjectLock {
    path: PathBuf,
    pid: u32,
    created_at: u64,
}
impl ProjectLock {
    pub fn acquire(hash: &str) -> io::Result<Self> {
        let d = state::dir().join("locks");
        fs::create_dir_all(&d)?;
        let path = d.join(format!("{hash}.lock"));
        let pid = std::process::id();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let make = || {
            fs::write(
                &path,
                serde_json::json!({"pid":pid,"created_at":now}).to_string(),
            )
        };
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => {
                make()?;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let old: Value = serde_json::from_slice(&fs::read(&path)?).unwrap_or_default();
                let op = old["pid"].as_u64().unwrap_or(0);
                let ot = old["created_at"].as_u64().unwrap_or(0);
                let stale = op == 0
                    || fs::metadata(format!("/proc/{op}")).is_err()
                    || now.saturating_sub(ot) > 600;
                if !stale {
                    return Err(io::Error::other("project lock active"));
                }
                fs::remove_file(&path)?;
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                make()?;
            }
            Err(e) => return Err(e),
        }
        Ok(Self {
            path,
            pid,
            created_at: now,
        })
    }
}
impl Drop for ProjectLock {
    fn drop(&mut self) {
        if let Ok(v) = serde_json::from_slice::<Value>(&fs::read(&self.path).unwrap_or_default()) {
            if v["pid"].as_u64() == Some(self.pid as u64)
                && v["created_at"].as_u64() == Some(self.created_at)
            {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}
