mod catalog;
mod config;
mod git_checkpoint;
mod hook_input;
mod project_map;
mod state;
use hook_input::HookInput;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{self, Read},
    path::PathBuf,
    process::{Command, Stdio},
};
fn home() -> PathBuf {
    env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".codex")))
        .unwrap()
}
fn state_dir() -> PathBuf {
    state::dir()
}
fn atomic(p: &std::path::Path, b: &[u8]) -> io::Result<()> {
    fs::create_dir_all(p.parent().unwrap())?;
    let t = p.with_extension("tmp");
    fs::write(&t, b)?;
    fs::rename(t, p)
}
fn digest(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    format!("{:x}", h.finalize())
}
fn main() {
    let mut a = env::args().skip(1);
    match a.next().as_deref() {
        Some("refresh-catalog") => println!("{}", catalog::refresh().unwrap().display()),
        Some("install-config") => config::install().unwrap(),
        Some("restore-config") => config::restore().unwrap(),
        Some("hook") => {
            let k = a.next().unwrap_or_default();
            let mut s = String::new();
            io::stdin().read_to_string(&mut s).unwrap();
            let i: HookInput = serde_json::from_str(&s).unwrap_or_default();
            if !i.is_sol_root() {
                println!("{{\"continue\":true}}");
                return;
            }
            if k == "prompt" || k == "session-start" {
                state::save(&i).unwrap();
                println!("{{\"continue\":true}}")
            } else {
                match git_checkpoint::run(&i) {
                    Ok(_) => {
                        if k == "precompact" {
                            println!("{{\"continue\":false,\"stopReason\":\"Context compaction dodged.\",\"systemMessage\":\"Context compaction dodged.\"}}")
                        } else {
                            println!("{{\"continue\":true}}")
                        }
                    }
                    Err(e) => println!("{{\"continue\":false,\"stopReason\":{:?}}}", e.to_string()),
                }
            }
        }
        Some("supervisor") => {
            let mut c = Command::new(home().join("packages/standalone/current/bin/codex"));
            c.args(a)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            std::process::exit(c.status().unwrap().code().unwrap_or(1))
        }
        _ => {}
    }
}
