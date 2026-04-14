#!/usr/bin/env bash
# Simulates `cargo publish --dry-run` for the workspace crates in the
# order they would actually be published, without hitting crates.io.
#
# Why this exists: `cargo publish` extracts each crate into
# `target/package/<crate>-<version>/` and verifies it by building from
# that location. Anything referenced via a relative path outside the
# crate root (e.g. `#[derive(RustEmbed)] #[folder = "../web/dist/"]`,
# a `build.rs` reading sibling files, or an `include_str!` escaping
# the crate) silently works in normal `cargo check` but fails at
# publish time. This script reproduces that isolation so such bugs
# surface on PRs instead of during a release.
#
# Strategy:
#   1. `make web-build` so the real web UI is staged (not just an
#      empty `.gitkeep`).
#   2. `cargo package -p zig-core` with full verify (no path deps).
#   3. `cargo package --no-verify -p zig-serve` to generate the
#      tarball, then manually `cargo check` the extracted crate with a
#      `[patch.crates-io]` override that points `zig-core` at the
#      extracted packaged copy. This mirrors what crates.io would do
#      except we substitute the yet-to-be-published `zig-core`.
#   4. Same pattern for `zig-cli`, which depends on both.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "==> building web UI"
make web-build

# Echoes the path to the extracted crate directory.
package_and_extract() {
    local crate="$1"
    local verify_flag="${2:-}"
    echo "==> packaging $crate" >&2
    if [[ -n "$verify_flag" ]]; then
        cargo package --allow-dirty "$verify_flag" -p "$crate" >&2
    else
        cargo package --allow-dirty -p "$crate" >&2
    fi
    local tarball
    tarball="$(ls "target/package/${crate}-"*.crate | head -1)"
    tar -xzf "$tarball" -C "$WORK"
    local extracted
    extracted="$(find "$WORK" -maxdepth 1 -type d -name "${crate}-*" | head -1)"
    echo "$extracted"
}

verify_extracted() {
    local crate_dir="$1"
    shift
    # Append patch overrides for any path-only workspace deps so the
    # extracted crate can resolve them without contacting crates.io.
    {
        echo
        echo "[patch.crates-io]"
        for override in "$@"; do
            echo "$override"
        done
    } >> "$crate_dir/Cargo.toml"
    echo "==> verifying $(basename "$crate_dir")" >&2
    (cd "$crate_dir" && cargo check)
}

# zig-core has no workspace deps; the standard verify is enough.
CORE_DIR="$(package_and_extract zig-core)"

# zig-serve depends on zig-core via path; verify manually against the
# extracted packaged copy of zig-core.
SERVE_DIR="$(package_and_extract zig-serve --no-verify)"
verify_extracted "$SERVE_DIR" \
    "zig-core = { path = \"$CORE_DIR\" }"

# zig-cli depends on both; verify manually.
CLI_DIR="$(package_and_extract zig-cli --no-verify)"
verify_extracted "$CLI_DIR" \
    "zig-core = { path = \"$CORE_DIR\" }" \
    "zig-serve = { path = \"$SERVE_DIR\" }"

echo "==> publish verification ok"
