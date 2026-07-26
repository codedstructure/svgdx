#!/usr/bin/env bash

set -euo pipefail

if ! command -v pandoc >/dev/null 2>&1; then
    echo "error: pandoc not found on PATH" >&2
    exit 1
fi

SCRIPT_DIR="$(realpath "$(dirname "${BASH_SOURCE[0]}")")"
LUA_FILTER="$SCRIPT_DIR/svgdx-pandoc-filter.lua"
INPUT="$SCRIPT_DIR/example.md"
OUT_DIR="$SCRIPT_DIR/out"
STDOUT_DIR="$OUT_DIR/stdout-cwd"

mkdir -p "$OUT_DIR/images"
mkdir -p "$STDOUT_DIR/images"

for output in out.html out.md out.pdf; do
    pandoc --lua-filter "$LUA_FILTER" "$INPUT" -o "$OUT_DIR/$output"
done

pushd "$STDOUT_DIR" >/dev/null
pandoc --lua-filter "$LUA_FILTER" "$INPUT" -t markdown > "$OUT_DIR/stdout.md"
popd >/dev/null

grep -F '![](images/green-circle.svg)' "$OUT_DIR/out.md" >/dev/null
grep -F "![]($STDOUT_DIR/images/green-circle.svg)" "$OUT_DIR/stdout.md" >/dev/null
