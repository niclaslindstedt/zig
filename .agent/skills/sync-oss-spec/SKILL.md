---
name: sync-oss-spec
description: "Use when this repo may have drifted out of conformance with OSS_SPEC.md. Runs the bash fallback validator from the oss-spec project (the nonbinary backup, since zig does not ship the oss-spec binary), walks the violations, and fixes each one until the repo is back in sync with the spec."
---

# Syncing zig with OSS_SPEC.md

`OSS_SPEC.md` at the repo root is the specification this project claims to conform to. zig is a *consumer* of the spec, not its reference implementation, so the canonical Rust validator (`oss-spec validate .`) is not built or installed locally. Instead, this skill drives validation through the **nonbinary fallback** that the oss-spec project ships alongside its binary: a language-agnostic bash mirror at `scripts/validate.sh` in the upstream repo. The bash mirror implements the same deterministic §19 checks as `src/validate/` and prints the AI quality checklist as a manual prompt at the end of its run.

This skill reacts to a change in *the repo* (a missing file, a malformed symlink, a stale workflow) by bringing it back under the spec's existing mandates. It is the counterpart to a hypothetical `update-spec`, which would react to a change in *the spec* by propagating new mandates into code. Run `sync-oss-spec` as the final step of a drift sweep — after `update-readme`, `update-docs`, etc. have settled — to catch residual violations that the per-artifact skills did not touch.

## Tracking mechanism

`.agent/skills/sync-oss-spec/.last-updated` contains the git commit hash of the last successful run. Empty means "never run" — use the repo's initial commit (`git rev-list --max-parents=0 HEAD`) as the baseline.

## Discovery process

1. Read the baseline:

   ```sh
   BASELINE=$(cat .agent/skills/sync-oss-spec/.last-updated)
   ```

2. Check whether `OSS_SPEC.md` itself changed since the baseline — that is the only input that can invalidate previously-passing conformance from outside the repo:

   ```sh
   git log --oneline "$BASELINE"..HEAD -- OSS_SPEC.md
   git diff --name-only "$BASELINE"..HEAD
   ```

   If `OSS_SPEC.md` changed, scan the diff for new MUST/SHALL clauses, new `§` subsections, and new entries in the bootstrap checklist — those are the most common sources of fresh violations.

3. Run the **nonbinary fallback validator** (the bash backup). Two equivalent invocations — pick whichever is more convenient:

   ```sh
   # No checkout needed — pulls the script from the upstream main branch and runs it against this repo:
   curl -fsSL https://raw.githubusercontent.com/niclaslindstedt/oss-spec/main/scripts/validate.sh | bash -s -- .

   # Or, if /tmp/oss-spec is already cloned (see AGENTS.md → "Related repositories"):
   /tmp/oss-spec/scripts/validate.sh .
   ```

   Each structural violation names the spec section (e.g. `§7.1`, `§10.3`, `§21.5`) and the file or directory at fault. The AI quality checklist printed at the end is a manual prompt — read it and act on any finding worth fixing.

   **Why the bash fallback is the primary path here.** zig does not vendor `scripts/validate.sh` and does not depend on the `oss-spec` crate, so building the Rust validator from source would require a fresh clone of the oss-spec repo and a `cargo build` that has nothing to do with zig's own toolchain. The bash mirror is deterministic, has no dependencies beyond a POSIX shell, and is the contract the oss-spec project publishes for downstream consumers exactly like this one.

4. For each violation, read the relevant section of `OSS_SPEC.md` so the fix matches the spec's intent rather than just silencing the check.

## Mapping table

| Violation spec section | Where to fix it in zig |
|---|---|
| §2 missing `LICENSE` | Create `LICENSE` with the SPDX-identified license text and the correct copyright holder |
| §3 missing `README.md` sections | Edit `README.md`; hand off to `update-readme` if extensive rewording is needed |
| §4/§5/§6 missing `CONTRIBUTING.md` / `CODE_OF_CONDUCT.md` / `SECURITY.md` | Create the file with the minimum content mandated by the corresponding spec section |
| §7.1 tool-specific guidance file is not a symlink | Replace the regular file with `ln -s AGENTS.md <path>` (or `ln -s ../AGENTS.md .github/copilot-instructions.md`). The required symlinks here are `CLAUDE.md`, `.cursorrules`, `.windsurfrules`, `GEMINI.md`, and `.github/copilot-instructions.md` |
| §8.4 missing `CHANGELOG.md` | Create an empty Keep-a-Changelog-formatted file; do **not** hand-author entries — `scripts/generate-changelog.sh` owns this file |
| §9 Makefile target missing | Add the missing target to `Makefile` and verify it runs end-to-end. The canonical zig targets are `build`, `test`, `lint`, `fmt`, `fmt-check`, `check`, `coverage`, `coverage-report` |
| §10.1/§10.3/§10.4 missing workflow | Create `.github/workflows/<file>.yml`; cross-reference `templates/_common/.github/workflows/` in `/tmp/oss-spec` for the canonical template |
| §10.3 floating or under-pinned toolchain | Edit the workflow to pin at or above the spec minimums in upstream `MIN_TOOLCHAIN_VERSIONS` (`src/validate/toolchain.rs` in the oss-spec repo) |
| §11.1 missing `docs/` content | Create the topic file under `docs/`, then run `update-docs` |
| §11.2 website drift | Run `make website` and inspect `website/src/generated/`; follow up with `update-website` |
| §13.5 `prompts/<name>/` has no versioned file | Add `prompts/<name>/1_0_0.md` with the required YAML front matter (`name`, `description`, `version: 1.0.0`, `references`) and `## System` / `## User` sections; then update the `include_str!` path in `zig-core/src/prompt.rs` per AGENTS.md → "Prompt versioning" |
| §15 missing issue / PR templates | Create the templates under `.github/ISSUE_TEMPLATE/` or `.github/PULL_REQUEST_TEMPLATE.md` |
| §19 raw print statement outside the project's central output module | Route the call through the equivalent helper (zig has no `src/output.rs` today; if violations land here, introduce one rather than papering over with `eprintln!`) |
| §20 inline `#[cfg(test)] mod { … }` block in `zig-core/src/` or `zig-cli/src/` | Move the tests to a sibling `*_tests.rs` file per AGENTS.md → "Test file conventions"; replace the gate with `#[cfg(test)] mod <name>_tests;` or delete it |
| §20.2 test file stem does not end with `_test(s)` / `Test(s)` | Rename the file so the stem matches `_?[Tt]ests?$` |
| §20.5 source file exceeds 1000 lines | **Preferred:** split the file by concern into sibling modules (the AGENTS.md "Where new code goes" table shows the canonical split points). **Common easy case:** if the file also has a §20 inline-test violation, extracting the test block to `*_tests.rs` usually resolves both at once. **Escape hatch:** if the size is genuinely justified (generated code, cohesive state machine, third-party snapshot), add `oss-spec:allow-large-file: <reason>` in any comment within the file's first 20 lines — the reason must be non-empty |
| §21.2 `.claude/skills` is not a symlink | Replace it with `ln -s ../.agent/skills .claude/skills` |
| §21.3 SKILL.md missing front matter fields | Add `name:` / `description:` to the front matter |
| §21.4 missing `.last-updated` | Touch the file and record the current `HEAD`: `git rev-parse HEAD > .agent/skills/<skill>/.last-updated` |
| §21.5 missing required `update-*` skill | Create `.agent/skills/<skill>/SKILL.md` (+ `.last-updated`); register it in `.agent/skills/maintenance/SKILL.md` and AGENTS.md → "Maintenance skills" |
| §21.6 `maintenance` skill registry row missing | Add the row in `.agent/skills/maintenance/SKILL.md`, alphabetical, with a deterministic run-order slot |

## Update checklist

- [ ] Read the baseline from `.agent/skills/sync-oss-spec/.last-updated` and diff `OSS_SPEC.md`
- [ ] Run the bash fallback validator and record every structural violation:

      curl -fsSL https://raw.githubusercontent.com/niclaslindstedt/oss-spec/main/scripts/validate.sh | bash -s -- .

- [ ] Read the AI quality checklist printed at the end of the bash run and record every finding worth acting on
- [ ] Walk the mapping table and fix each violation at its source
- [ ] If a fix would require a propagation step that belongs to a per-artifact skill (e.g. README rewording, manpage regen), hand off to that skill first, then re-run this skill
- [ ] Re-run the bash fallback validator — it must exit `0` and print "repo conforms to OSS_SPEC.md"
- [ ] Run `make fmt`, `make lint`, `make test` — zig's own toolchain must still be green after the fixes
- [ ] Write the new baseline:

      git rev-parse HEAD > .agent/skills/sync-oss-spec/.last-updated

## Verification

1. The bash fallback validator exits `0` and prints "repo conforms to OSS_SPEC.md".
2. `make build && make test && make lint && make fmt-check` all pass.
3. Every violation present before this run has a matching edit in the diff — no violations were silenced by editing `OSS_SPEC.md` or by adding spurious `oss-spec:allow-*` markers.
4. `.agent/skills/sync-oss-spec/.last-updated` was rewritten with the current `HEAD`.

## Skill self-improvement

After a run, extend this file:

1. **Grow the mapping table** whenever a new §X.Y section starts producing violations that the table does not yet cover. Keep the entries zig-specific — point at the actual file in this repo where the fix lands.
2. **Record fix recipes** (exact commands or edit patterns) for violations that required more than a one-line change.
3. **Flag recurring drift** — if the same violation keeps coming back, either a CI check is missing or a different skill's mapping table needs a new row. Fix the upstream cause, not just the symptom.
4. **Promote the binary path only if zig ever vendors `oss-spec`.** Until then, the bash fallback is the contract — do not add `cargo run -q -- validate` instructions that would only work inside the oss-spec repo.
5. **Commit the skill edit** alongside the repo fixes so the knowledge compounds.
