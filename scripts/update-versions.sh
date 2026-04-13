#!/usr/bin/env bash
set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

NEW_VERSION="${1:-}"
[ -n "$NEW_VERSION" ] || die "usage: update-versions.sh <version>"
[[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "invalid version format: $NEW_VERSION"

cd "$(git rev-parse --show-toplevel)"

echo "Updating versions to $NEW_VERSION..."

# --- update Cargo.toml package versions ---
for toml in zig-cli/Cargo.toml zig-core/Cargo.toml zig-serve/Cargo.toml; do
    sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$toml"
    rm -f "$toml.bak"
    echo "  updated $toml"
done

# --- update internal dependency versions ---
for toml in zig-cli/Cargo.toml zig-serve/Cargo.toml; do
    sed -i.bak "s/zig-core = { version = \"[^\"]*\"/zig-core = { version = \"$NEW_VERSION\"/" "$toml"
    rm -f "$toml.bak"
    echo "  updated zig-core dependency in $toml"
done

# --- update npm package version ---
NPM_PKG="bindings/typescript/package.json"
sed -i.bak "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" "$NPM_PKG"
rm -f "$NPM_PKG.bak"
echo "  updated $NPM_PKG"

# --- regenerate lockfile ---
cargo generate-lockfile 2>/dev/null || cargo check 2>/dev/null || true
echo "  regenerated Cargo.lock"

echo "Done."
