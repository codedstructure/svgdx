#!/usr/bin/env bash

# For (size) profiling use `--profiling` here and `cargo install twiggy`,
# then `twiggy top pkg/svgdx_bg.wasm`
PROFILE_ARG="--release"  # One of `--dev`, `--profiling`, `--release`

echo "Ensuring wasm-pack is installed..."
cargo install wasm-pack
echo "Building WASM to pkg/ ..."
wasm-pack build "${PROFILE_ARG}" --target web --no-default-features --no-typescript --no-pack
echo
echo "Generated files:"
ls -l pkg/
