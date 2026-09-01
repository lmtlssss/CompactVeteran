use crate::{atomic, home, state_dir};
use std::{fs, io};
use toml_edit::DocumentMut;
pub fn install() -> io::Result<()> {
    let p = home().join("config.toml");
    let mut d = fs::read_to_string(&p)
        .unwrap_or_default()
        .parse::<DocumentMut>()
        .map_err(|e| io::Error::other(e.to_string()))?;
    atomic(&state_dir().join("config-ownership.json"), b"{}")?;
    d["model_catalog_json"] = toml_edit::value(crate::catalog::refresh()?.display().to_string());
    d.remove("model_context_window");
    d.remove("model_auto_compact_token_limit");
    atomic(&p, d.to_string().as_bytes())
}
pub fn restore() -> io::Result<()> {
    Ok(())
}
