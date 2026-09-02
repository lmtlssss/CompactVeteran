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
fn clip(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.into();
    }
    let marker = "\n[clipped; full text is in the raw transcript]\n";
    let room = max.saturating_sub(marker.len());
    let mut n = room / 2;
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    let mut tail = room - n;
    while !s.is_char_boundary(s.len() - tail) {
        tail -= 1;
    }
    format!("{}{}{}", &s[..n], marker, &s[s.len() - tail..])
}
pub fn write(r: &Path, h: &str, s: &state::SessionState) -> io::Result<PathBuf> {
    let project = state::load_project(h)?.unwrap_or_default();
    let t = s.transcript_path.clone().unwrap_or_default();
    let (tb, th) = if !t.is_empty() && Path::new(&t).is_file() {
        let b = fs::read(&t)?;
        (b.len(), dh(&b))
    } else {
        (0, "missing".into())
    };
    let branch = g(r, &["branch", "--show-current"])?;
    let head = g(r, &["rev-parse", "HEAD"])?;
    let clean = g(r, &["status", "--porcelain", "--untracked-files=all"])?.is_empty();
    let objective = project
        .objective
        .as_deref()
        .filter(|x| !x.is_empty())
        .or(s.latest_prompt.as_deref())
        .unwrap_or("");
    let cursor = project
        .last_assistant_result
        .as_deref()
        .filter(|x| !x.is_empty())
        .or(s.last_assistant_message.as_deref())
        .unwrap_or("");
    let prompt_state = if project.prompt_pending {
        "pending"
    } else {
        "completed"
    };
    let next = if project.prompt_pending {
        "Answer the Objective exactly once. Cursor is the prior completed boundary. Do not repeat Cursor work. Use listed project sources or the raw transcript only as needed for a specific ambiguity."
    } else {
        "The Objective is already answered. Do not answer or restart it. Continue only an explicit unresolved action named by Cursor; if none exists, stop without inventing or retracing work. Use listed project sources or the raw transcript only as needed for a specific ambiguity."
    };
    let mut x = format!("# CompactVeteran handoff\n\n## Scope\n\n- canonical root: {}\n- branch: {}\n- HEAD: {}\n- clean: {}\n- current session: {}\n- transcript: {}\n- transcript prefix bytes: {}\n- transcript prefix SHA256: {}\n- prompt state: {}\n\n## Objective\n\n{}\n\n## Cursor\n\n{}\n\n## Next action\n\n{}\n\n## Recent commits\n\n```text\n{}\n```\n\n## Sources\n\n", r.display(), branch, head, clean, s.session_id, t, tb, th, prompt_state, clip(objective, 6000), clip(cursor, 4000), next, g(r, &["log", "-5", "--oneline"])?);
    for name in ["AGENTS.md", "ROADMAP.md", "PRODUCT_ROADMAP.md", "README.md"] {
        if r.join(name).exists() {
            x.push_str(&format!("- {}\n", r.join(name).display()));
        }
    }
    let registry = home().join("project-truth/registry.toml");
    if registry.exists() {
        x.push_str(&format!("- {}\n", registry.display()));
    }
    x.push_str("\n## Recovery pointers\n\n- transcript: ");
    x.push_str(&t);
    x.push_str("\n- transcript prefix bytes: ");
    x.push_str(&tb.to_string());
    x.push_str("\n- transcript prefix SHA256: ");
    x.push_str(&th);
    x.push_str("\n\n### Session lineage\n\n");
    for z in project.sessions.iter().rev().take(3).rev() {
        x.push_str(&format!(
            "- {}\t{}\n",
            z.session_id,
            z.transcript_path.as_deref().unwrap_or("")
        ));
    }
    if x.len() > 16384 {
        return Err(io::Error::other("project map exceeds 16384 bytes"));
    }
    let p = home().join("project-maps").join(format!("{h}.md"));
    atomic::write(&p, x.as_bytes())?;
    Ok(p)
}
