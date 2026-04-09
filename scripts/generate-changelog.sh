#!/usr/bin/env bash
set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

NEW_VERSION="${1:-}"
PREV_TAG="${2:-}"

[ -n "$NEW_VERSION" ] || die "usage: generate-changelog.sh <version> [previous-tag]"
[[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "invalid version format: $NEW_VERSION"

cd "$(git rev-parse --show-toplevel)"

# --- determine commit range ---
if [ -z "$PREV_TAG" ]; then
    PREV_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
fi

if [ -n "$PREV_TAG" ]; then
    COMMITS=$(git log "$PREV_TAG"..HEAD --pretty=format:"%s" --no-merges)
else
    COMMITS=$(git log --pretty=format:"%s" --no-merges)
fi

# --- categorize commits ---
ADDED=""
FIXED=""
PERF=""
DOCS=""
TESTS=""

while IFS= read -r msg; do
    [ -z "$msg" ] && continue

    # Strip conventional commit prefix and capitalize
    CLEAN=$(echo "$msg" | sed 's/^[a-z]*\(([^)]*)\)\?: *//' | sed 's/^./\U&/')

    case "$msg" in
        feat*) ADDED="${ADDED}- ${CLEAN}\n" ;;
        fix*)  FIXED="${FIXED}- ${CLEAN}\n" ;;
        perf*) PERF="${PERF}- ${CLEAN}\n" ;;
        docs*) DOCS="${DOCS}- ${CLEAN}\n" ;;
        test*) TESTS="${TESTS}- ${CLEAN}\n" ;;
    esac
done <<< "$COMMITS"

# --- build new section ---
TODAY=$(date +%Y-%m-%d)
SECTION="## [$NEW_VERSION] - $TODAY\n"

if [ -n "$ADDED" ]; then SECTION="${SECTION}\n### Added\n\n${ADDED}"; fi
if [ -n "$FIXED" ]; then SECTION="${SECTION}\n### Fixed\n\n${FIXED}"; fi
if [ -n "$PERF" ];  then SECTION="${SECTION}\n### Performance\n\n${PERF}"; fi
if [ -n "$DOCS" ];  then SECTION="${SECTION}\n### Documentation\n\n${DOCS}"; fi
if [ -n "$TESTS" ]; then SECTION="${SECTION}\n### Tests\n\n${TESTS}"; fi

if [ -z "$ADDED$FIXED$PERF$DOCS$TESTS" ]; then
    SECTION="${SECTION}\nNo notable changes.\n"
fi

# --- write changelog ---
CHANGELOG="CHANGELOG.md"

if [ ! -f "$CHANGELOG" ]; then
    printf "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\nThe format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\nand this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n" > "$CHANGELOG"
    printf "%b" "$SECTION" >> "$CHANGELOG"
    echo "Created $CHANGELOG"
else
    # Insert new section after the header (before first ## entry)
    TMPFILE=$(mktemp)
    awk -v section="$(printf '%b' "$SECTION")" '
        /^## \[/ && !inserted {
            print section
            inserted = 1
        }
        { print }
    ' "$CHANGELOG" > "$TMPFILE"

    # If no existing version entry was found, append
    if ! grep -q "^## \[" "$TMPFILE"; then
        printf "%b" "$SECTION" >> "$CHANGELOG"
    else
        mv "$TMPFILE" "$CHANGELOG"
    fi
    rm -f "$TMPFILE"
    echo "Updated $CHANGELOG"
fi
