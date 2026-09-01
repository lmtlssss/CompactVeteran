use crate::home;
use std::{
    env, fs, io,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};

fn stock() -> io::Result<PathBuf> {
    let p = fs::canonicalize(home().join("packages/standalone/current/bin/codex"))?;
    if p == fs::canonicalize(env::current_exe()?)? {
        return Err(io::Error::other("stock codex resolves to CompactVeteran"));
    }
    Ok(p)
}
pub fn set(_trusted: bool) -> io::Result<()> {
    let p = stock()?;
    let mut c = Command::new(p);
    c.args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = c.spawn()?;
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("app-server stdin"))?;
    use std::io::Write;
    writeln!(
        input,
        "{{\"id\":1,\"method\":\"initialize\",\"params\":{{}}}}"
    )?;
    writeln!(input, "{{\"method\":\"initialized\"}}")?;
    writeln!(
        input,
        "{{\"id\":2,\"method\":\"hooks/list\",\"params\":{{\"cwd\":{}}}}}",
        serde_json::to_string(&env::current_dir()?.display().to_string()).unwrap()
    )?;
    input.flush()?;
    drop(input);
    let _ = child.wait();
    Ok(())
}
pub fn doctor() -> io::Result<()> {
    let stock = stock()?;
    let exe = env::current_exe()?;
    let data = env::var_os("PLUGIN_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("plugins/data/compactveteran-compactveteran"));
    let checks = [
        (
            "platform",
            cfg!(target_os = "linux") && cfg!(target_arch = "x86_64"),
        ),
        ("stock", stock != fs::canonicalize(&exe)?),
        ("binary", exe.metadata()?.permissions().mode() & 0o111 != 0),
        ("data", data.is_dir()),
        (
            "maps",
            fs::create_dir_all(home().join("project-maps")).is_ok(),
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
