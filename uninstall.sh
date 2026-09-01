#!/usr/bin/env bash
set -euo pipefail
home="${CODEX_HOME:-$HOME/.codex}"; data="$home/plugins/data/compactveteran-compactveteran"
if [[ -f "$data/stock-codex" ]]; then cp -f "$data/stock-codex" "$HOME/.local/bin/codex"; else rm -f "$HOME/.local/bin/codex"; fi
rm -rf "$data" "$home/compactveteran-models.json"
echo "CompactVeteran uninstalled; project maps preserved."
