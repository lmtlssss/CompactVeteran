use crate::{atomic, home, state};
use std::{fs, io};
use toml_edit::{value, DocumentMut, Item, Table};
const KEYS: [&str; 3] = [
    "model_catalog_json",
    "model_context_window",
    "model_auto_compact_token_limit",
];
fn path() -> std::path::PathBuf {
    state::dir().join("config-ownership.toml")
}
pub fn install() -> io::Result<()> {
    let overlay = crate::catalog::refresh()?;
    let cp = home().join("config.toml");
    let mut d = fs::read_to_string(&cp)
        .unwrap_or_default()
        .parse::<DocumentMut>()
        .map_err(|e| io::Error::other(e.to_string()))?;
    let op = path();
    if !op.exists() {
        let mut own = DocumentMut::new();
        own["owned_overlay"] = value(overlay.display().to_string());
        let mut prior = Table::new();
        for key in KEYS {
            let mut t = Table::new();
            t["present"] = value(d.get(key).is_some());
            if let Some(item) = d.get(key) {
                t["value"] = item.clone();
            }
            prior[key] = Item::Table(t);
        }
        own["prior"] = Item::Table(prior);
        atomic::write(&op, own.to_string().as_bytes())?;
    }
    d["model_catalog_json"] = value(overlay.display().to_string());
    d.remove("model_context_window");
    d.remove("model_auto_compact_token_limit");
    atomic::write(&cp, d.to_string().as_bytes())
}
pub fn restore() -> io::Result<()> {
    let op = path();
    if !op.exists() {
        return Ok(());
    }
    let own = fs::read_to_string(&op)?
        .parse::<DocumentMut>()
        .map_err(|e| io::Error::other(e.to_string()))?;
    let mut d = fs::read_to_string(home().join("config.toml"))
        .unwrap_or_default()
        .parse::<DocumentMut>()
        .map_err(|e| io::Error::other(e.to_string()))?;
    let overlay = own["owned_overlay"].as_str().unwrap_or("");
    if d["model_catalog_json"].as_str() == Some(overlay) {
        restore_key(&mut d, &own, "model_catalog_json", true);
    }
    for key in [&KEYS[1], &KEYS[2]] {
        if d.get(key).is_none() {
            restore_key(&mut d, &own, key, false);
        }
    }
    atomic::write(&home().join("config.toml"), d.to_string().as_bytes())?;
    fs::remove_file(op)
}
fn restore_key(d: &mut DocumentMut, own: &DocumentMut, key: &str, remove_owned: bool) {
    let item = &own["prior"][key];
    if item["present"].as_bool() == Some(true) {
        d[key] = item["value"].clone();
    } else if remove_owned {
        d.remove(key);
    }
}
pub fn status() -> io::Result<()> {
    let op = path();
    let overlay = if op.exists() {
        fs::read_to_string(&op)?
            .parse::<DocumentMut>()
            .map_err(|e| io::Error::other(e.to_string()))?["owned_overlay"]
            .as_str()
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };
    let d = fs::read_to_string(home().join("config.toml"))
        .unwrap_or_default()
        .parse::<DocumentMut>()
        .map_err(|e| io::Error::other(e.to_string()))?;
    println!(
        "{{\"overlay_path\":{:?},\"owned\":{}}}",
        overlay,
        d["model_catalog_json"].as_str() == Some(&overlay)
    );
    Ok(())
}

pub fn is_owned() -> io::Result<bool> {
    let op = path();
    if !op.exists() {
        return Ok(false);
    }
    let own = fs::read_to_string(op)?
        .parse::<DocumentMut>()
        .map_err(|e| io::Error::other(e.to_string()))?;
    let overlay = own["owned_overlay"].as_str().unwrap_or("");
    let d = fs::read_to_string(home().join("config.toml"))
        .unwrap_or_default()
        .parse::<DocumentMut>()
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(d["model_catalog_json"].as_str() == Some(overlay))
}
