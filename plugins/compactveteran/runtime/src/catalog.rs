use crate::{atomic, state};
use serde_json::{json, Value};
use std::{fs, io, path::PathBuf};

pub fn refresh() -> io::Result<PathBuf> {
    let original: Value = serde_json::from_str(&fs::read_to_string(
        crate::home().join("models_cache.json"),
    )?)?;
    let mut patched = original.clone();
    let models = patched
        .get_mut("models")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| io::Error::other("models must be an array"))?;
    let mut seen = [0u8; 3];
    for model in models.iter_mut() {
        let obj = model
            .as_object_mut()
            .ok_or_else(|| io::Error::other("model must be an object"))?;
        match obj.get("slug").and_then(Value::as_str) {
            Some("gpt-5.6-sol") => {
                seen[0] += 1;
                obj.insert("context_window".into(), json!(1050000));
                obj.insert("max_context_window".into(), json!(1050000));
                obj.insert("auto_compact_token_limit".into(), json!(950000));
            }
            Some("gpt-5.6-terra") => seen[1] += 1,
            Some("gpt-5.6-luna") => seen[2] += 1,
            _ => {}
        }
    }
    if seen != [1, 1, 1] {
        return Err(io::Error::other(
            "catalog requires exactly one Sol, Terra, and Luna",
        ));
    }
    let before = original.get("models").and_then(Value::as_array).unwrap();
    for (i, model) in models.iter().enumerate() {
        if model.get("slug").and_then(Value::as_str) != Some("gpt-5.6-sol") && model != &before[i] {
            return Err(io::Error::other("non-Sol catalog entry changed"));
        }
    }
    let p = state::dir().join("models-overlay.json");
    atomic::write(&p, &serde_json::to_vec(&patched)?)?;
    Ok(p)
}
