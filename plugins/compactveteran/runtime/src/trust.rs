use crate::{catalog, config, home};
use serde_json::{json, Value};
use std::{
    env, fs,
    io::{self, BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};
fn stock() -> io::Result<PathBuf> {
    let p = fs::canonicalize(home().join("packages/standalone/current/bin/codex"))?;
    if p == fs::canonicalize(env::current_exe()?)? {
        return Err(io::Error::other("stock codex resolves to CompactVeteran"));
    }
    Ok(p)
}
fn rpc(trusted: Option<bool>) -> io::Result<Vec<(String, String, bool)>> {
    let mut c = Command::new(stock()?)
        .args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut i = c.stdin.take().ok_or_else(|| io::Error::other("stdin"))?;
    let stdout = c.stdout.take().ok_or_else(|| io::Error::other("stdout"))?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(Err(io::Error::other("app-server closed")));
                    break;
                }
                Ok(_) => match serde_json::from_str::<Value>(&line) {
                    Ok(v) => {
                        if tx.send(Ok(v)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(io::Error::other(e)));
                        break;
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            }
        }
    });
    let send = |i: &mut std::process::ChildStdin, v: Value| -> io::Result<()> {
        serde_json::to_writer(&mut *i, &v).map_err(io::Error::other)?;
        i.write_all(b"\n")?;
        i.flush()
    };
    let read = |id: u64| -> io::Result<Value> {
        loop {
            let v = rx
                .recv_timeout(Duration::from_secs(10))
                .map_err(|e| io::Error::other(format!("app-server response: {e}")))??;
            if v["id"].as_u64() == Some(id) {
                if let Some(error) = v.get("error") {
                    return Err(io::Error::other(error.to_string()));
                }
                if v.get("result").is_none() {
                    return Err(io::Error::other("response missing result"));
                }
                return Ok(v);
            }
        }
    };
    send(
        &mut i,
        json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"compactveteran","version":"0.1.0"}}}),
    )?;
    read(1)?;
    send(&mut i, json!({"method":"initialized","params":{}}))?;
    send(
        &mut i,
        json!({"id":2,"method":"hooks/list","params":{"cwds":[env::current_dir()?.display().to_string()]}}),
    )?;
    let v = read(2)?;
    let hs = v["result"]["data"][0]["hooks"]
        .as_array()
        .ok_or_else(|| io::Error::other("no hooks"))?;
    let mut edits = Vec::new();
    let mut out = Vec::new();
    for n in ["UserPromptSubmit", "Stop", "PreCompact", "SessionStart"] {
        let h = hs
            .iter()
            .find(|h| h["pluginId"] == "compactveteran@compactveteran" && h["eventName"] == n)
            .ok_or_else(|| io::Error::other(format!("missing {n}")))?;
        let k = h["key"]
            .as_str()
            .ok_or_else(|| io::Error::other("key"))?
            .to_string();
        let hash = h["currentHash"]
            .as_str()
            .ok_or_else(|| io::Error::other("hash"))?
            .to_string();
        out.push((
            k.clone(),
            hash.clone(),
            h["enabled"].as_bool().unwrap_or(false) && h["trustedHash"] == h["currentHash"],
        ));
        edits.push(if trusted == Some(true){json!({"keyPath":format!("hooks.state.\"{k}\""),"value":{"enabled":true,"trusted_hash":hash},"mergeStrategy":"upsert"})}else{json!({"keyPath":format!("hooks.state.\"{k}\""),"value":null,"mergeStrategy":"replace"})});
    }
    if trusted.is_some() {
        send(
            &mut i,
            json!({"id":3,"method":"config/batchWrite","params":{"edits":edits,"reloadUserConfig":true}}),
        )?;
        read(3)?;
    }
    let _ = c.kill();
    let _ = c.wait();
    Ok(out)
}
pub fn set(t: bool) -> io::Result<()> {
    rpc(Some(t)).map(|_| ())
}
pub fn doctor() -> io::Result<()> {
    let s = stock()?;
    let e = env::current_exe()?;
    let maps = home().join("project-maps");
    fs::create_dir_all(&maps)?;
    let d = env::var_os("PLUGIN_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("plugins/data/compactveteran-compactveteran"));
    let checks = [
        (
            "platform",
            cfg!(target_os = "linux") && cfg!(target_arch = "x86_64"),
        ),
        ("stock", s != fs::canonicalize(&e)?),
        ("binary", e.metadata()?.permissions().mode() & 0o111 != 0),
        ("data", d.is_dir()),
        ("maps", maps.is_dir()),
        (
            "hooks",
            rpc(None)
                .map(|x| x.len() == 4 && x.iter().all(|z| z.2))
                .unwrap_or(false),
        ),
        ("catalog", catalog::refresh().is_ok()),
        ("config", config::is_owned().unwrap_or(false)),
        (
            "launcher",
            env::var_os("COMPACTVETERAN_EXPECT_LAUNCHER").is_some()
                || fs::read_to_string(home().join("../.local/bin/codex"))
                    .map(|x| x.contains("compactveteran"))
                    .unwrap_or(false),
        ),
    ];
    for (n, ok) in checks {
        println!("{n:10} {}", if ok { "ok" } else { "fail" });
        if !ok {
            return Err(io::Error::other(format!("doctor: {n}")));
        }
    }
    Ok(())
}
