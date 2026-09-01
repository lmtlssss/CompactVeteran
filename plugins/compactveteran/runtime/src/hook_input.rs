use serde::Deserialize;
#[derive(Debug, Default, Deserialize, Clone)]
pub struct HookInput {
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub transcript_path: Option<String>,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub trigger: Option<String>,
    pub last_assistant_message: Option<String>,
}
impl HookInput {
    pub fn is_sol_root(&self) -> bool {
        self.model.as_deref() == Some("gpt-5.6-sol")
            && self.agent_id.is_none()
            && self.agent_type.is_none()
    }
}
