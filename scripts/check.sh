#!/usr/bin/env bash

# Perform a bunch of checks on the project, starting with quickest

set -e

export RUSTFLAGS=-Dwarnings
export RUSTDOCFLAGS=-Dwarnings

cargo fmt -q --check || (cargo fmt --verbose --check ; exit 1)

# check each combination of Release/Dev and all/no features
cargo clippy --verbose --all-targets --no-default-features
cargo clippy --verbose --all-targets --all-features

# should be quick if we've just done clippy check
cargo doc --verbose --no-deps --document-private-items --all-features

# there are a few things cfg'd differently for release
cargo clippy --release --verbose --all-targets --no-default-features
cargo clippy --release --verbose --all-targets --all-features

if cargo --list | grep -q llvm-cov ; then
    cargo llvm-cov --all-features
else
    cargo test --all-features
    echo
    echo "  ** Could not find llvm-cov; skipped coverage check."
    echo "  ** See https://github.com/taiki-e/cargo-llvm-cov for installation."
fi
