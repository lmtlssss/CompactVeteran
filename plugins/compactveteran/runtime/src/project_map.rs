use crate::{atomic, home, hook_input::HookInput};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};
fn g(r: &Path, a: &[&str]) -> String {
    String::from_utf8_lossy(
        &Command::new("git")
            .arg("-C")
            .arg(r)
            .args(a)
            .output()
            .unwrap()
            .stdout,
    )
    .trim()
    .into()
}
pub fn write(r: &Path, i: &HookInput) -> io::Result<PathBuf> {
    let mut h = Sha256::new();
    h.update(r.to_string_lossy().as_bytes());
    let p = home()
        .join("project-maps")
        .join(format!("{:x}.md", h.finalize()));
    let tr = i.transcript_path.clone().unwrap_or_default();
    let b = fs::read(&tr).unwrap_or_default();
    let txt=format!("# CompactVeteran\n\n- root: {}\n- remote: {}\n- branch: {}\n- upstream: {}\n- HEAD: {}\n- status: {}\n- transcript: {}\n- transcript_sha256: {}\n- session_lineage: {}\n- latest_user_directive:\n```\n{}\n```\n\n## recent commits\n{}\n\n## resume\nRead this map, inspect Git and referenced raw logs, and continue the unfinished work from HEAD.\n",r.display(),g(r,&["remote","-v"]),g(r,&["branch","--show-current"]),g(r,&["rev-parse","--abbrev-ref","--symbolic-full-name","@{u"]),g(r,&["rev-parse","HEAD"]),g(r,&["status","--porcelain","--untracked-files=all"]),tr,crate::digest(&b),i.session_id.clone().unwrap_or_default(),i.prompt.clone().unwrap_or_default(),g(r,&["log","-10","--oneline"]));
    atomic(&p, txt.as_bytes())?;
    Ok(p)
}
