#!/usr/bin/env bash
set -euo pipefail
repo=lmtlssss/CompactVeteran; h="${CODEX_HOME:-$HOME/.codex}"; d="$h/plugins/data/compactveteran-compactveteran"; s="${XDG_STATE_HOME:-$HOME/.local/state}/compactveteran/install"; mkdir -p "$d" "$s" "$HOME/.local/bin"
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || exit 1; command -v curl >/dev/null
stock="$h/packages/standalone/current/bin/codex"; [[ -x "$stock" ]] || stock="$(readlink -f "$(command -v codex)")"; [[ "$stock" != "$(readlink -f "$HOME/.local/bin/codex" 2>/dev/null || true)" ]] || exit 1
[[ -e "$s/original" ]] || { [[ -L "$HOME/.local/bin/codex" ]] && readlink "$HOME/.local/bin/codex" >"$s/original" || cp -p "$HOME/.local/bin/codex" "$s/original" 2>/dev/null || true; }
t="$(mktemp "$d/.cv.XXXXXX")"; trap 'rm -f "$t"' EXIT; curl -fsSL "https://github.com/$repo/releases/latest/download/compactveteran-x86_64-unknown-linux-gnu" -o "$t"; chmod 0755 "$t"; mv -f "$t" "$d/compactveteran"; printf '%s\n' "$stock" >"$s/stock-path"; "$d/compactveteran" refresh-catalog; "$d/compactveteran" install-config; "$d/compactveteran" trust
t="$(mktemp "$HOME/.local/bin/.codex.XXXXXX")"; printf '%s\n' '#!/usr/bin/env bash' 'exec "${CODEX_HOME:-$HOME/.codex}/plugins/data/compactveteran-compactveteran/compactveteran" supervisor "$@"' >"$t"; chmod 0755 "$t"; mv -f "$t" "$HOME/.local/bin/codex"; "$d/compactveteran" doctor; echo 'CompactVeteran installed. No context left behind.'
