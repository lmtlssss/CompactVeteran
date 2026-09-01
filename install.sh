#!/usr/bin/env bash
set -euo pipefail
repo=${COMPACTVETERAN_REPO:-lmtlssss/CompactVeteran}; codex_home=${CODEX_HOME:-$HOME/.codex}; data=$codex_home/plugins/data/compactveteran-compactveteran; state=${XDG_STATE_HOME:-$HOME/.local/state}/compactveteran/install; bin=$HOME/.local/bin/codex
command -v codex >/dev/null; command -v curl >/dev/null; [[ $(uname -s) == Linux && $(uname -m) == x86_64 ]]
stock=$codex_home/packages/standalone/current/bin/codex; [[ -x $stock ]] || stock=$(readlink -f "$(command -v codex)"); [[ -x $stock ]] || exit 1
mkdir -p "$state" "$data" "$HOME/.local/bin"
if [[ ! -e $state/kind ]]; then if [[ -L $bin ]]; then echo symlink >"$state/kind"; readlink "$bin" >"$state/target"; elif [[ -f $bin ]]; then echo file >"$state/kind"; cp -p "$bin" "$state/original"; else echo missing >"$state/kind"; fi; fi
if [[ -d $repo ]]; then "$stock" plugin marketplace add "$repo" >/dev/null 2>&1 || true; else "$stock" plugin marketplace add "$repo" --ref main >/dev/null 2>&1 || true; fi
"$stock" plugin marketplace upgrade compactveteran >/dev/null 2>&1 || true; j=$("$stock" plugin add compactveteran@compactveteran --json); root=$(printf '%s\n' "$j" | sed -n 's/.*"installedPath"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p'); [[ -n $root && -d $root ]]
t=$(mktemp "$data/.cv.XXXXXX"); trap 'rm -f "$t"' EXIT; if [[ -n ${COMPACTVETERAN_BINARY:-} ]]; then cp "$COMPACTVETERAN_BINARY" "$t"; else curl -fsSL "https://github.com/$repo/releases/latest/download/compactveteran-x86_64-unknown-linux-gnu" -o "$t"; fi; chmod 0755 "$t"; mv -f "$t" "$data/compactveteran"; printf '%s\n' "$stock">"$state/stock-path"; PLUGIN_DATA="$data" "$data/compactveteran" refresh-catalog; PLUGIN_DATA="$data" "$data/compactveteran" install-config; PLUGIN_DATA="$data" "$data/compactveteran" trust
t=$(mktemp "$HOME/.local/bin/.codex.XXXXXX"); printf '%s\n' '#!/usr/bin/env bash' 'exec "${CODEX_HOME:-$HOME/.codex}/plugins/data/compactveteran-compactveteran/compactveteran" supervisor "$@"' >"$t"; chmod 0755 "$t"; mv -f "$t" "$bin"; PLUGIN_DATA="$data" "$data/compactveteran" doctor; echo 'CompactVeteran installed. No context left behind.'
