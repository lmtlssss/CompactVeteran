use crate::{atomic, hook_input::HookInput};
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
    #[serde(default)]
    pub last_assistant_message: Option<String>,
    #[serde(default)]
    pub prompt_turn_id: Option<String>,
    #[serde(default)]
    pub completed_turn_id: Option<String>,
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
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub last_assistant_result: Option<String>,
    #[serde(default)]
    pub prompt_pending: bool,
}
pub fn dir() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")))
        .unwrap()
        .join("compactveteran")
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
pub fn load_project(root_hash: &str) -> io::Result<Option<ProjectState>> {
    let p = dir().join("projects").join(format!("{root_hash}.json"));
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
        s.latest_prompt = i.prompt.clone();
        s.prompt_turn_id = i.turn_id.clone();
    }
    if i.last_assistant_message
        .as_deref()
        .is_some_and(|x| !x.is_empty())
    {
        s.last_assistant_message = i.last_assistant_message.clone();
        s.completed_turn_id = i.turn_id.clone();
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
    atomic::write(
        &dir().join("sessions").join(format!("{id}.json")),
        &serde_json::to_vec_pretty(&s).unwrap(),
    )?;
    Ok(s)
}
pub fn record_project_session(root: &str, h: &str, s: &SessionState) -> io::Result<()> {
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
    x.canonical_root = root.into();
    if let Some(v) = s.latest_prompt.as_ref().filter(|v| !v.is_empty()) {
        x.objective = Some(v.clone());
        x.prompt_pending = s.prompt_turn_id.is_some() && s.prompt_turn_id != s.completed_turn_id;
    }
    if let Some(v) = s.last_assistant_message.as_ref().filter(|v| !v.is_empty()) {
        x.last_assistant_result = Some(v.clone());
        if s.latest_prompt.is_none() {
            x.prompt_pending = false;
        } else if s.completed_turn_id == s.prompt_turn_id {
            x.prompt_pending = false;
        }
    }
    atomic::write(&p, &serde_json::to_vec_pretty(&x).unwrap())
}
