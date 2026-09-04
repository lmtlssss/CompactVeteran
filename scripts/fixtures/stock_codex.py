#!/usr/bin/env python3
"""Small deterministic stock-Codex process used by prove-system.sh."""
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path


def env(name):
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"missing {name}")
    return Path(value)


STATE = env("FIXTURE_STATE")
PLUGIN_ROOT = env("FIXTURE_PLUGIN_ROOT")
PLUGIN_BIN = env("FIXTURE_PLUGIN_BIN")
REPO = env("FIXTURE_REPO")
CURRENT = env("FIXTURE_CURRENT_LINK")
V2 = env("FIXTURE_V2_TARGET")


def atomic(path, value, binary=False):
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    if binary:
        tmp.write_bytes(value)
    else:
        tmp.write_text(value)
    os.replace(tmp, path)


def read_json(name, default):
    path = STATE / name
    try:
        return json.loads(path.read_text())
    except FileNotFoundError:
        return default


def write_json(name, value):
    atomic(STATE / name, json.dumps(value, indent=2, sort_keys=True) + "\n")


def plugin(command):
    state = read_json("state.json", {})
    if command[:2] == ["marketplace", "add"]:
        state.update(marketplace=True, marketplaceName="compactveteran")
    elif command == ["marketplace", "list"]:
        if state.get("marketplace"):
            print("compactveteran")
        return
    elif command[:2] == ["marketplace", "upgrade"]:
        if not state.get("marketplace"):
            raise SystemExit("marketplace not installed")
    elif command[:2] == ["marketplace", "remove"]:
        state.pop("marketplace", None)
        state.pop("marketplaceName", None)
    elif command[:2] == ["add", "compactveteran@compactveteran"]:
        if not state.get("marketplace"):
            raise SystemExit("marketplace not installed")
        state["plugin"] = True
        print(json.dumps({"installedPath": str(PLUGIN_ROOT)}, indent=2))
    elif command[:2] == ["remove", "compactveteran@compactveteran"]:
        state.pop("plugin", None)
    write_json("state.json", state)


def app_server():
    hook_state = read_json("hooks.json", {})
    initialized = False
    pending_initialize = None
    for line in sys.stdin:
        request = json.loads(line)
        method, ident = request.get("method"), request.get("id")
        if method == "initialized":
            initialized = True
            if pending_initialize is not None:
                sys.stdout.write(json.dumps(pending_initialize, separators=(",", ":")) + "\n")
                sys.stdout.flush()
                pending_initialize = None
            continue
        if method == "initialize":
            response = {"jsonrpc": "2.0", "id": ident, "result": {"protocolVersion": "1", "serverInfo": {"name": "compactveteran", "version": "1"}}}
            if not initialized:
                pending_initialize = response
                continue
        elif method == "hooks/list":
            cwds = request.get("params", {}).get("cwds", [str(REPO)])
            cwd = cwds[0] if cwds else str(REPO)
            events = [("userPromptSubmit", "UserPromptSubmit"), ("stop", "Stop"), ("preCompact", "PreCompact"), ("sessionStart", "SessionStart")]
            hooks = [{"eventName": wire, "pluginId": "compactveteran@compactveteran", "key": "cv-" + key.lower(), "currentHash": "hash-" + key.lower(), "enabled": hook_state.get(key, {}).get("enabled", False), "trustedHash": None, "trustStatus": "trusted" if hook_state.get(key, {}).get("trusted_hash") == "hash-" + key.lower() else "untrusted"} for wire, key in events]
            response = {"jsonrpc": "2.0", "id": ident, "result": {"data": [{"cwd": cwd, "hooks": hooks}]}}
        elif method == "config/batchWrite":
            for edit in request.get("params", {}).get("edits", []):
                key = edit.get("keyPath", "")
                value = edit.get("value", edit)
                event = next((e for e in ("UserPromptSubmit", "Stop", "PreCompact", "SessionStart") if e.lower() in key.lower()), None)
                if event:
                    if value is None:
                        hook_state.pop(event, None)
                        continue
                    hook_state.setdefault(event, {})
                    for field in ("enabled", "trusted_hash"):
                        if field in value:
                            if value[field] is None:
                                hook_state[event].pop(field, None)
                            else:
                                hook_state[event][field] = value[field]
            atomic(STATE / "hooks.json", json.dumps(hook_state, sort_keys=True, indent=2) + "\n")
            response = {"jsonrpc": "2.0", "id": ident, "result": {}}
        else:
            response = {"jsonrpc": "2.0", "id": ident, "error": {"code": -32601, "message": "method not found"}}
        sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
        sys.stdout.flush()


def invoke(kind, count, version, session, turn, transcript, prompt=None):
    payload = {"hook_event_name": {"prompt": "UserPromptSubmit", "stop": "Stop", "precompact": "PreCompact", "session-start": "SessionStart"}[kind], "model": "gpt-5.6-sol", "session_id": session, "turn_id": turn, "cwd": str(REPO), "transcript_path": str(transcript), "last_assistant_message": ("first result complete; next action: continue locally" if count == 1 else ("pending request answered once" if count == 3 else "second result complete; next action: report the proof")) if kind == "stop" else None}
    if prompt is not None:
        payload["prompt"] = prompt
    if kind == "precompact":
        payload["trigger"] = "manual" if count == 1 else "auto"
        payload["count"] = count
        atomic(STATE / f"precompact-{count}.payload.json", json.dumps(payload, indent=2) + "\n")
    result = subprocess.run([str(PLUGIN_BIN), "hook", kind], input=json.dumps(payload), text=True, capture_output=True, env=os.environ.copy())
    atomic(STATE / f"{kind}-{count}.stderr.log", result.stderr, binary=False)
    if result.returncode or not result.stdout.strip():
        raise SystemExit("hook failed")
    try:
        output = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise SystemExit("invalid hook output") from exc
    atomic(STATE / f"{kind}-{count}.json", json.dumps(output, indent=2) + "\n")


def invoke_model(model, name):
    payload = {"hook_event_name": "PreCompact", "model": model, "session_id": f"{model}-session", "turn_id": "bypass", "cwd": str(REPO), "transcript_path": str(STATE / f"{name}.jsonl"), "trigger": "manual", "count": 1}
    result = subprocess.run([str(PLUGIN_BIN), "hook", "precompact"], input=json.dumps(payload), text=True, capture_output=True, env=os.environ.copy())
    atomic(STATE / f"{name}.json", result.stdout)


def interactive():
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    inv = read_json("invocation-count.json", 0) + 1
    write_json("invocation-count.json", inv)
    version = os.environ.get("STOCK_VERSION", "v1")
    db = env("XDG_STATE_HOME") / "codex-runtime-state/logs_2.sqlite"
    if not db.exists():
        db.parent.mkdir(parents=True, exist_ok=True)
        db.write_bytes(b"fresh")
    with (STATE / "invocations.jsonl").open("a") as stream:
        stream.write(json.dumps({"count": inv, "pid": os.getpid(), "version": version, "argv": sys.argv[1:], "cwd": os.getcwd(), "codex_install_dir": os.environ.get("CODEX_INSTALL_DIR")}) + "\n")
    if inv == 3:
        if version != "v2": raise SystemExit("third invocation requires v2")
        transcript = STATE / "transcript3.jsonl"
        atomic(transcript, b'{"session_id":"session-3","turn_id":"turn-3"}\n', binary=True)
        payload = {"hook_event_name":"SessionStart","model":"gpt-5.6-sol","session_id":"session-3","cwd":str(REPO),"transcript_path":str(transcript),"source":"startup"}
        result = subprocess.run([str(PLUGIN_BIN), "hook", "session-start"], input=json.dumps(payload), text=True, capture_output=True, env=os.environ.copy())
        atomic(STATE / "session-start-3.json", json.dumps(json.loads(result.stdout), indent=2) + "\n")
        invoke("prompt", 3, version, "session-3", "turn-3", transcript, sys.argv[-1])
        invoke("stop", 3, version, "session-3", "turn-3", transcript)
        return
    if inv not in (1, 2) or version != "v1":
        raise SystemExit("unexpected invocation")
    transcript = STATE / f"transcript{inv}.jsonl"
    atomic(transcript, (f"{{\"session_id\":\"session-{inv}\",\"turn_id\":\"turn-{inv}\"}}\n").encode(), binary=True)
    session, turn = f"session-{inv}", f"turn-{inv}"
    session_payload = {"hook_event_name": "SessionStart", "model": "gpt-5.6-sol", "session_id": session, "cwd": str(REPO), "transcript_path": str(transcript), "source": "startup", "permission_mode": "default"}
    session_result = subprocess.run([str(PLUGIN_BIN), "hook", "session-start"], input=json.dumps(session_payload), text=True, capture_output=True, env=os.environ.copy())
    atomic(STATE / f"session-start-{inv}.json", json.dumps(json.loads(session_result.stdout), indent=2) + "\n")
    if inv == 1:
        invoke_model("gpt-5.6-terra", "terra-bypass")
        invoke_model("gpt-5.6-luna", "luna-bypass")
    invoke("prompt", inv, version, session, turn, transcript, "build the first checkpoint" if inv == 1 else sys.argv[-1])
    if inv == 2:
        atomic(REPO / "work-two.txt", "checkpoint 2\n")
        invoke("stop", inv, version, session, turn, transcript)
        invoke("prompt", inv, version, session, "turn-2-real", transcript, "pending request must survive compaction")
        invoke("precompact", inv, version, session, "turn-2-real", transcript)
        CURRENT.unlink(missing_ok=True)
        CURRENT.symlink_to(V2)
        stock_bin = Path(os.environ["CODEX_INSTALL_DIR"])
        stock_bin.mkdir(parents=True, exist_ok=True)
        visible = stock_bin / "codex"
        visible.unlink(missing_ok=True)
        visible.symlink_to(CURRENT / "bin/codex")
        while True: time.sleep(1)
    atomic(REPO / ("work-one.txt" if inv == 1 else "work-two.txt"), f"checkpoint {inv}\n")
    invoke("stop", inv, version, session, turn, transcript)
    invoke("precompact", inv, version, session, turn, transcript)
    if inv == 2:
        CURRENT.unlink(missing_ok=True)
        CURRENT.symlink_to(V2)
    while True:
        time.sleep(1)


if __name__ == "__main__":
    if sys.argv[1:2] == ["plugin"]:
        plugin(sys.argv[2:])
    elif sys.argv[1:3] == ["app-server", "--stdio"]:
        app_server()
    elif sys.argv[1:2] == ["--version"]:
        db = env("XDG_STATE_HOME") / "codex-runtime-state/logs_2.sqlite"
        if not db.exists():
            db.parent.mkdir(parents=True, exist_ok=True)
            db.write_bytes(b"fresh")
        print("codex fixture")
    else:
        interactive()
