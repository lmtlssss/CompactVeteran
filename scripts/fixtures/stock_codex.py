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
    for line in sys.stdin:
        request = json.loads(line)
        method, ident = request.get("method"), request.get("id")
        if method == "initialized":
            continue
        if method == "initialize":
            response = {"jsonrpc": "2.0", "id": ident, "result": {"protocolVersion": "1", "serverInfo": {"name": "compactveteran", "version": "1"}}}
        elif method == "hooks/list":
            cwds = request.get("params", {}).get("cwds", [str(REPO)])
            cwd = cwds[0] if cwds else str(REPO)
            events = ["UserPromptSubmit", "Stop", "PreCompact", "SessionStart"]
            hooks = [{"eventName": e, "pluginId": "compactveteran@compactveteran", "key": "cv-" + e.lower(), "currentHash": "hash-" + e.lower(), "enabled": hook_state.get(e, {}).get("enabled", False), "trustedHash": hook_state.get(e, {}).get("trusted_hash", None)} for e in events]
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
    payload = {"hook_event_name": {"prompt": "UserPromptSubmit", "stop": "Stop", "precompact": "PreCompact", "session-start": "SessionStart"}[kind], "model": "gpt-5.6-sol", "session_id": session, "turn_id": turn, "cwd": str(REPO), "transcript_path": str(transcript)}
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


def invoke_model(kind, model, name):
    payload = {"hook_event_name": "PreCompact", "model": model, "session_id": f"{model}-session", "turn_id": "bypass", "cwd": str(REPO), "transcript_path": str(STATE / f"{name}.jsonl"), "trigger": "manual", "count": 1}
    result = subprocess.run([str(PLUGIN_BIN), "hook", "precompact"], input=json.dumps(payload), text=True, capture_output=True, env=os.environ.copy())
    atomic(STATE / f"{name}.json", result.stdout)


def interactive():
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    inv = read_json("invocation-count.json", 0) + 1
    write_json("invocation-count.json", inv)
    version = os.environ.get("STOCK_VERSION", "v1")
    with (STATE / "invocations.jsonl").open("a") as stream:
        stream.write(json.dumps({"count": inv, "pid": os.getpid(), "version": version, "argv": sys.argv[1:], "cwd": os.getcwd()}) + "\n")
    if inv == 3:
        if version != "v2": raise SystemExit("third invocation requires v2")
        return
    if inv not in (1, 2) or version != "v1":
        raise SystemExit("unexpected invocation")
    transcript = STATE / f"transcript{inv}.jsonl"
    atomic(transcript, (f"{{\"session_id\":\"session-{inv}\",\"turn_id\":\"turn-{inv}\"}}\n").encode(), binary=True)
    session, turn = f"session-{inv}", f"turn-{inv}"
    invoke("session-start", inv, version, session, turn, transcript)
    if inv == 1:
        invoke_model("precompact", "gpt-5.6-terra", "terra-bypass")
        invoke_model("precompact", "gpt-5.6-luna", "luna-bypass")
    invoke("prompt", inv, version, session, turn, transcript, f"build the first checkpoint" if inv == 1 else "continue the second checkpoint")
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
    else:
        interactive()
