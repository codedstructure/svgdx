#!/usr/bin/env bash

export RUSTDOCFLAGS="-D warnings"

cargo doc --no-deps --document-private-items --all-features --open
