---
name: maintenance
description: "Periodic repository health checks, housekeeping, and staleness routing to update skills."
---

# maintenance

Periodic repository health checks and housekeeping. Acts as a router — after running its own checks, it invokes the appropriate update skills for any stale areas.

## When to run

Run this skill when:
- Dependabot PRs have accumulated and need triage
- CI is failing without an obvious cause
- The repo feels "drifted" from its conventions (AGENTS.md, Makefile targets, workflow triggers)

## What it checks

1. All symlinks (CLAUDE.md, .cursorrules, .windsurfrules, GEMINI.md, .github/copilot-instructions.md, .claude/skills) point to the correct targets
2. `.editorconfig`, `.github/dependabot.yml`, `.github/PULL_REQUEST_TEMPLATE.md`, and `.github/ISSUE_TEMPLATE/` are present and complete
3. Makefile exposes the canonical targets: `build`, `test`, `lint`, `fmt`, `fmt-check`, `check`, `coverage`
4. CI workflow uses `make` targets (not bare `cargo` commands) and pins the Rust toolchain version
5. Release workflow is triggered by a tag push (`v*`); the `version-bump` workflow creates the tag via `scripts/release.sh` using a PAT so the push triggers `release.yml`
6. All `.agent/skills/*/SKILL.md` files are present

## Staleness routing

After the checks above, evaluate each update skill and invoke it if stale:

| Skill | Stale when |
|-------|------------|
| `update-readme` | CLI commands, flags, architecture, or workflow model changed since `.agent/skills/update-readme/.last-updated` |
| `update-docs` | Workflow format, execution engine, patterns, or zag integration changed since `.agent/skills/update-docs/.last-updated` |
| `update-manpages` | CLI commands or flags changed since `.agent/skills/update-manpages/.last-updated` |
| `update-bindings` | Rust workflow model (`zig-core/src/workflow/model.rs`) or CLI surface changed since `.agent/skills/update-bindings/.last-updated` |
| `update-website` | Commands, patterns, workflow model, or version number changed since `.agent/skills/update-website/.last-updated` |

For each skill, compare the `.last-updated` timestamp (or last commit touching it) against recent commits that affect the relevant files. If changes exist, invoke the skill via `/skill-name`.

## How to run

This skill has no automated script — the checks and staleness evaluation are performed by reading the relevant files and comparing against AGENTS.md and the repo conventions.
