---
description: "Use when docs may be stale. Discovers commits since the last docs update, identifies what changed (workflow format, execution engine, patterns, zag integration, etc.), and updates the affected docs/*.md files to match the current implementation."
---

# Updating the Docs

The `docs/` directory contains in-depth documentation for zig's features and design decisions. Unlike manpages (command-level reference) or the README (overview), docs/ files explore concepts in depth with analysis and cross-references. They get stale when the workflow format, execution engine, or zag integration changes without corresponding docs updates.

## Current Docs

| File | Covers |
|------|--------|
| `zwf-vs-zag-gap-analysis.md` | Gap analysis between .zwf format capabilities and zag features |

## Tracking Mechanism

The file `.claude/skills/update-docs/.last-updated` contains the git commit hash from the last time the docs were comprehensively updated. Use this as the baseline for discovering what changed.

## Discovery Process

1. Read the baseline commit hash:
   ```sh
   BASELINE=$(cat .claude/skills/update-docs/.last-updated)
   ```

2. List all commits since the baseline:
   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. Check what files changed:
   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Categorize the changes using the docs mapping below to determine which docs need updating.

5. Read the affected docs and the corresponding source-of-truth files. Fix any discrepancies.

## Docs Mapping

| Changed files / commit scope | Doc(s) to update |
|------------------------------|-----------------|
| `zig-core/src/workflow/model.rs` (new fields, types) | `zwf-vs-zag-gap-analysis.md` (supported features table) |
| `zig-core/src/workflow/parser.rs` (format changes) | `zwf-vs-zag-gap-analysis.md` |
| `zig-core/src/run.rs` (execution changes) | `zwf-vs-zag-gap-analysis.md` (zag mapping column) |
| `zig-cli/src/cli.rs` (Pattern enum) | `zwf-vs-zag-gap-analysis.md` (pattern support) |
| New zag features adopted | `zwf-vs-zag-gap-analysis.md` (close gaps) |
| New .zwf fields | `zwf-vs-zag-gap-analysis.md` (add to supported table) |

## Implementation Files

### Primary (docs being updated)

- `docs/*.md` — the documentation files

### Secondary (read-only, sources of truth)

| Source of truth | What it tells you |
|----------------|-------------------|
| `zig-core/src/workflow/model.rs` | .zwf data model — all fields and types |
| `zig-core/src/workflow/parser.rs` | .zwf format parsing rules |
| `zig-core/src/workflow/validate.rs` | Validation constraints |
| `zig-core/src/run.rs` | How workflows are executed via zag |
| `zig-cli/src/cli.rs` | CLI commands and patterns |
| `manpages/*.md` | Command reference (should be consistent) |
| `README.md` | High-level overview (should be consistent) |

## Implementation Patterns

### Closing a gap in the gap analysis

When a .zwf feature is implemented that was previously listed as a gap:

1. Move the item from the "gaps" section to the "supported" section
2. Add the zag mapping (how it maps to zag commands/flags)
3. Update the summary counts

### Adding a new gap

When a new zag feature is discovered that .zwf doesn't support:

1. Add to the appropriate gap category
2. Document what the zag feature does and why it might be useful
3. Suggest a potential .zwf syntax if applicable

### Adding a new doc file

When a new conceptual topic needs documentation:

1. Create `docs/<topic>.md` with clear title and scope
2. Update the "Current Docs" table in this skill file

## Update Checklist

- [ ] Read baseline from `.last-updated` and run `git log` to identify changes
- [ ] Read all affected docs and source-of-truth files
- [ ] Update `zwf-vs-zag-gap-analysis.md` if workflow model, parser, or execution changed
- [ ] Verify examples use correct current syntax
- [ ] Ensure no sections were accidentally deleted or corrupted
- [ ] Consider whether `update-readme` and `update-manpages` skills should also be run
- [ ] Update `.claude/skills/update-docs/.last-updated` with current HEAD commit hash:
  ```sh
  git rev-parse HEAD > .claude/skills/update-docs/.last-updated
  ```

## Verification

1. Read each updated doc and verify facts against source code
2. Check all internal cross-references
3. Confirm `.last-updated` file was updated

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Update the mapping table**: If new source-of-truth files or doc sections were discovered, add them.
2. **Add new patterns**: If you found a recurring update pattern not documented here, add it.
3. **Update the current docs table**: If new docs were added, update the table at the top.
4. **Commit the skill update** along with the docs updates so improvements are preserved.
