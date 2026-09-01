import hashlib
import json
import os
import pathlib
import subprocess
import sys
import tomllib


def env(name):
    value = os.environ.get(name, "").strip()
    assert value, f"missing environment variable {name}"
    return pathlib.Path(value)


def read(path):
    return json.loads(path.read_text())


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args], text=True).strip()


def git_dir(path, *args):
    return subprocess.check_output(["git", "--git-dir", str(path), *args], text=True).strip()


def check_installed(home, codex, state, xdg, repo):
    launcher = home / ".local/bin/codex"
    assert launcher.is_file() and launcher.stat().st_mode & 0o111, "launcher missing or not executable"
    assert "compactveteran" in launcher.read_text(), "launcher is not CompactVeteran"
    s = read(state / "state.json")
    assert s.get("marketplace") is True and s.get("plugin") is True, "plugin state is not installed"
    hooks = read(state / "hooks.json")
    events = ["UserPromptSubmit", "Stop", "PreCompact", "SessionStart"]
    assert set(hooks) == set(events), "hook event set changed"
    for event in events:
        h = hooks[event]
        assert h.get("enabled") is True and h.get("trusted_hash") == "hash-" + event.lower(), f"hook {event} is not trusted"
    config = tomllib.loads((codex / "config.toml").read_text())
    assert config.get("model_catalog_json") == str(xdg / "compactveteran/models-overlay.json"), "catalog ownership wrong"
    assert "model_context_window" not in config and "model_auto_compact_token_limit" not in config, "global context override leaked"
    assert config.get("unrelated") == "keep", "unrelated config changed"
    assert (xdg / "compactveteran/config-ownership.toml").is_file(), "ownership file missing"
    overlay = xdg / "compactveteran/models-overlay.json"
    assert overlay.is_file(), "model overlay missing"
    original = read(codex / "models_cache.json")
    patched = read(overlay)
    assert patched.keys() == original.keys(), "catalog root changed"
    assert [m["slug"] for m in patched["models"]].count("gpt-5.6-sol") == 1 and [m["slug"] for m in patched["models"]].count("gpt-5.6-terra") == 1 and [m["slug"] for m in patched["models"]].count("gpt-5.6-luna") == 1 and [m["slug"] for m in patched["models"]].count("other") == 1, "catalog model set changed"
    for got, want in zip(patched["models"], original["models"]):
        if got["slug"] == "gpt-5.6-sol":
            assert got.get("context_window") == 1050000 and got.get("max_context_window") == 1050000 and got.get("auto_compact_token_limit") == 950000, "Sol catalog values wrong"
        else:
            assert got == want, f"non-Sol model changed: {got.get('slug')}"
    for key in patched:
        if key != "models":
            assert patched[key] == original[key], f"catalog root key changed: {key}"


def check_lifecycle(codex, state, xdg, repo, v2, remote, proof):
    assert read(state / "session-start-1.json").get("continue") is True and read(state / "session-start-2.json").get("continue") is True, "Sol SessionStart failed"
    for name in ("terra-bypass", "luna-bypass"):
        assert read(state / f"{name}.json") == {"continue": True}, f"{name} did not bypass"
    assert not (xdg / "compactveteran/sessions/gpt-5.6-terra-session.json").exists() and not (xdg / "compactveteran/sessions/gpt-5.6-luna-session.json").exists(), "non-Sol state created"
    rows = [json.loads(x) for x in (state / "invocations.jsonl").read_text().splitlines()]
    assert [x["count"] for x in rows] == [1, 2, 3] and [x["version"] for x in rows] == ["v1", "v1", "v2"], "invocation sequence changed"
    assert len({x["pid"] for x in rows}) == 3 and all(x["pid"] > 0 for x in rows), "process lineage invalid"
    canonical = str(repo.resolve())
    assert all(x["cwd"] == canonical for x in rows), "invocation cwd is not canonical"
    h = hashlib.sha256(canonical.encode()).hexdigest()
    map_path = codex / "project-maps" / (h + ".md")
    maps = list((codex / "project-maps").glob("*.md"))
    assert len(maps) == 1 and maps[0] == map_path, "project map count/path wrong"
    for n, row in enumerate(rows[1:3], 2):
        assert row["argv"] == ["-C", canonical, "--model", "gpt-5.6-sol", f"Read {map_path}. Use its Objective, Cursor, and Next action. Continue immediately from local HEAD. Open a referenced raw log only if a specific ambiguity blocks the next action."], f"invocation {n} handoff argv wrong"
        joined = " ".join(row["argv"]).lower()
        for bad in ("resume", "session-1", "session-2", "build the first checkpoint", "continue the second checkpoint"):
            assert bad not in joined, f"forbidden handoff text: {bad}"
    for n in (1, 2):
        assert read(state / f"prompt-{n}.json").get("continue") is True, f"prompt {n} did not continue"
        assert read(state / f"stop-{n}.json").get("continue") is True, f"stop {n} did not continue"
        p = read(state / f"precompact-{n}.json")
        assert p.get("continue") is False and p.get("stopReason") == "Context compaction dodged." and p.get("systemMessage") == "Context compaction dodged.", f"precompact {n} result wrong"
        assert read(state / f"precompact-{n}.payload.json").get("trigger") == ("manual" if n == 1 else "auto"), f"precompact {n} trigger wrong"
    for p in state.rglob("*"):
        assert "compacted" not in p.name
        if p.is_file():
            try: assert "compacted_history" not in p.read_text() and "generated summary" not in p.read_text()
            except UnicodeDecodeError: pass
    assert not git(repo, "status", "--porcelain") and all(git(repo, "ls-files", x) == x for x in ("work-one.txt", "work-two.txt")), "repository is not clean/tracked"
    assert git(repo, "rev-parse", "HEAD") != git_dir(remote, "rev-parse", "refs/heads/main"), "local HEAD did not diverge"
    assert "compactveteran: checkpoint" not in git_dir(remote, "log", "--format=%s", "refs/heads/main"), "remote received checkpoint"
    assert sum(x.startswith("compactveteran: checkpoint ") for x in git(repo, "log", "--format=%s").splitlines()) == 2, "checkpoint count wrong"
    text = map_path.read_text()
    assert text.startswith("# CompactVeteran handoff\n") and f"- canonical root: {canonical}" in text and f"- HEAD: {git(repo, 'rev-parse', 'HEAD')}" in text and "- branch: main" in text and "- clean: true" in text and "- upstream:" not in text and "- remote:" not in text, "map Git state wrong"
    t2 = (state / "transcript2.jsonl").resolve()
    assert str(t2) in text and hashlib.sha256(t2.read_bytes()).hexdigest() in text and "build the first checkpoint" in text and "second result complete; next action: report the proof" in text, "map transcript/cursor wrong"
    assert "session-1\t" + str((state / "transcript1.jsonl").resolve()) in text and "session-2\t" + str(t2) in text and "compactveteran: checkpoint" in text and "- " + str(repo / "README.md") in text and "- " + str(repo / "AGENTS.md") in text and "- " + str(repo / "ROADMAP.md") in text and "Continue the Objective from the Cursor at local HEAD. Use listed project sources only as needed. Open the raw transcript only for a specific unresolved ambiguity." in text and len(text.encode()) <= 16384 and "offline.git" not in text and "upstream:" not in text and "remote:" not in text, "map contents incomplete"
    assert len(text.split("## Recent commits\n\n```text\n", 1)[1].split("\n```", 1)[0].splitlines()) <= 5 and len(text.split("### Session lineage\n\n", 1)[1].splitlines()) <= 3, "map sections exceed bounds"
    runtime = xdg / "compactveteran"
    for p in (runtime / "sessions/session-1.json", runtime / "sessions/session-2.json", runtime / "projects" / (h + ".json")):
        assert p.is_file(), f"state file missing: {p.name}"
    project = read(runtime / "projects" / (h + ".json"))
    assert project.get("canonical_root") == canonical and project.get("objective") == "build the first checkpoint" and project.get("last_assistant_result") == "second result complete; next action: report the proof" and [(x.get("session_id"), x.get("transcript_path")) for x in project.get("sessions", [])] == [("session-1", str((state / "transcript1.jsonl").resolve())), ("session-2", str(t2))], "project lineage wrong"
    for n in (1, 2):
        session = read(runtime / "sessions" / f"session-{n}.json")
        assert session.get("session_id") == f"session-{n}" and session.get("latest_prompt") == ("build the first checkpoint" if n == 1 else None) and session.get("transcript_path") == str((state / f"transcript{n}.jsonl").resolve()), f"session {n} state wrong"
    assert pathlib.Path(codex / "packages/standalone/current").resolve() == v2.resolve(), "current release did not switch"
    proof.mkdir(parents=True, exist_ok=True)
    (proof / "map-path").write_text(str(map_path))
    (proof / "map-sha").write_text(hashlib.sha256(map_path.read_bytes()).hexdigest())


def check_uninstalled(home, codex, state, xdg, proof, v2):
    s = read(state / "state.json") if (state / "state.json").exists() else {}
    assert not s.get("marketplace") and not s.get("plugin"), "plugin state remains installed"
    hooks = read(state / "hooks.json") if (state / "hooks.json").exists() else {}
    assert all(not (v.get("enabled") is True or "trusted_hash" in v) for v in hooks.values()), "trusted hooks remain"
    config = tomllib.loads((codex / "config.toml").read_text())
    assert config == {"model_catalog_json": "/prior/catalog.json", "model_context_window": 333333, "model_auto_compact_token_limit": 222222, "unrelated": "keep", "after_install": "preserve"}, "config was not restored"
    launcher = home / ".local/bin/codex"
    assert launcher.is_symlink() and launcher.readlink() == pathlib.Path(proof.joinpath("original-target").read_text().strip()), "launcher target not restored"
    assert launcher.resolve() == (v2 / "bin/codex").resolve(), "restored launcher does not reach v2"
    assert not (codex / "plugins/data/compactveteran-compactveteran").exists() and not (xdg / "compactveteran").exists(), "plugin data remains"
    map_path = pathlib.Path((proof / "map-path").read_text())
    assert map_path.exists() and hashlib.sha256(map_path.read_bytes()).hexdigest() == (proof / "map-sha").read_text(), "map was changed"
    assert len(list((codex / "project-maps").glob("*.md"))) == 1, "map count changed"


def main():
    try:
        phase = sys.argv[1]
        home, codex, state, xdg = env("HOME"), env("CODEX_HOME"), env("FIXTURE_STATE"), env("XDG_STATE_HOME")
        repo, v2, remote, proof = env("FIXTURE_REPO"), env("FIXTURE_V2_TARGET"), env("PROOF_REMOTE"), env("PROOF_TMP")
        if phase == "installed": check_installed(home, codex, state, xdg, repo)
        elif phase == "lifecycle": check_lifecycle(codex, state, xdg, repo, v2, remote, proof)
        elif phase == "uninstalled": check_uninstalled(home, codex, state, xdg, proof, v2)
        else: raise AssertionError("phase must be installed, lifecycle, or uninstalled")
    except (AssertionError, KeyError, IndexError, OSError, ValueError, tomllib.TOMLDecodeError, subprocess.CalledProcessError) as exc:
        print(str(exc), file=sys.stderr)
        return 1
    return 0


raise SystemExit(main())
