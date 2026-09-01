#!/usr/bin/env sh
set -eu
h=${CODEX_HOME:-$HOME/.codex}; d=$h/plugins/data/compactveteran-compactveteran; s=${XDG_STATE_HOME:-$HOME/.local/state}/compactveteran/install; b=$HOME/.local/bin/codex
stock=$h/packages/standalone/current/bin/codex
if test -x "$d/compactveteran"; then PLUGIN_DATA="$d" "$d/compactveteran" untrust || true; PLUGIN_DATA="$d" "$d/compactveteran" restore-config || true; fi
"$stock" plugin remove compactveteran@compactveteran >/dev/null 2>&1 || true; "$stock" plugin marketplace remove compactveteran >/dev/null 2>&1 || true
case $(cat "$s/kind" 2>/dev/null || echo missing) in symlink) rm -f "$b"; ln -s "$(cat "$s/target")" "$b";; file) rm -f "$b"; cp -p "$s/original" "$b";; *) rm -f "$b"; test ! -x "$h/packages/standalone/current/bin/codex" || ln -s "$h/packages/standalone/current/bin/codex" "$b";; esac
rm -rf "$d" "${XDG_STATE_HOME:-$HOME/.local/state}/compactveteran"; echo "CompactVeteran removed. Project maps preserved at $h/project-maps."
