#!/usr/bin/env bash
set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

BUMP="${1:-}"

# --- safety checks ---
[ -z "$(git status --porcelain)" ] || die "working tree is not clean"
[ "$(git branch --show-current)" = "main" ] || die "not on main branch"

# --- read current version ---
CARGO_TOML="zig-cli/Cargo.toml"
CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)"/\1/')
[[ "$CURRENT" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "could not parse version from $CARGO_TOML: $CURRENT"

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

# --- auto-detect bump from conventional commits ---
if [ -z "$BUMP" ]; then
    LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
    if [ -n "$LAST_TAG" ]; then
        COMMITS=$(git log "$LAST_TAG"..HEAD --pretty=format:"%s" --no-merges)
    else
        COMMITS=$(git log --pretty=format:"%s" --no-merges)
    fi

    BUMP="patch"
    while IFS= read -r msg; do
        [ -z "$msg" ] && continue
        if echo "$msg" | grep -qE '^[a-z]+(\(.+\))?!:|BREAKING CHANGE'; then
            BUMP="major"
            break
        elif echo "$msg" | grep -qE '^feat(\(.+\))?:'; then
            BUMP="minor"
        fi
    done <<< "$COMMITS"
fi

# --- bump version ---
case "$BUMP" in
    patch) PATCH=$((PATCH + 1)) ;;
    minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
    major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
    *) die "invalid bump type: $BUMP (expected patch, minor, or major)" ;;
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"
TAG="v$NEW_VERSION"

# --- check for duplicate tag ---
if git rev-parse "$TAG" >/dev/null 2>&1; then
    die "tag $TAG already exists"
fi

echo "=== Release ==="
echo "  current: $CURRENT"
echo "  bump:    $BUMP"
echo "  new:     $NEW_VERSION"
echo "  tag:     $TAG"
echo ""

# --- update versions and commit ---
scripts/update-versions.sh "$NEW_VERSION"
git add -A
git diff --cached --quiet || git commit -m "chore: bump version to $NEW_VERSION"

# --- create and push tag + version commit ---
git tag -a "$TAG" -m "Release $TAG"
git push origin main "$TAG"

echo ""
echo "Tag $TAG created and pushed. The release workflow will handle the rest."
