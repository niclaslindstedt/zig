#!/usr/bin/env bash
# Simulates `cargo publish --dry-run` for the crates that are at risk
# of publish-isolation bugs, without actually hitting crates.io.
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
# Scope:
#   - zig-core: has no workspace deps, so `cargo package` with full
#     verify exercises the real publish path.
#   - zig-serve: depends on zig-core (via path + version). We package
#     it with --no-verify and then run `cargo check` against the
#     extracted tarball with a [patch.crates-io] that points zig-core
#     at its own extracted packaged copy — this mirrors what the
#     real publish-verify does, substituting the yet-to-be-published
#     upstream crate.
#   - zig-cli is intentionally skipped. It has no build.rs, no
#     rust-embed, no include_str!, and no relative path references,
#     so the publish-isolation failure mode can't hit it. Verifying
#     it would require standing up a local crates.io replacement
#     (because `cargo package -p zig-cli` resolves zig-serve against
#     the registry during packaging, and [patch.crates-io] does not
#     apply there) which is a lot of machinery for no real coverage.
#
# Note: this script assumes the previous workspace crate is already
# available on crates.io at the current version, which is the common
# case on PRs between releases. On a version-bump PR that bumps all
# three crates at once, `cargo package -p zig-serve` will fail at
# dep resolution because the new zig-core version isn't published
# yet — in that case treat the failure as expected for the window
# between bumping the version and the first successful release.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "==> building web UI"
make web-build

# Sets the global EXTRACTED_DIR to the extracted tarball path.
# Using a global avoids the `x=$(cmd)` trap where `set -e` does not
# propagate failures from inside command substitutions.
EXTRACTED_DIR=""
package_and_extract() {
    local crate="$1"
    shift
    echo "==> packaging $crate"
    cargo package --allow-dirty "$@" -p "$crate"
    local tarball
    tarball="$(ls "target/package/${crate}-"*.crate | head -1)"
    tar -xzf "$tarball" -C "$WORK"
    EXTRACTED_DIR="$(find "$WORK" -maxdepth 1 -type d -name "${crate}-*" | head -1)"
    if [[ -z "$EXTRACTED_DIR" ]]; then
        echo "failed to locate extracted $crate tarball" >&2
        exit 1
    fi
}

verify_extracted() {
    local crate_dir="$1"
    shift
    # Append patch overrides so the extracted crate resolves its path
    # deps against our staged packaged copies.
    {
        echo
        echo "[patch.crates-io]"
        for override in "$@"; do
            echo "$override"
        done
    } >> "$crate_dir/Cargo.toml"
    echo "==> verifying $(basename "$crate_dir")"
    (cd "$crate_dir" && cargo check)
}

# zig-core: no workspace deps, full verify exercises the real path.
package_and_extract zig-core
CORE_DIR="$EXTRACTED_DIR"

# zig-serve: package without verify, then manually check against
# the extracted zig-core.
package_and_extract zig-serve --no-verify
SERVE_DIR="$EXTRACTED_DIR"
verify_extracted "$SERVE_DIR" \
    "zig-core = { path = \"$CORE_DIR\" }"

echo "==> publish verification ok"
