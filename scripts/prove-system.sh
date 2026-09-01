#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
for tool in cargo git python3 timeout; do command -v "$tool" >/dev/null || exit 1; done
cargo build --locked --manifest-path "$root/plugins/compactveteran/runtime/Cargo.toml" --quiet
for f in "$root/.agents/plugins/marketplace.json" "$root/plugins/compactveteran/.codex-plugin/plugin.json" "$root/plugins/compactveteran/hooks/hooks.json"; do python3 -m json.tool "$f" >/dev/null; done
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
export HOME="$tmp/home"
export CODEX_HOME="$HOME/.codex" XDG_STATE_HOME="$HOME/.local/state" PATH="$HOME/.local/bin:$PATH"
[[ "$HOME" == "$tmp/home" && "$CODEX_HOME" == "$tmp/home/.codex" && "$XDG_STATE_HOME" == "$tmp/home/.local/state" ]] || { echo 'proof isolation failed' >&2; exit 1; }
[[ "$HOME" == "$tmp/"* && "$CODEX_HOME" == "$tmp/"* && "$XDG_STATE_HOME" == "$tmp/"* ]] || { echo 'proof paths escaped temp root' >&2; exit 1; }
export PROOF_TMP="$tmp/proof" PROJECT_ROOT="$root" PROOF_REMOTE="$tmp/remote.git"
mkdir -p "$PROOF_TMP"
export FIXTURE_STATE="$XDG_STATE_HOME/fixture" FIXTURE_PLUGIN_ROOT="$root/plugins/compactveteran" FIXTURE_PLUGIN_BIN="$root/plugins/compactveteran/runtime/target/debug/compactveteran" FIXTURE_REPO="$tmp/repo" FIXTURE_CURRENT_LINK="$CODEX_HOME/packages/standalone/current" FIXTURE_V2_TARGET="$CODEX_HOME/packages/standalone/releases/v2"
mkdir -p "$HOME/.local/bin" "$CODEX_HOME/packages/standalone/releases/v1/bin" "$FIXTURE_V2_TARGET/bin" "$FIXTURE_STATE" "$FIXTURE_REPO"
printf '%s\n' '{"client_version":"fixture","etag":"etag","fetched_at":"2026-09-01","root":{"marker":"fixture"},"models":[{"slug":"gpt-5.6-sol","context_window":333333,"auto_compact_token_limit":222222,"marker":{"sol":1}},{"slug":"gpt-5.6-terra","context_window":444444,"marker":{"terra":2}},{"slug":"gpt-5.6-luna","context_window":555555,"marker":{"luna":3}},{"slug":"other","context_window":666666,"marker":{"other":4}}]}' >"$CODEX_HOME/models_cache.json"
for v in v1 v2; do printf '#!/usr/bin/env bash\nexec env STOCK_VERSION=%s python3 %q "$@"\n' "$v" "$root/scripts/fixtures/stock_codex.py" >"$CODEX_HOME/packages/standalone/releases/$v/bin/codex"; chmod +x "$CODEX_HOME/packages/standalone/releases/$v/bin/codex"; done
ln -s "$CODEX_HOME/packages/standalone/releases/v1" "$FIXTURE_CURRENT_LINK"; ln -s "$FIXTURE_CURRENT_LINK/bin/codex" "$HOME/.local/bin/codex"; readlink "$HOME/.local/bin/codex" >"$PROOF_TMP/original-target"
git -C "$FIXTURE_REPO" init -q -b main; git -C "$FIXTURE_REPO" config user.email test@example.com; git -C "$FIXTURE_REPO" config user.name test; git init --bare -q "$PROOF_REMOTE"; git -C "$FIXTURE_REPO" remote add origin "$PROOF_REMOTE"
printf '# fixture\n' >"$FIXTURE_REPO/README.md"; printf '# rules\n' >"$FIXTURE_REPO/AGENTS.md"; printf '# roadmap\n' >"$FIXTURE_REPO/ROADMAP.md"; git -C "$FIXTURE_REPO" add .; git -C "$FIXTURE_REPO" commit -qm initial; git -C "$FIXTURE_REPO" push -qu origin main
printf '%s\n' '# preserve' 'model_catalog_json="/prior/catalog.json"' 'model_context_window=333333' 'model_auto_compact_token_limit=222222' 'unrelated="keep"' >"$CODEX_HOME/config.toml"
COMPACTVETERAN_REPO="$root" COMPACTVETERAN_BINARY="$FIXTURE_PLUGIN_BIN" sh "$root/install.sh" >"$tmp/install.out"; test "$(tail -n1 "$tmp/install.out")" = 'CompactVeteran installed. No context left behind.'
python3 "$root/scripts/fixtures/assert_system.py" installed
printf '%s\n' 'after_install="preserve"' >>"$CODEX_HOME/config.toml"
(cd "$FIXTURE_REPO" && timeout 30 codex)
python3 "$root/scripts/fixtures/assert_system.py" lifecycle
sh "$root/uninstall.sh" >/dev/null
python3 "$root/scripts/fixtures/assert_system.py" uninstalled
echo 'CompactVeteran system proof: PASS'
