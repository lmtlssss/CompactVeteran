mod atomic;
mod catalog;
mod config;
mod control;
mod git_checkpoint;
mod hook_input;
mod project_lock;
mod project_map;
mod state;
mod supervisor;
mod trust;
use hook_input::HookInput;
use std::{
    env, fs,
    io::{self, Read, Write},
    path::PathBuf,
};
fn home() -> PathBuf {
    env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}
fn run_main() -> io::Result<i32> {
    let mut a = env::args_os().skip(1);
    let command = a.next().and_then(|x| x.into_string().ok());
    match command.as_deref() {
        Some("refresh-catalog") => {
            catalog::refresh().map(|p| println!("{}", p.display()))?;
        }
        Some("install-config") => config::install()?,
        Some("restore-config") => config::restore()?,
        Some("config-status") => config::status()?,
        Some("trust") => trust::set(true)?,
        Some("untrust") => trust::set(false)?,
        Some("doctor") => trust::doctor()?,
        Some("hook") => {
            let k = a.next().unwrap_or_default();
            let mut s = String::new();
            io::stdin().read_to_string(&mut s)?;
            let i: HookInput = match serde_json::from_str(&s) {
                Ok(x) => x,
                Err(e) => {
                    println!(
                        "{}",
                        serde_json::json!({"continue":false,"stopReason":e.to_string()})
                    );
                    return Ok(0);
                }
            };
            if !i.is_sol_root() {
                println!("{{\"continue\":true}}");
                return Ok(0);
            }
            if k == "prompt" || k == "session-start" {
                if let Err(e) = state::merge_hook(&i) {
                    println!(
                        "{}",
                        serde_json::json!({"continue":false,"stopReason":e.to_string()})
                    );
                    return Ok(0);
                }
                println!("{{\"continue\":true}}")
            } else {
                match git_checkpoint::run(&i) {
                    Ok(c) => {
                        if k == "precompact" {
                            let mut stop = "Context compaction dodged.".to_string();
                            let notified = if let Some(p) = env::var_os("COMPACTVETERAN_SOCKET") {
                                let r = control::RestartRequest {
                                    map: c.map_path.to_string_lossy().into(),
                                    cwd: c.root.to_string_lossy().into(),
                                    model: i.model.clone().unwrap_or_default(),
                                };
                                control::notify(&PathBuf::from(p), &r).is_ok()
                            } else {
                                false
                            };
                            if !notified {
                                stop = format!(
                                    "Context compaction dodged. Run compactveteran continue {}",
                                    c.map_path.display()
                                );
                            }
                            let out = serde_json::json!({"continue":false,"stopReason":stop,"systemMessage":"Context compaction dodged."});
                            io::stdout().write_all(format!("{}\n", out).as_bytes())?;
                            io::stdout().flush()?
                        } else {
                            println!("{{\"continue\":true}}")
                        }
                    }
                    Err(e) => println!(
                        "{}",
                        serde_json::json!({"continue":false,"stopReason":e.to_string()})
                    ),
                }
            }
        }
        Some("supervisor") => return supervisor::run(a.collect(), None),
        Some("continue") => {
            let map = PathBuf::from(a.next().ok_or_else(|| io::Error::other("missing map"))?);
            let text = fs::read_to_string(&map)?;
            let root = text
                .lines()
                .find_map(|l| l.strip_prefix("- canonical root: "))
                .ok_or_else(|| io::Error::other("map has no canonical root"))?;
            if !PathBuf::from(root).is_dir() {
                return Err(io::Error::other("map root missing"));
            }
            return supervisor::run(
                Vec::new(),
                Some(control::RestartRequest {
                    map: map.to_string_lossy().into(),
                    cwd: root.into(),
                    model: "gpt-5.6-sol".into(),
                }),
            );
        }
        _ => return Ok(0),
    }
    Ok(0)
}

fn main() {
    if let Err(e) = run_main() {
        eprintln!("compactveteran: {e}");
        std::process::exit(1)
    }
}
