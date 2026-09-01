#!/usr/bin/env bash
set -euo pipefail
home="${CODEX_HOME:-$HOME/.codex}"; data="$home/plugins/data/compactveteran-compactveteran"; state="${XDG_STATE_HOME:-$HOME/.local/state}/compactveteran/install"; [[ -x "$data/compactveteran" ]] && { "$data/compactveteran" untrust; "$data/compactveteran" restore-config; }
if [[ -f "$state/original" ]]; then cp -p "$state/original" "$HOME/.local/bin/codex"; else ln -sfn "$home/packages/standalone/current/bin/codex" "$HOME/.local/bin/codex"; fi
rm -rf "$data" "$state"; echo "CompactVeteran uninstalled; project maps preserved at $home/project-maps."
