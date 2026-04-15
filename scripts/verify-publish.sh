#!/usr/bin/env bash
# Simulates `cargo publish` for every workspace crate without hitting
# crates.io. Catches publish-isolation bugs — relative path references
# (e.g. `#[folder = "../web/dist/"]`, build.rs reading sibling files,
# include_str! escaping the crate root) that work under `cargo check`
# but fail at publish time because the tarball is extracted in isolation.
#
# `cargo package --workspace` (stable since Cargo 1.90) packages all
# members in dependency order and verifies each one against the
# previously-packaged siblings via a local registry overlay, so this
# works on version-bump PRs where the new versions aren't on crates.io
# yet.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> building web UI"
make web-build

echo "==> packaging workspace"
cargo package --workspace --allow-dirty --locked

echo "==> publish verification ok"
