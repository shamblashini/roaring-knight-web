#!/usr/bin/env bash
# Extract assets from a local DELTARUNE install. The install is only ever READ:
# data files are copied into extract/ first and all tools run on the copies.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GAME="${DELTARUNE_DIR:-$HOME/.steam/steam/steamapps/common/DELTARUNE}"
UTMT="${UTMT_CLI:-$HOME/.cache/kf-tools/utmt/UndertaleModCli}"
CH="$GAME/chapter3_windows"
mkdir -p "$ROOT/extract/mus" "$ROOT/extract/ext"
rm -f "$ROOT/extract/data.win"
cp "$CH/data.win" "$CH/audiogroup1.dat" "$ROOT/extract/"
chmod -w "$ROOT/extract/data.win"
cp "$GAME/mus/knight.ogg" "$ROOT/extract/mus/"
cp "$CH"/*.ogg "$ROOT/extract/ext/"
export RK_OUT="$ROOT/extract/raw"
"$UTMT" dump "$ROOT/extract/data.win" -o "$ROOT/extract/dump" -c UMT_DUMP_ALL </dev/null
"$UTMT" load "$ROOT/extract/data.win" -s "$ROOT/tools/export_assets.csx" </dev/null
"$UTMT" load "$ROOT/extract/data.win" -s "$ROOT/tools/export_masks.csx" </dev/null
"$UTMT" load "$ROOT/extract/data.win" -s "$ROOT/tools/export_nineslice.csx" </dev/null
python3 "$ROOT/tools/object_depths.py"
echo "extract done"
