---
name: maintenance
description: "Use when you want to bring every drift-prone artifact in the repo back into sync. Dispatches to all individual update-* skills in the correct order, aggregates their results, and leaves a single combined PR ready to review."
---

# Maintenance

This is the umbrella skill for zig. It does no rewriting itself — it decides which sync skills are stale, runs each one, and reports a combined summary. Use it when you do not know which specific artifact is out of date, or when several have likely drifted at once (for example, after a large merge).

## When to run

- After a big merge from the default branch when you are not sure which surfaces moved.
- On a cadence (weekly / before a release) as a "drift sweep".
- When CI flags a staleness check but it is unclear which skill to invoke.
- When Dependabot PRs have accumulated and need triage alongside a docs sweep.

Do **not** use this skill for a targeted fix — if you know exactly which artifact is stale, call the corresponding `update-*` skill directly.

## Registry

The registry is the single source of truth for which sync skills exist in this repo. Every `update-*` directory under `.agent/skills/` must appear here exactly once. Add rows whenever you create a new sync skill.

| Skill | Fixes | Run order |
|---|---|---|
| `update-manpages` | `manpages/*.md` vs. clap CLI definitions                            | 1 — manpages mirror the CLI parser and must settle first |
| `update-docs`     | `docs/*.md` vs. workflow format, engine, and zag integration        | 2 |
| `update-bindings` | `bindings/typescript/` vs. Rust workflow model (`zig-core/src/workflow/model.rs`) and CLI surface | 3 |
| `update-readme`   | `README.md` vs. current public surface                              | 4 |
| `update-website`  | `website/` source-derived content vs. commands, patterns, version   | 5 |

Run order matters: upstream fixes must land before downstream skills read them. Manpages settle first; `docs/` references those manpages; bindings mirror the Rust source independently but should be regenerated before the README summarizes them; the README summarizes everything above; the website is rendered from all of the above.

## Housekeeping checks

In addition to the staleness sweep, this skill verifies the repo's structural invariants. Run these before the staleness discovery:

1. All symlinks (`CLAUDE.md`, `.cursorrules`, `.windsurfrules`, `GEMINI.md`, `.github/copilot-instructions.md`, `.claude/skills`) point to the correct targets.
2. `.editorconfig`, `.github/dependabot.yml`, `.github/PULL_REQUEST_TEMPLATE.md`, and `.github/ISSUE_TEMPLATE/` are present and complete.
3. Makefile exposes the canonical targets: `build`, `test`, `lint`, `fmt`, `fmt-check`, `check`, `coverage`.
4. CI workflow uses `make` targets (not bare `cargo` commands) and pins the Rust toolchain version.
5. Release workflow is triggered by a tag push (`v*`); the `version-bump` workflow creates the tag via `scripts/release.sh` using a PAT so the push triggers `release.yml`.
6. All `.agent/skills/*/SKILL.md` files are present.

Report any housekeeping failures but do not abort the staleness sweep — fix them separately.

## Discovery process

For each skill in the registry, decide whether it needs to run:

1. Read the skill's `.last-updated` file:

   ```sh
   BASELINE=$(cat .agent/skills/<skill>/.last-updated)
   ```

   An empty or missing file means "never run" — schedule it.

2. Diff the watched paths for that skill against the baseline:

   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

   If any file in the skill's mapping table appears in the diff, schedule the skill.

3. Build the list of skills to run, preserving the run order from the registry.

## Execution

For each scheduled skill, in order:

1. Load `.agent/skills/<skill>/SKILL.md`.
2. Follow its discovery process, mapping table, and update checklist exactly.
3. Verify the skill's own verification section passes.
4. Record the commit hash the skill wrote to its `.last-updated`.

Between skills, do **not** commit — aggregate all edits into a single working tree so the final commit covers the whole sync sweep.

## Update checklist

- [ ] Run the housekeeping checks and note any failures
- [ ] Read every skill's `.last-updated` and build the schedule
- [ ] Run each scheduled skill in registry order
- [ ] After all skills finish, run:
    - [ ] `make fmt`
    - [ ] `make lint`
    - [ ] `make test`
- [ ] Stage every touched file (including each updated `.last-updated`)
- [ ] Commit with a conventional-commit message describing the sweep
- [ ] Update `.agent/skills/maintenance/.last-updated`:

      git rev-parse HEAD > .agent/skills/maintenance/.last-updated

- [ ] Hand off to the `commit` skill to push and open / update the PR

## Verification

1. Every scheduled skill's verification section must pass.
2. `make lint` and `make test` must pass.
3. The final diff should touch only documentation files, skill `.last-updated` files, and (rarely) small code adjustments that the skills flagged.
4. Every skill that ran must have its `.last-updated` rewritten with the same commit hash.

## Skill self-improvement

After every run, update this file:

1. **Add new sync skills to the registry.** Every new `update-*` skill must appear here, in alphabetical order, with a clear run-order slot.
2. **Adjust run order** if you discovered a hidden dependency (e.g. skill A reads files that skill B rewrites).
3. **Record drift signals.** If a change should have triggered a skill but did not appear in any skill's mapping table, extend that skill's mapping table — not this one.
4. **Commit the skill edits** together with the drift sweep so the orchestration knowledge compounds.
