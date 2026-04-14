---
description: "Use when docs may be stale. Discovers commits since the last docs update, identifies what changed (workflow format, execution engine, patterns, zag integration, etc.), and updates the affected docs/*.md files to match the current implementation."
---

# Updating the Docs

The `docs/` directory contains conceptual documentation — the `.zug` format,
orchestration patterns, the variable system, condition expressions, and the
memory scratch pad. Unlike manpages (command-level reference) or the README
(overview), docs/ files explore concepts in depth with examples and
cross-references. They are embedded into the `zig` binary via
`zig-core/src/docs.rs` and exposed through the `zig docs <topic>` command.
They get stale when the workflow format, execution engine, or CLI surface
changes without corresponding docs updates.

## Current Docs

| File | Covers |
|------|--------|
| `zug.md` | The `.zug` workflow format (sections, fields, zip archives) |
| `patterns.md` | Orchestration patterns (sequential, fan-out, generator/critic, …) |
| `variables.md` | Variable declarations, substitution, saves, data flow |
| `conditions.md` | Condition expressions for step gating |
| `memory.md` | Memory scratch pad, tiers, rendered `<memory>` block |

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
| `zig-core/src/workflow/model.rs` (new fields, types) | `zug.md`, `variables.md`, `conditions.md` |
| `zig-core/src/workflow/parser.rs` (format changes) | `zug.md` |
| `zig-core/src/workflow/validate.rs` (validation rules) | `zug.md`, `variables.md` |
| `zig-core/src/run.rs` (execution changes) | `zug.md`, `patterns.md` |
| `zig-core/src/memory.rs` (memory tiers, manifest, modes) | `memory.md` |
| `zig-cli/src/cli.rs` (Pattern enum) | `patterns.md` |
| New `.zug` field | `zug.md` (field tables), possibly `variables.md`/`conditions.md` |
| New orchestration pattern | `patterns.md` |
| New concept that needs documentation | new `docs/<topic>.md` + `zig-core/src/docs.rs` |

## Implementation Files

### Primary (docs being updated)

- `docs/*.md` — the documentation files
- `zig-core/src/docs.rs` — the embedded docs registry (add a new topic here when creating a new file)

### Secondary (read-only, sources of truth)

| Source of truth | What it tells you |
|----------------|-------------------|
| `zig-core/src/workflow/model.rs` | `.zug` data model — all fields and types |
| `zig-core/src/workflow/parser.rs` | `.zug` format parsing rules |
| `zig-core/src/workflow/validate.rs` | Validation constraints |
| `zig-core/src/run.rs` | How workflows are executed via zag |
| `zig-core/src/memory.rs` | Memory tiers, manifest format, injection modes |
| `zig-cli/src/cli.rs` | CLI commands and patterns |
| `manpages/*.md` | Command reference (should be consistent) |
| `README.md` | High-level overview (should be consistent) |

## Implementation Patterns

### Adding a new `.zug` field

1. Update `docs/zug.md` field tables with the new field, default, and description
2. Update `docs/variables.md` or `docs/conditions.md` if the field affects those subsystems
3. Add an example to the relevant section

### Adding a new orchestration pattern

1. Add a new section to `docs/patterns.md` with a description, "use when", and example
2. Reference `zig workflow create --pattern <name>` if the CLI scaffolds it

### Adding a new doc file

When a new conceptual topic needs documentation:

1. Create `docs/<topic>.md` with a clear title and scope
2. Add an `include_str!` entry and a `TOPICS` row in `zig-core/src/docs.rs`
3. Add a test entry in `zig-core/src/docs_tests.rs`
4. Update the "Current Docs" table in this skill file
5. Add cross-references from related docs via `zig docs <topic>`

## Update Checklist

- [ ] Read baseline from `.last-updated` and run `git log` to identify changes
- [ ] Read all affected docs and source-of-truth files
- [ ] Update `zug.md` if workflow model or parser changed
- [ ] Update `patterns.md` if orchestration patterns or execution model changed
- [ ] Update `variables.md` / `conditions.md` if those subsystems changed
- [ ] Update `memory.md` if the memory subsystem changed
- [ ] Verify examples use correct current syntax
- [ ] Ensure no sections were accidentally deleted or corrupted
- [ ] Verify `zig docs <topic>` renders each updated doc
- [ ] Consider whether `update-readme` and `update-manpages` skills should also be run
- [ ] Update `.claude/skills/update-docs/.last-updated` with current HEAD commit hash:
  ```sh
  git rev-parse HEAD > .claude/skills/update-docs/.last-updated
  ```

## Verification

1. Read each updated doc and verify facts against source code
2. Run `cargo build` to ensure embedded docs still compile
3. Run `zig docs` to verify the listing and `zig docs <topic>` for each changed file
4. Check all internal cross-references
5. Confirm `.last-updated` file was updated

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Update the mapping table**: If new source-of-truth files or doc sections were discovered, add them.
2. **Add new patterns**: If you found a recurring update pattern not documented here, add it.
3. **Update the current docs table**: If new docs were added, update the table at the top.
4. **Commit the skill update** along with the docs updates so improvements are preserved.
