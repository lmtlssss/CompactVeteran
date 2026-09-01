#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
fail(){ echo "FAIL: $*" >&2; exit 1; }; assert_eq(){ [[ "$1" == "$2" ]] || fail "$3"; }
cargo build --locked --manifest-path "$root/plugins/compactveteran/runtime/Cargo.toml" --quiet
for f in "$root/.agents/plugins/marketplace.json" "$root/plugins/compactveteran/plugin.json" "$root/plugins/compactveteran/hooks/hooks.json"; do python -m json.tool "$f" >/dev/null || fail "invalid json"; done
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
export HOME="$tmp/home" CODEX_HOME="$HOME/.codex" XDG_STATE_HOME="$HOME/.local/state" PATH="$HOME/.local/bin:$PATH"
export FIXTURE_STATE="$XDG_STATE_HOME/fixture" FIXTURE_PLUGIN_ROOT="$root/plugins/compactveteran" FIXTURE_PLUGIN_BIN="$root/plugins/compactveteran/runtime/target/debug/compactveteran" FIXTURE_REPO="$tmp/repo" FIXTURE_CURRENT_LINK="$CODEX_HOME/packages/standalone/current" FIXTURE_V2_TARGET="$CODEX_HOME/packages/standalone/releases/v2"
mkdir -p "$HOME/.local/bin" "$CODEX_HOME/packages/standalone/releases/v1/bin" "$FIXTURE_V2_TARGET/bin" "$FIXTURE_STATE" "$FIXTURE_REPO"
printf '%s\n' '{"client_version":"fixture","etag":"etag","fetched_at":"2026-09-01","models":[{"slug":"gpt-5.6-sol","context_window":333333,"auto_compact_token_limit":222222},{"slug":"gpt-5.6-terra","context_window":444444},{"slug":"gpt-5.6-luna","context_window":555555},{"slug":"other","context_window":666666}]}' >"$CODEX_HOME/models_cache.json"
for v in v1 v2; do printf '#!/usr/bin/env bash\nexec env STOCK_VERSION=%s python3 %q "$@"\n' "$v" "$root/scripts/fixtures/stock_codex.py" >"$CODEX_HOME/packages/standalone/releases/$v/bin/codex"; chmod +x "$CODEX_HOME/packages/standalone/releases/$v/bin/codex"; done
ln -s "$CODEX_HOME/packages/standalone/releases/v1" "$FIXTURE_CURRENT_LINK"; ln -s "$FIXTURE_CURRENT_LINK/bin/codex" "$HOME/.local/bin/codex"
git -C "$FIXTURE_REPO" init -q -b main; git -C "$FIXTURE_REPO" config user.email test@example.com; git -C "$FIXTURE_REPO" config user.name test; git init --bare -q "$tmp/remote.git"; git -C "$FIXTURE_REPO" remote add origin "$tmp/remote.git"
printf '# fixture\n' >"$FIXTURE_REPO/README.md"; printf '# rules\n' >"$FIXTURE_REPO/AGENTS.md"; printf '# roadmap\n' >"$FIXTURE_REPO/ROADMAP.md"; git -C "$FIXTURE_REPO" add .; git -C "$FIXTURE_REPO" commit -qm initial; git -C "$FIXTURE_REPO" push -qu origin main
printf '%s\n' '# preserve' 'model_catalog_json="/prior/catalog.json"' 'model_context_window=333333' 'model_auto_compact_token_limit=222222' 'unrelated="keep"' >"$CODEX_HOME/config.toml"
COMPACTVETERAN_REPO="$root" COMPACTVETERAN_BINARY="$FIXTURE_PLUGIN_BIN" bash "$root/install.sh" >"$tmp/install.out"; test "$(tail -n1 "$tmp/install.out")" = 'CompactVeteran installed. No context left behind.'
printf '%s\n' 'after_install="preserve"' >>"$CODEX_HOME/config.toml"
(cd "$FIXTURE_REPO" && timeout 30 codex)
test "$(wc -l <"$FIXTURE_STATE/invocations.jsonl")" -eq 3; test "$(jq -r .version "$FIXTURE_STATE/invocations.jsonl" | tr '\n' ' ')" = 'v1 v1 v2 '; test "$(jq -r .continue "$FIXTURE_STATE/precompact-1.json")" = false; test "$(jq -r .systemMessage "$FIXTURE_STATE/precompact-1.json")" = 'Context compaction dodged.'; test -z "$(git -C "$FIXTURE_REPO" status --porcelain)"
test "$(jq -r .trigger "$FIXTURE_STATE/precompact-1.payload.json")" = manual; test "$(jq -r .trigger "$FIXTURE_STATE/precompact-2.payload.json")" = auto; test "$(jq -r .continue "$FIXTURE_STATE/stop-1.json")" = true; test "$(jq -r .continue "$FIXTURE_STATE/stop-2.json")" = true
map=$(find "$CODEX_HOME/project-maps" -type f -name '*.md' -print -quit); test -n "$map"; cp "$map" "$tmp/map"; test -e "$FIXTURE_CURRENT_LINK/bin/codex"
bash "$root/uninstall.sh" >/dev/null; test ! -e "$FIXTURE_STATE/state.json"; test -e "$map"; cmp -s "$tmp/map" "$map"
echo 'CompactVeteran system proof: PASS'
