use serde::{Deserialize, Deserializer};
fn transcript<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Object(mut o) => o
            .remove("value")
            .and_then(|v| v.as_str().map(str::to_owned)),
        _ => None,
    })
}
#[derive(Debug, Default, Deserialize, Clone)]
pub struct HookInput {
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    #[serde(deserialize_with = "transcript", default)]
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
