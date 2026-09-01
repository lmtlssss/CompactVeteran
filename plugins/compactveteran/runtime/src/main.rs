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
use hook_input::HookInput;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};
fn home() -> PathBuf {
    env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}
fn digest(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    format!("{:x}", h.finalize())
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
                state::merge_hook(&i)?;
                println!("{{\"continue\":true}}")
            } else {
                match git_checkpoint::run(&i) {
                    Ok(c) => {
                        if k == "precompact" {
                            let mut out = serde_json::json!({"continue":false,"stopReason":"Context compaction dodged.","systemMessage":"Context compaction dodged."});
                            io::stdout().write_all(format!("{}\n", out).as_bytes())?;
                            io::stdout().flush()?;
                            if let Some(p) = env::var_os("COMPACTVETERAN_SOCKET") {
                                let r = control::RestartRequest {
                                    map: c.map_path.to_string_lossy().into(),
                                    cwd: c.root.to_string_lossy().into(),
                                    model: i.model.clone().unwrap_or_default(),
                                };
                                if control::notify(&PathBuf::from(p), &r).is_err() {
                                    out["stopReason"] = serde_json::json!(format!(
                                        "Context compaction dodged. Run compactveteran continue {}",
                                        r.map
                                    ));
                                    println!("{}", out)
                                }
                            }
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

fn run(f: impl FnOnce() -> io::Result<()>) {
    if let Err(e) = f() {
        eprintln!("compactveteran: {e}");
        std::process::exit(1);
    }
}
