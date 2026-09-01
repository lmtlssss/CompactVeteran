use crate::hook_input::HookInput;
use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SessionState {
    pub session_id: String,
    pub latest_turn_id: Option<String>,
    pub latest_prompt: Option<String>,
    pub transcript_path: Option<String>,
    pub cwd: Option<String>,
    pub model: Option<String>,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct SessionRef {
    pub session_id: String,
    pub transcript_path: Option<String>,
}
#[derive(Serialize, Deserialize, Default)]
pub struct ProjectState {
    pub canonical_root: String,
    pub sessions: Vec<SessionRef>,
}
pub fn dir() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")))
        .unwrap()
        .join("compactveteran")
}
fn put(p: &PathBuf, b: &[u8]) -> io::Result<()> {
    fs::create_dir_all(p.parent().unwrap())?;
    let t = p.with_file_name(format!(
        ".{}.{}.tmp",
        p.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    fs::write(&t, b)?;
    fs::rename(t, p)
}
pub fn load_session(id: &str) -> io::Result<Option<SessionState>> {
    if id.is_empty() {
        return Err(io::Error::other("missing session_id"));
    }
    let p = dir().join("sessions").join(format!("{id}.json"));
    if !p.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(p)?)?))
}
pub fn merge_hook(i: &HookInput) -> io::Result<SessionState> {
    let id = i
        .session_id
        .as_deref()
        .filter(|x| !x.is_empty())
        .ok_or_else(|| io::Error::other("missing session_id"))?;
    let mut s = load_session(id)?.unwrap_or(SessionState {
        session_id: id.into(),
        ..Default::default()
    });
    if i.turn_id.as_deref().is_some_and(|x| !x.is_empty()) {
        s.latest_turn_id = i.turn_id.clone()
    }
    if i.prompt.as_deref().is_some_and(|x| !x.is_empty()) {
        s.latest_prompt = i.prompt.clone()
    }
    if i.transcript_path.as_deref().is_some_and(|x| !x.is_empty()) {
        s.transcript_path = i.transcript_path.clone()
    }
    if i.cwd.as_deref().is_some_and(|x| !x.is_empty()) {
        s.cwd = i.cwd.clone()
    }
    if i.model.as_deref().is_some_and(|x| !x.is_empty()) {
        s.model = i.model.clone()
    }
    put(
        &dir().join("sessions").join(format!("{id}.json")),
        &serde_json::to_vec_pretty(&s).unwrap(),
    )?;
    Ok(s)
}
pub fn record_project_session(h: &str, s: &SessionState) -> io::Result<()> {
    let p = dir().join("projects").join(format!("{h}.json"));
    let mut x: ProjectState = if p.exists() {
        serde_json::from_slice(&fs::read(&p)?)?
    } else {
        Default::default()
    };
    if let Some(z) = x.sessions.iter_mut().find(|z| z.session_id == s.session_id) {
        z.transcript_path = s.transcript_path.clone()
    } else {
        x.sessions.push(SessionRef {
            session_id: s.session_id.clone(),
            transcript_path: s.transcript_path.clone(),
        })
    }
    x.canonical_root = s.cwd.clone().unwrap_or_default();
    put(&p, &serde_json::to_vec_pretty(&x).unwrap())
}
pub fn save(i: &HookInput) -> io::Result<()> {
    merge_hook(i).map(|_| ())
}
