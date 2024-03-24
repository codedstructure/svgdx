#!/usr/bin/env bash

set -e

export RUSTFLAGS=-Dwarnings

cargo fmt -q --check || (cargo fmt --verbose --check ; exit 1)

cargo clippy --verbose --all-targets --no-default-features
cargo clippy --verbose --all-targets --all-features

if cargo --list | grep -q llvm-cov ; then
    cargo llvm-cov --all-features
else
    cargo test --all-features
    echo
    echo "  ** Could not find llvm-cov; skipped coverage check."
    echo "  ** See https://github.com/taiki-e/cargo-llvm-cov for installation."
fi

