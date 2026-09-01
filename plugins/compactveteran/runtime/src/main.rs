mod atomic;
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
fn digest(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    format!("{:x}", h.finalize())
}
fn main() {
    let mut a = env::args().skip(1);
    match a.next().as_deref() {
        Some("refresh-catalog") => run(|| catalog::refresh().map(|p| println!("{}", p.display()))),
        Some("install-config") => run(config::install),
        Some("restore-config") => run(config::restore),
        Some("config-status") => run(config::status),
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

fn run(f: impl FnOnce() -> io::Result<()>) {
    if let Err(e) = f() {
        eprintln!("compactveteran: {e}");
        std::process::exit(1);
    }
}
