use crate::{atomic, home, state};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};
fn g(r: &Path, a: &[&str]) -> io::Result<String> {
    let o = Command::new("git").arg("-C").arg(r).args(a).output()?;
    if !o.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&o.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).trim().into())
}
fn dh(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    format!("{:x}", h.finalize())
}
pub fn write(r: &Path, h: &str, s: &state::SessionState) -> io::Result<PathBuf> {
    let t = s.transcript_path.clone().unwrap_or_default();
    let th = if !t.is_empty() && Path::new(&t).is_file() {
        dh(&fs::read(&t)?)
    } else {
        "missing".into()
    };
    let clean = g(r, &["status", "--porcelain", "--untracked-files=all"])?.is_empty();
    let mut x=format!("# CompactVeteran handoff\n\n## Scope\n\n- canonical root: {}\n- branch: {}\n- HEAD: {}\n- clean: {}\n- current session: {}\n- transcript: {}\n- transcript SHA256: {}\n\n## Latest directive\n\n",r.display(),g(r,&["branch","--show-current"] )?,g(r,&["rev-parse","HEAD"] )?,clean,s.session_id,t,th);
    let f = if s.latest_prompt.as_deref().unwrap_or("").contains("```") {
        "````"
    } else {
        "```"
    };
    x += f;
    x.push_str("text\n");
    x.push_str(s.latest_prompt.as_deref().unwrap_or(""));
    x.push('\n');
    x += f;
    x.push_str("\n\n## Recent commits\n\n```text\n");
    x += &g(r, &["log", "-10", "--oneline"])?;
    x.push_str("\n```\n\n## Session lineage\n\n```text\n");
    if let Some(project) = state::load_project(h)? {
        for session in project.sessions {
            x.push_str(&session.session_id);
            x.push('\t');
            x.push_str(session.transcript_path.as_deref().unwrap_or(""));
            x.push('\n');
        }
    }
    x.push_str("```\n\n## Sources\n\n");
    for name in ["AGENTS.md", "ROADMAP.md", "PRODUCT_ROADMAP.md", "README.md"] {
        let p = r.join(name);
        if p.exists() {
            x.push_str(&format!("- {}\n", p.display()));
        }
    }
    let registry = home().join("project-truth/registry.toml");
    if registry.exists() {
        x.push_str(&format!("- {}\n", registry.display()));
    }
    x.push_str("\n## Resume\n\nRead this map. Treat it as a map, inspect Git and the referenced raw logs, and continue the unfinished work from HEAD.\n");
    let p = home().join("project-maps").join(format!("{h}.md"));
    atomic::write(&p, x.as_bytes())?;
    Ok(p)
}
