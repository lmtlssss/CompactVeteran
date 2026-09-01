#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"; bin="$root/plugins/compactveteran/runtime/target/debug/compactveteran"
cargo build --manifest-path "$root/plugins/compactveteran/runtime/Cargo.toml" --quiet
test -f "$root/.agents/plugins/marketplace.json"; test -f "$root/plugins/compactveteran/hooks/hooks.json"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT; export HOME="$tmp" CODEX_HOME="$tmp/.codex"; mkdir -p "$CODEX_HOME"; printf '{"models":[{"slug":"gpt-5.6-sol","context_window":100,"auto_compact_token_limit":90},{"slug":"terra","context_window":7}]}' > "$CODEX_HOME/models_cache.json"
test "$($bin overlay)" = "$CODEX_HOME/compactveteran-models.json"; grep -q 1050000 "$CODEX_HOME/compactveteran-models.json"
mkdir "$tmp/repo"; cd "$tmp/repo"; git init -q -b main; git config user.email test@example.com; git config user.name test; echo x > file; "$bin" checkpoint; test -z "$(git status --porcelain)"; test -n "$(find "$CODEX_HOME/project-maps" -name '*.md' -print -quit)"; echo "complete system proof: PASS"
