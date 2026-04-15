---
name: update-readme
description: "Use when the README.md may be stale. Discovers commits since the last README update, identifies what changed (commands, flags, workflows, architecture, etc.), and merges updates into README.md."
---

# Updating the README

The README.md is the primary user-facing documentation. It covers installation, commands, flags, the `.zwf` format, architecture, and development instructions. It gets stale when new features land without corresponding README updates.

## Tracking Mechanism

The file `.claude/skills/update-readme/.last-updated` contains the git commit hash from the last time the README was comprehensively updated. Use this as the baseline for discovering what changed.

## Discovery Process

1. Read the baseline commit hash:
   ```sh
   BASELINE=$(cat .claude/skills/update-readme/.last-updated)
   ```

2. List all commits since the baseline:
   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. Check what files changed:
   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Categorize the changes using the section mapping below to determine which README sections need updating.

5. Read the current README.md and the corresponding source-of-truth files. Fix any discrepancies.

## Section Mapping

| Changed files / commit scope | README section(s) to update |
|------------------------------|----------------------------|
| `zig-cli/src/cli.rs` (Command enum) | **Commands** list, **Flags** table |
| `zig-cli/src/cli.rs` (Pattern enum) | **Commands** (workflow create section) |
| `zig-core/src/workflow/` | **The `.zwf` format** section |
| `zig-core/src/run.rs` | **`zig run`** section, architecture notes |
| `zig-core/src/create.rs` | **`zig workflow create`** section |
| `zig-core/src/prompt.rs` | Prompt templates, workflow generation |
| Architecture changes | **Architecture** section |
| Install method changes | **Install** section |
| `Cargo.toml` (version, deps) | **Prerequisites**, **Install** |
| `Makefile` changes | **Development** section |

## Implementation Files

### Primary

- **README.md** — the file being updated

### Secondary (read-only, sources of truth)

| Source of truth | What it tells you |
|----------------|-------------------|
| `zig-cli/src/cli.rs` | All CLI commands, flags, subcommands |
| `zig-core/src/lib.rs` | Public modules and crate structure |
| `zig-core/src/workflow/model.rs` | `.zwf` workflow data model |
| `zig-core/src/workflow/parser.rs` | `.zwf` format parsing rules |
| `zig-core/src/run.rs` | Workflow execution logic |
| `zig-core/src/create.rs` | Interactive workflow creation |
| `zig-core/src/man.rs` | Manpage topics (mirrors command list) |
| `manpages/*.md` | Detailed command documentation |
| `Makefile` | Build commands and development workflow |

## Update Checklist

- [ ] Read baseline from `.last-updated` and run `git log` to identify changes
- [ ] Read `README.md` and all source-of-truth files for affected sections
- [ ] Update **Commands** list if commands were added/removed/renamed
- [ ] Update **Flags** table if CLI flags changed
- [ ] Update **`zig run`** section if run behavior changed
- [ ] Update **`zig workflow create`** section if creation behavior changed
- [ ] Update **The `.zwf` format** section if workflow model changed
- [ ] Update **Architecture** section if crate structure changed
- [ ] Update **Install** section if installation methods changed
- [ ] Update **Prerequisites** if dependency requirements changed
- [ ] Update **Development** section if build commands changed
- [ ] Verify all code examples are correct against current source
- [ ] Consider whether `update-manpages` skill should also be run
- [ ] Update `.claude/skills/update-readme/.last-updated` with current HEAD commit hash:
  ```sh
  git rev-parse HEAD > .claude/skills/update-readme/.last-updated
  ```

## Verification

1. Read through the updated README sections and verify they match the current source code
2. Check that all command names, flag names, and examples are syntactically correct
3. Ensure no sections were accidentally deleted or corrupted
4. Confirm the `.last-updated` file was updated

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Update line numbers**: If README sections shifted significantly, update the section mapping.
2. **Add new mappings**: If you discovered new source-of-truth files or README sections, add them.
3. **Record patterns**: If you found a recurring update pattern not documented here, add it.
4. **Commit the skill update** along with the README update so improvements are preserved.
