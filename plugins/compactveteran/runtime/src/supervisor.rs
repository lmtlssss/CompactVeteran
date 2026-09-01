use crate::{
    config,
    control::{Listener, RestartRequest},
    home,
};
use std::{
    env,
    ffi::OsString,
    io::{self, Read},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicI32, Ordering},
        Once,
    },
};
static SIGNAL: AtomicI32 = AtomicI32::new(0);
static INIT: Once = Once::new();
extern "C" fn signal_handler(sig: i32) {
    SIGNAL.store(sig, Ordering::SeqCst);
}
fn signals() {
    INIT.call_once(|| unsafe {
        libc::signal(libc::SIGINT, signal_handler as *const () as usize);
        libc::signal(libc::SIGTERM, signal_handler as *const () as usize);
    });
}
fn stock() -> io::Result<PathBuf> {
    let p = home().join("packages/standalone/current/bin/codex");
    let p = std::fs::canonicalize(p)?;
    let meta = std::fs::metadata(&p)?;
    if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
        return Err(io::Error::other("stock codex is not executable"));
    }
    if p == std::fs::canonicalize(env::current_exe()?)? {
        return Err(io::Error::other("codex supervisor recursion"));
    }
    Ok(p)
}
use std::os::unix::fs::PermissionsExt;
fn launch(
    bin: &PathBuf,
    args: &[OsString],
    req: Option<&RestartRequest>,
    sock: &str,
) -> io::Result<Child> {
    let mut c = Command::new(bin);
    if let Some(r) = req {
        c.args([OsString::from("-C"),r.cwd.clone().into(),OsString::from("--model"),r.model.clone().into(),OsString::from(format!("Read {}. Treat it as a map, inspect Git and the referenced raw logs, and continue the unfinished work from HEAD.",r.map))]);
    } else {
        c.args(args);
    }
    c.env("COMPACTVETERAN_SOCKET", sock)
        .env("COMPACTVETERAN_SUPERVISED", "1")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
}
pub fn run(args: Vec<OsString>, initial: Option<RestartRequest>) -> io::Result<i32> {
    signals();
    config::install()?;
    let pid = std::process::id();
    let path = state_path().join("run").join(format!("{pid}.sock"));
    let listener = Listener::bind(path.clone())?;
    let sock = path.to_string_lossy().to_string();
    let mut req = initial;
    loop {
        config::install()?;
        let bin = stock()?;
        let mut child = launch(&bin, &args, req.as_ref(), &sock)?;
        loop {
            if let Some(sig) = pending() {
                unsafe {
                    libc::kill(child.id() as i32, sig);
                }
            }
            if let Some(status) = child.try_wait()? {
                if pending().is_none() {
                    return Ok(status.code().unwrap_or(1));
                }
            }
            match listener.socket.accept() {
                Ok((mut s, _)) => {
                    let mut b = String::new();
                    s.read_to_string(&mut b)?;
                    if let Ok(r) = serde_json::from_str::<RestartRequest>(&b) {
                        std::thread::sleep(std::time::Duration::from_millis(750));
                        terminate(&mut child)?;
                        req = Some(r);
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(50))
                }
                Err(e) => return Err(e),
            }
        }
    }
}
fn terminate(c: &mut Child) -> io::Result<()> {
    unsafe { libc::kill(c.id() as i32, libc::SIGTERM) };
    for _ in 0..60 {
        if c.try_wait()?.is_some() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    c.kill()?;
    c.wait().map(|_| ())
}
fn pending() -> Option<i32> {
    match SIGNAL.swap(0, Ordering::SeqCst) {
        0 => None,
        x => Some(x),
    }
}
fn state_path() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("compactveteran")
}
