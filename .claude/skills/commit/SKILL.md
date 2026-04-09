---
description: "Commit, push, and open a PR. Runs quality checks, creates a conventional commit, pushes the branch, and opens or updates a pull request."
---

# Commit, Push & PR Workflow

## Pre-commit Quality Checks

Before committing, run all four checks and ensure they pass:

```sh
make build && make test && make clippy && make fmt
```

All four must succeed. Fix any issues before proceeding.

## Branch Management

Always work on a feature branch — never commit directly to `main`. Branch naming should follow `type/short-description` in kebab-case, derived from the commit type and summary.

## Commit Message Format

Conventional commit format: `type(scope): summary in imperative mood`

**Types and their changelog impact:**

| Type | Changelog section | Version bump |
|------|-------------------|--------------|
| `feat` | Added | minor |
| `fix` | Fixed | patch |
| `perf` | Performance | patch |
| `docs` | Documentation | none |
| `test` | Tests | none |
| `refactor`, `chore`, `ci`, `style`, `build` | *(not included)* | none |

**Scopes**: lowercase, comma-separated if multiple (e.g., `refactor(parser,cli): simplify error handling`).

**Breaking changes**: Use `type!: summary` or add a `BREAKING CHANGE:` footer for major version bumps.

## PR Title

The PR title **must** follow conventional commit format — it becomes the squashed commit message on `main` and drives changelog generation. It should reflect the combined scope of all commits on the branch.

When adding commits to an existing PR, update the PR title and description to reflect the new combined scope.

## Documentation Sync

If user-facing behavior changed, also update:

- `README.md` — commands, flags, examples
- `manpages/*.md` — command reference pages
- `CLAUDE.md` — if architecture changed
- `zig-core/src/man.rs` — if manpages were added or removed

## Workflow

1. Run quality checks: `make build && make test && make clippy && make fmt`
2. Stage changed files
3. Commit with conventional commit message
4. Push to feature branch
5. Open or update PR with conventional commit title and description

## Verification

```sh
make build    # Must compile cleanly
make test     # All tests must pass
make clippy   # Zero warnings
make fmt      # Formatted
```
