#!/usr/bin/env bash

# For (size) profiling use `--profiling` here and `cargo install twiggy`,
# then `twiggy top editor/pkg/svgdx_bg.wasm`
PROFILE_ARG="--release"  # One of `--dev`, `--profiling`, `--release`
MANIFEST_PATH=$(cargo locate-project --message-format plain)
OUT_DIR="$(dirname $MANIFEST_PATH)/editor/pkg"

echo "Ensuring wasm-pack is installed..."
cargo install wasm-pack
echo "Building WASM to ${OUT_DIR} ..."
wasm-pack build "${PROFILE_ARG}" --out-dir "${OUT_DIR}" --target web --no-default-features --no-typescript --no-pack
echo
echo "Generated files:"
ls -l "${OUT_DIR}"
