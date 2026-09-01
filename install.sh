#!/usr/bin/env bash
set -euo pipefail
repo="lmtlssss/CompactVeteran"; asset="compactveteran-x86_64-unknown-linux-gnu"
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || exit 1
command -v codex >/dev/null || exit 1
codex_home="${CODEX_HOME:-$HOME/.codex}"; data="$codex_home/plugins/data/compactveteran-compactveteran"; mkdir -p "$data" "$HOME/.local/bin"
curl -fsSL "https://github.com/$repo/releases/latest/download/$asset" -o "$data/compactveteran"; chmod 0755 "$data/compactveteran"
stock="$(command -v codex)"; [[ "$stock" != "$HOME/.local/bin/codex" ]] && cp -f "$stock" "$data/stock-codex" || true
cat > "$HOME/.local/bin/codex" <<'SH'
#!/usr/bin/env bash
exec "${CODEX_HOME:-$HOME/.codex}/plugins/data/compactveteran-compactveteran/compactveteran" supervisor "$@"
SH
chmod 0755 "$HOME/.local/bin/codex"; echo "CompactVeteran installed."
