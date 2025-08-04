#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.
set -eux

cargo check --quiet --workspace --all-targets
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --  -D warnings -W clippy::all

trunk build

# cargo install --locked cargo-deny
cargo deny check -d
# cargo install typos-cli
typos

# cargo test
cargo test --quiet --workspace --all-targets --all-features
cargo test --quiet --workspace --doc
