---
description: "Use when manpages may be stale. Discovers commits since the last manpage update, identifies what changed (commands, flags, workflow format, patterns, etc.), and updates the affected manpages/*.md files to match the current implementation."
---

# Updating the Manpages

The `manpages/` directory contains markdown manpages embedded at compile time
via `include_str!()` in `zig-core/src/man.rs` and accessed via
`zig man <topic>`. They are the authoritative **command-level** reference
documentation — one manpage per CLI command. Conceptual documentation (the
`.zug` format, patterns, variables, conditions, memory) lives in `docs/` and
is updated via the `update-docs` skill. Manpages get stale when CLI flags,
commands, or command behavior change without updating the corresponding
manpage.

## Current Manpages

| File | Covers |
|------|--------|
| `zig.md` | Overview of the zig CLI |
| `run.md` | Execute a `.zug` workflow file |
| `listen.md` | Tail a running or completed zig session |
| `workflow.md` | Manage workflows (list, show, create, delete, pack) |
| `resources.md` | Manage reference files advertised to agents |
| `describe.md` | Generate a `.zug` file from a prompt |
| `validate.md` | Validate a `.zug` workflow file |
| `serve.md` | Start the HTTP API server |

## Tracking Mechanism

The file `.claude/skills/update-manpages/.last-updated` contains the git commit hash from the last time the manpages were comprehensively updated. Use this as the baseline for discovering what changed.

## Discovery Process

1. Read the baseline commit hash:
   ```sh
   BASELINE=$(cat .claude/skills/update-manpages/.last-updated)
   ```

2. List all commits since the baseline:
   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. Check what files changed:
   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Categorize the changes using the manpage mapping below to determine which manpages need updating.

5. Read the affected manpages and source-of-truth files. Fix any discrepancies.

## Manpage Mapping

| Changed files / commit scope | Manpage(s) to update |
|------------------------------|---------------------|
| `zig-cli/src/cli.rs` (global flags: debug, quiet) | `zig.md` (Flags section) |
| `zig-cli/src/cli.rs` (Command::Run) | `run.md` |
| `zig-cli/src/cli.rs` (Command::Listen) | `listen.md` |
| `zig-cli/src/cli.rs` (Command::Workflow) | `workflow.md` |
| `zig-cli/src/cli.rs` (Command::Resources) | `resources.md` |
| `zig-cli/src/cli.rs` (Command::Describe) | `describe.md` |
| `zig-cli/src/cli.rs` (Command::Validate) | `validate.md` |
| `zig-cli/src/cli.rs` (Command::Serve) | `serve.md` |
| `zig-cli/src/cli.rs` (new Command variant) | New `manpages/<cmd>.md` + `zig.md` + `man.rs` |
| `zig-core/src/run.rs` | `run.md` |
| `zig-core/src/listen.rs` | `listen.md` |
| `zig-core/src/manage.rs` / `create.rs` / `pack.rs` | `workflow.md` |
| `zig-core/src/resources_manage.rs` | `resources.md` |
| `zig-serve/**` | `serve.md` |

## Implementation Files

### Primary

- `manpages/*.md` — the command manpage files being updated
- `zig-core/src/man.rs` — must be updated when adding new manpages (const, match arm, TOPICS entry)

### Secondary (read-only, sources of truth)

| Source of truth | What it tells you |
|----------------|-------------------|
| `zig-cli/src/cli.rs` | All CLI commands, flags, subcommands, patterns |
| `zig-core/src/run.rs` | Workflow execution behavior |
| `zig-core/src/listen.rs` | Session tailing behavior |
| `zig-core/src/manage.rs` | Workflow list/show/delete behavior |
| `zig-core/src/create.rs` | Interactive creation behavior |
| `zig-core/src/pack.rs` | Workflow packing behavior |
| `zig-core/src/resources_manage.rs` | Resource management command behavior |
| `zig-serve/**` | HTTP API server behavior |
| `docs/*.md` | Conceptual documentation (cross-reference via `zig docs <topic>`) |
| `README.md` | High-level docs (should be consistent with manpages) |

## Implementation Patterns

### Adding a new command

When a new variant is added to the `Command` enum in `cli.rs`:

1. Create `manpages/<cmd>.md` following the existing format
2. Update `zig.md` with the new command in the commands list
3. Update `zig-core/src/man.rs`:
   - Add `pub const <CMD>: &str = include_str!("../manpages/<cmd>.md");` in `mod pages`
   - Add `("<cmd>", "Description")` to `TOPICS`
   - Add `"<cmd>" => Some(pages::<CMD>),` to the `get()` match

### Command vs. concept

Anything that describes a concept (e.g., a new `.zug` field, a new orchestration
pattern, the memory tier layout) belongs in `docs/`, not `manpages/`. Use the
`update-docs` skill for those changes. Manpages should cross-reference concept
docs via `` `zig docs <topic>` ``.

## Update Checklist

- [ ] Read baseline from `.last-updated` and run `git log` to identify changes
- [ ] Read `zig-cli/src/cli.rs` to get current clap definitions
- [ ] Read all affected manpages and source-of-truth files
- [ ] Update `zig.md` if commands or global flags changed
- [ ] Update command-specific manpages for changed flags or behavior
- [ ] Create new `manpages/<cmd>.md` for any new commands
- [ ] Update `zig-core/src/man.rs` if manpages were added (const, match, TOPICS)
- [ ] Verify flag names and descriptions match `cli.rs` exactly
- [ ] Verify all examples use correct current syntax
- [ ] Consider whether `update-readme` or `update-docs` skills should also be run
- [ ] Update `.claude/skills/update-manpages/.last-updated` with current HEAD commit hash:
  ```sh
  git rev-parse HEAD > .claude/skills/update-manpages/.last-updated
  ```

## Verification

1. Build and run tests:
   ```sh
   make build
   make test
   ```
   The `man_tests.rs` tests verify manpage content has proper headers and that all topics are registered.
2. For each updated manpage, verify flag names and descriptions match `cli.rs` clap definitions
3. Verify new topics are registered in `man.rs` (const, match arm, TOPICS entry)
4. Ensure no sections were accidentally deleted or corrupted
5. Confirm `.last-updated` file was updated

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Update the mapping table**: If new source-of-truth files or manpage sections were discovered, add them.
2. **Add new patterns**: If you found a recurring update pattern not documented here, add it.
3. **Update the current manpages table**: If new manpages were added, update the table at the top.
4. **Commit the skill update** along with the manpage updates so improvements are preserved.
