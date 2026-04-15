# maintenance

Periodic repository health checks and housekeeping.

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
5. Release workflow is triggered by `workflow_run` on `version-bump`, not by a tag push
6. All `.agent/skills/*/SKILL.md` files are present

## How to run

This skill has no automated script — the checks above are performed by reading the relevant files and comparing against AGENTS.md and the OSS spec.
