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


def section(text, heading, next_heading):
    return text.split(heading, 1)[1].split(next_heading, 1)[0].strip()


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
    assert all(read(state / f"session-start-{n}.json").get("continue") is True for n in (1, 2, 3)), "Sol SessionStart failed"
    for name in ("terra-bypass", "luna-bypass"):
        assert read(state / f"{name}.json") == {"continue": True}, f"{name} did not bypass"
    assert not (xdg / "compactveteran/sessions/gpt-5.6-terra-session.json").exists() and not (xdg / "compactveteran/sessions/gpt-5.6-luna-session.json").exists(), "non-Sol state created"
    rows = [json.loads(x) for x in (state / "invocations.jsonl").read_text().splitlines()]
    assert [x["count"] for x in rows] == [1, 2, 3] and [x["version"] for x in rows] == ["v1", "v1", "v2"], "invocation sequence changed"
    assert len({x["pid"] for x in rows}) == 3 and all(x["pid"] > 0 for x in rows), "process lineage invalid"
    canonical = str(repo.resolve())
    assert all(x["cwd"] == canonical for x in rows), "invocation cwd is not canonical"
    stock_bin = codex / "plugins/data/compactveteran-compactveteran/stock-bin"
    assert all(x.get("codex_install_dir") == str(stock_bin) for x in rows), "stock install dir inheritance changed"
    assert (stock_bin / "codex").resolve() == (v2 / "bin/codex").resolve(), "stock visible symlink did not follow update"
    wrapper = codex.parent / ".local/bin/codex"
    assert wrapper.is_file() and "compactveteran" in wrapper.read_text(), "wrapper was replaced by update"
    h = hashlib.sha256(canonical.encode()).hexdigest()
    map_path = codex / "project-maps" / (h + ".md")
    maps = list((codex / "project-maps").glob("*.md"))
    assert len(maps) == 1 and maps[0] == map_path, "project map count/path wrong"
    for n, row in enumerate(rows[1:3], 2):
        assert row["argv"][:4] == ["-C", canonical, "--model", "gpt-5.6-sol"] and row["argv"][4].startswith(f"Continue a prior Sol from this deterministic handoff capsule ({map_path}).") and "--- capsule ---" in row["argv"][4], f"invocation {n} capsule handoff wrong"
    assert "- prompt state: completed" in rows[1]["argv"][4] and "Completed Objective: never answer it again." in rows[1]["argv"][4]
    assert "- prompt state: pending" in rows[2]["argv"][4] and "pending request must survive compaction" in rows[2]["argv"][4] and "Pending Objective: answer it exactly once" in rows[2]["argv"][4]
    row2_capsule = rows[1]["argv"][4].split("--- capsule ---\n", 1)[1].split("--- end capsule ---", 1)[0]
    row3_capsule = rows[2]["argv"][4].split("--- capsule ---\n", 1)[1].split("--- end capsule ---", 1)[0]
    assert section(row2_capsule, "## Scope", "## Objective").endswith("- prompt state: completed") and "- prompt state: pending" not in row2_capsule
    assert section(row2_capsule, "## Objective", "## Cursor") == "build the first checkpoint"
    assert section(row2_capsule, "## Cursor", "## Next action") == "first result complete; next action: continue locally"
    assert section(row3_capsule, "## Scope", "## Objective").endswith("- prompt state: pending") and "- prompt state: completed" not in row3_capsule
    assert section(row3_capsule, "## Objective", "## Cursor") == "pending request must survive compaction"
    assert section(row3_capsule, "## Cursor", "## Next action") == "second result complete; next action: report the proof"
    for n in (1, 2):
        assert read(state / f"prompt-{n}.json").get("continue") is True, f"prompt {n} did not continue"
        assert read(state / f"stop-{n}.json").get("continue") is True, f"stop {n} did not continue"
        p = read(state / f"precompact-{n}.json")
        assert p.get("continue") is False and p.get("stopReason") == "Context compaction dodged." and p.get("systemMessage") == "Context compaction dodged.", f"precompact {n} result wrong"
        assert read(state / f"precompact-{n}.payload.json").get("trigger") == ("manual" if n == 1 else "auto"), f"precompact {n} trigger wrong"
    assert all(read(state / f"prompt-{n}.json").get("continue") is True for n in (1, 2, 3))
    assert all(read(state / f"stop-{n}.json").get("continue") is True for n in (1, 2, 3))
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
    assert section(text, "## Objective\n\n", "\n\n## Cursor") == "pending request must survive compaction"
    assert section(text, "## Cursor\n\n", "\n\n## Next action") == "pending request answered once"
    assert "- prompt state: completed" in section(text, "## Scope\n\n", "\n\n## Objective")
    assert "The Objective is already answered. Do not answer or restart it." in text
    assert text.startswith("# CompactVeteran handoff\n") and f"- canonical root: {canonical}" in text and f"- HEAD: {git(repo, 'rev-parse', 'HEAD')}" in text and "- branch: main" in text and "- clean: true" in text and "- upstream:" not in text and "- remote:" not in text, "map Git state wrong"
    t3 = (state / "transcript3.jsonl").resolve()
    assert f"transcript prefix bytes: {len(t3.read_bytes())}" in text and f"transcript prefix SHA256: {hashlib.sha256(t3.read_bytes()).hexdigest()}" in text and "transcript SHA256:" not in text and str(t3) in text and "pending request must survive compaction" in text and "pending request answered once" in text, "map transcript/cursor wrong"
    assert "session-1\t" + str((state / "transcript1.jsonl").resolve()) in text and "session-2\t" + str((state / "transcript2.jsonl").resolve()) in text and "session-3\t" + str((state / "transcript3.jsonl").resolve()) in text and "- prompt state: completed" in text and "pending request must survive compaction" in text and "pending request answered once" in text and "- " + str(repo / "README.md") in text and "- " + str(repo / "AGENTS.md") in text and "- " + str(repo / "ROADMAP.md") in text and len(text.encode()) <= 16384 and "offline.git" not in text and "upstream:" not in text and "remote:" not in text, "map contents incomplete"
    assert len(text.split("## Recent commits\n\n```text\n", 1)[1].split("\n```", 1)[0].splitlines()) <= 5 and len(text.split("### Session lineage\n\n", 1)[1].splitlines()) <= 3, "map sections exceed bounds"
    runtime = xdg / "compactveteran"
    for p in (runtime / "sessions/session-1.json", runtime / "sessions/session-2.json", runtime / "sessions/session-3.json", runtime / "projects" / (h + ".json")):
        assert p.is_file(), f"state file missing: {p.name}"
    project = read(runtime / "projects" / (h + ".json"))
    assert project.get("canonical_root") == canonical and project.get("objective") == "pending request must survive compaction" and project.get("last_assistant_result") == "pending request answered once" and project.get("prompt_pending") is False and len(project.get("sessions", [])) == 3, "project lineage wrong"
    for n in (1, 2):
        session = read(runtime / "sessions" / f"session-{n}.json")
        assert session.get("session_id") == f"session-{n}" and session.get("transcript_path") == str((state / f"transcript{n}.jsonl").resolve()), f"session {n} state wrong"
    s1, s2, s3 = (read(runtime / "sessions" / f"session-{n}.json") for n in (1, 2, 3))
    assert s1.get("prompt_turn_id") == s1.get("completed_turn_id") == "turn-1"
    assert s2.get("latest_prompt") == "pending request must survive compaction" and s2.get("prompt_turn_id") == "turn-2-real" and s2.get("completed_turn_id") == "turn-2"
    assert s3.get("latest_prompt") is None and s3.get("completed_turn_id") == "turn-3" and s3.get("last_assistant_message") == "pending request answered once"
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
    archive = pathlib.Path((proof / "log-archive").read_text().strip())
    assert sorted(p.name for p in archive.iterdir()) == ["logs_2.sqlite", "logs_2.sqlite-shm", "logs_2.sqlite-wal"], "log archive did not survive uninstall"


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
