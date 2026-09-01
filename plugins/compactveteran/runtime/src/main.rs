use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{env, fs, io::{self, Read}, path::{Path, PathBuf}, process::Command};

const SOL: &str = "gpt-5.6-sol";
fn home() -> PathBuf { env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".codex"))).expect("CODEX_HOME") }
fn root() -> PathBuf { env::current_dir().expect("cwd") }
fn map_path(r: &Path) -> PathBuf { let mut h=Sha256::new(); h.update(r.to_string_lossy().as_bytes()); home().join("project-maps").join(format!("{:x}.md", h.finalize())) }
fn git(args: &[&str]) -> String { String::from_utf8(Command::new("git").args(args).output().expect("git").stdout).unwrap_or_default().trim().into() }
fn checkpoint() -> io::Result<()> {
    let r=root(); if !r.join(".git").exists() { Command::new("git").args(["init","-b","main"]).current_dir(&r).status()?; }
    let status=git(&["status","--porcelain"]); if status.is_empty() { return Ok(()) }
    for line in status.lines() { let p=line.get(3..).unwrap_or(""); if [".env","credentials","secrets"].iter().any(|x| p.to_ascii_lowercase().contains(x)) { return Err(io::Error::new(io::ErrorKind::PermissionDenied,"credential filename guard")); } }
    Command::new("git").args(["add","-A"]).current_dir(&r).status()?;
    if !Command::new("git").args(["diff","--cached","--check"]).current_dir(&r).status()?.success() { return Err(io::Error::other("cached whitespace")); }
    Command::new("git").args(["commit","-m","compactveteran: checkpoint"]).current_dir(&r).status()?;
    let upstream=git(&["rev-parse","--abbrev-ref","--symbolic-full-name","@{u}"]); if !upstream.is_empty() { Command::new("git").args(["push"]).current_dir(&r).status()?; }
    if !git(&["status","--porcelain"]).is_empty() { return Err(io::Error::other("working tree not clean")); }
    write_map(&r)
}
fn write_map(r:&Path)->io::Result<()> { let map=map_path(r); fs::create_dir_all(map.parent().unwrap())?; let head=git(&["rev-parse","HEAD"]); let remote=git(&["remote","get-url","origin"]); let branch=git(&["branch","--show-current"]); let text=format!("# CompactVeteran\n\n- root: {}\n- remote: {}\n- branch: {}\n- HEAD: {}\n- status: clean\n- transcript: {}\n- transcript_sha256: unavailable\n- session_lineage: {}\n- latest_user_directive: {}\n- recent_commits:\n{}\n- roadmap: inspect the repository and referenced raw logs\n\n## resume\nRead this map, inspect Git and raw logs, then continue from HEAD.\n",r.display(),remote,branch,head,env::var("CODEX_ROLLOUT_PATH").unwrap_or_default(),env::var("CODEX_THREAD_ID").unwrap_or_default(),env::var("COMPACTVETERAN_LATEST_PROMPT").unwrap_or_default(),git(&["log","-5","--oneline"])); let tmp=map.with_extension("md.tmp"); fs::write(&tmp,text)?; fs::rename(tmp,map) }
fn overlay()->io::Result<PathBuf>{ let h=home(); let src=h.join("models_cache.json"); let v:Value=serde_json::from_str(&fs::read_to_string(&src)?)?; let mut out=v.clone(); if let Some(models)=out.get_mut("models").and_then(Value::as_array_mut){ for m in models { if m.get("slug").and_then(Value::as_str)==Some(SOL) { if let Some(o)=m.as_object_mut(){o.insert("context_window".into(),json!(1_050_000));o.insert("max_context_window".into(),json!(1_050_000));o.insert("auto_compact_token_limit".into(),json!(950_000));} } } } let p=h.join("compactveteran-models.json"); fs::write(&p,serde_json::to_vec(&out)?)?; Ok(p) }
fn main(){ let cmd=env::args().nth(1).unwrap_or_default(); if cmd=="supervisor" { let _=overlay(); let stock=home().join("packages/standalone/current/bin/codex"); let status=Command::new(stock).args(env::args().skip(2)).status().expect("stock Codex"); std::process::exit(status.code().unwrap_or(1)); } let mut input=String::new(); io::stdin().read_to_string(&mut input).ok(); let event:Value=serde_json::from_str(&input).unwrap_or_default(); let model=event.get("model").and_then(Value::as_str).unwrap_or(""); if model!=SOL && cmd!="overlay" { println!("{}",json!({"continue":true})); return } let result=match cmd.as_str(){"overlay"=>overlay().map(|p|println!("{}",p.display())),"checkpoint"=>checkpoint().map(|_|println!("{}",json!({"continue":true}))),"precompact"=>checkpoint().map(|_|println!("{}",json!({"continue":false,"stopReason":"Context compaction dodged.","systemMessage":"Context compaction dodged."}))),"prompt"=>{env::set_var("COMPACTVETERAN_LATEST_PROMPT",event.get("prompt").and_then(Value::as_str).unwrap_or(""));Ok(())},_=>Ok(())}; if let Err(e)=result{eprintln!("CompactVeteran: {e}");std::process::exit(1)} }
