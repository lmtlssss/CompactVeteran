use crate::{hook_input::HookInput, project_map};
use std::{io, path::PathBuf, process::Command};
fn g(p: &PathBuf, a: &[&str]) -> io::Result<String> {
    let o = Command::new("git").arg("-C").arg(p).args(a).output()?;
    if !o.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&o.stderr).into_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).trim().into())
}
pub fn run(i: &HookInput) -> io::Result<PathBuf> {
    let c = PathBuf::from(i.cwd.clone().unwrap_or_else(|| ".".into()));
    let r = PathBuf::from(g(&c, &["rev-parse", "--show-toplevel"]).or_else(|_| {
        let o = Command::new("git")
            .args(["init", "-b", "main"])
            .arg(&c)
            .output()?;
        if !o.status.success() {
            return Err(io::Error::other("git init failed"));
        }
        g(&c, &["rev-parse", "--show-toplevel"])
    })?);
    let s = g(&r, &["status", "--porcelain", "--untracked-files=all"])?;
    if !s.is_empty() {
        if g(&r, &["branch", "--show-current"])
            .unwrap_or_default()
            .is_empty()
        {
            return Err(io::Error::other("detached HEAD"));
        }
        for l in s.lines() {
            let p = l.get(3..).unwrap_or("").to_ascii_lowercase();
            if p.contains(".env")
                || p.contains("credentials")
                || p.contains("secrets")
                || p.ends_with(".pem")
                || p.ends_with(".key")
                || p.ends_with("id_rsa")
                || p.ends_with("id_ed25519")
            {
                return Err(io::Error::other("credential filename guard"));
            }
        }
        g(&r, &["add", "-A"])?;
        g(&r, &["diff", "--cached", "--check"])?;
        g(
            &r,
            &[
                "-c",
                "user.name=CompactVeteran",
                "-c",
                "user.email=compactveteran@localhost",
                "commit",
                "-m",
                "compactveteran: checkpoint",
            ],
        )?;
    }
    project_map::write(&r, i)
}
