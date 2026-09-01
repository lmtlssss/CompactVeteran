use crate::{hook_input::HookInput, project_lock::ProjectLock, project_map, state};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};
fn g(p: &Path, a: &[&str]) -> io::Result<String> {
    let o = Command::new("git").arg("-C").arg(p).args(a).output()?;
    if !o.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&o.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).trim().into())
}
pub fn root(c: &Path) -> io::Result<PathBuf> {
    if let Ok(x) = g(c, &["rev-parse", "--show-toplevel"]) {
        return fs::canonicalize(x);
    }
    fs::create_dir_all(c)?;
    let o = Command::new("git")
        .args(["init", "-b", "main"])
        .arg(c)
        .output()?;
    if !o.status.success() {
        return Err(io::Error::other("git init failed"));
    }
    fs::canonicalize(g(c, &["rev-parse", "--show-toplevel"])?)
}
pub fn root_hash(r: &Path) -> String {
    let mut h = Sha256::new();
    h.update(r.to_string_lossy().as_bytes());
    format!("{:x}", h.finalize())
}
pub struct Checkpoint {
    pub map_path: PathBuf,
    pub root: PathBuf,
}
pub fn run(i: &HookInput) -> io::Result<Checkpoint> {
    let s = state::merge_hook(i)?;
    let r = root(Path::new(i.cwd.as_deref().unwrap_or(".")))?;
    let h = root_hash(&r);
    let _l = ProjectLock::acquire(&h)?;
    if g(&r, &["branch", "--show-current"])?.is_empty() {
        return Err(io::Error::other("detached HEAD"));
    }
    let dirty = g(&r, &["status", "--porcelain", "--untracked-files=all"])?;
    guard(&dirty)?;
    if !dirty.is_empty() {
        g(&r, &["add", "-A"])?;
        let q = Command::new("git")
            .arg("-C")
            .arg(&r)
            .args(["diff", "--cached", "--quiet"])
            .status()?;
        if q.code() == Some(1) {
            let id = s
                .latest_turn_id
                .as_deref()
                .filter(|x| !x.is_empty())
                .unwrap_or(&s.session_id);
            let m = format!("compactveteran: checkpoint {}", &id[..id.len().min(12)]);
            g(
                &r,
                &[
                    "-c",
                    "user.name=CompactVeteran",
                    "-c",
                    "user.email=compactveteran@localhost",
                    "commit",
                    "-m",
                    &m,
                ],
            )?;
        } else if !q.success() {
            return Err(io::Error::other("git diff failed"));
        }
    }
    if !g(&r, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        return Err(io::Error::other("working tree not clean"));
    }
    state::record_project_session(&r.to_string_lossy(), &h, &s)?;
    let mp = project_map::write(&r, &h, &s)?;
    Ok(Checkpoint {
        map_path: mp,
        root: r,
    })
}
fn guard(s: &str) -> io::Result<()> {
    for l in s.lines() {
        let p = l.get(3..).unwrap_or("");
        let p = p.split_once(" -> ").map(|x| x.1).unwrap_or(p);
        let n = Path::new(p)
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if n == ".env"
            || n.starts_with(".env.")
            || n.starts_with("credentials")
            || n.starts_with("secrets")
            || n.ends_with(".pem")
            || n.ends_with(".key")
            || n == "id_rsa"
            || n == "id_ed25519"
        {
            return Err(io::Error::other("credential filename guard"));
        }
    }
    Ok(())
}
