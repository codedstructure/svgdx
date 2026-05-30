#!/usr/bin/env bash

set -euo pipefail

# This script wraps mdbook and ensures the mdbook-svgdx preprocessor
# is installed and meets the required version.

for prereq in mdbook mdbook-svgdx; do
    if ! command -v "$prereq" >/dev/null 2>&1; then
        echo "error: $prereq not found" >&2
        exit 1
    fi
done

REQ_MAJOR=0
REQ_MINOR=16
SCRIPT_DIR="$(realpath "$(dirname "${BASH_SOURCE[0]}")")"
REPO_ROOT="${SCRIPT_DIR}/.."

usage() {
    echo "usage: $0 mdbook-args..." >&2
    exit 2
}

version_check() {
    local version="$1"
    local version_core="${version%%[-+]*}"
    local major minor rest

    IFS=. read -r major minor rest <<< "$version_core"

    [[ "$major" =~ ^[0-9]+$ ]] || return 1
    [[ "$minor" =~ ^[0-9]+$ ]] || return 1

    (( major > $REQ_MAJOR || (major == $REQ_MAJOR && minor >= $REQ_MINOR) ))
}

if [[ $# -lt 1 ]]; then
    usage
fi

# mdbook-svgdx -V: "mdbook-svgdx 0.16.0 (... details ...)"
version="$(mdbook-svgdx -V | awk '{print $2}')" || true

if [[ -z "$version" ]] || ! version_check "$version"; then
    echo "error: mdbook-svgdx >= ${REQ_MAJOR}.${REQ_MINOR} is required; found '${version:-unknown}'" >&2
    exit 1
fi

(cd "$REPO_ROOT"; cargo build --release --bin svgdx --no-default-features --features cli)
svgdx_bin="${REPO_ROOT}/target/release/svgdx"

if ! [[ -x "$svgdx_bin" ]]; then
    echo "error: could not locate built svgdx binary under cargo target directory" >&2
    exit 1
fi

(cd "$REPO_ROOT"; MDBOOK_SVGDX_BIN="$svgdx_bin" exec mdbook "$@")
