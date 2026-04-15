# AGENTS.md

This file is the canonical agent-guidance document for this repository. All AI agent tool files (CLAUDE.md, .cursorrules, .windsurfrules, GEMINI.md, .github/copilot-instructions.md) are symlinks to this file.

## Build commands

```bash
make build          # dev build
make release        # optimized release build
make test           # run all tests
make fmt            # format code
make lint           # lint (zero warnings required)
make fmt-check      # check formatting without modifying
make check          # type-check without building
make coverage       # coverage summary
make coverage-report # HTML coverage report
```

Run all checks before committing: `make build && make test && make lint && make fmt`

## Commit conventions

Follow conventional commit style: `type(scope): summary`

**Types**: `feat`, `fix`, `refactor`, `docs`, `test`, `perf`, `chore`

PRs are squash-merged — the PR title becomes the commit message and drives changelog generation. Breaking changes use `!` (e.g., `feat!: ...`) or a `BREAKING CHANGE:` footer.

Do not manually edit `CHANGELOG.md` — it is auto-generated from conventional commits.

## Branch naming

Use `<type>/<slug>` following the same types as commit conventions:

- `feat/my-feature`
- `fix/some-bug`
- `docs/update-readme`
- `chore/bump-deps`

## Architecture

```
zig-core (library crate)
  .zwf/.zwfz file parsing, workflow validation, execution engine

zig-cli (binary crate)
  CLI argument parsing (clap) → dispatch to zig-core
  Delegates to zag for agent interactions
```

Dependency flow: `zig-core ← zig-cli`

- **zig-cli/src/**: Thin CLI wrapper — argument parsing and command dispatch
- **zig-core/src/**: Core library — .zwf/.zwfz format, workflow engine, zag integration

Zig uses `zag` (specifically `zag-orch` orchestration primitives) behind the scenes. The `zig workflow create` command invokes zag in interactive mode to generate `.zwf` workflow files. The `zig run` command parses and executes `.zwf` / `.zwfz` files by delegating to zag orchestration.

## Development workflow

1. Write code
2. Add tests (in separate `*_tests.rs` files, not inline `#[cfg(test)]` blocks)
3. `make build` — compile cleanly
4. `make test` — all tests pass
5. `make lint` — zero warnings
6. `make fmt` — format
7. Update docs if needed (README.md, manpages, AGENTS.md)
8. Commit using conventional commit style — use `/commit` to handle the full workflow

## Test file conventions

- Tests live in separate `*_tests.rs` files (never inline `#[cfg(test)] mod tests` blocks)
- Test file stems must end with `_test`, `_tests`, `Test`, or `Tests`
- Functions called by external test files must be declared `pub`

## Where new code goes

1. **CLI flags/commands** → `zig-cli/src/cli.rs`
2. **Command dispatch** → `zig-cli/src/main.rs`
3. **Workflow model** → `zig-core/src/workflow/model.rs`
4. **Workflow parsing** → `zig-core/src/workflow/parser.rs`
5. **Workflow validation** → `zig-core/src/workflow/validate.rs`
6. **Workflow execution** → `zig-core/src/run.rs`
7. **Workflow creation** → `zig-core/src/create.rs`
8. **Workflow management** → `zig-core/src/manage.rs` (list, show, delete)
9. **Prompt templates** → `zig-core/src/prompt.rs` + `prompts/` — see **Prompt versioning** below
10. **Manpages** → `manpages/*.md` + `zig-core/src/man.rs`
11. **TypeScript bindings** → `bindings/typescript/` (types, builder, workflow parser)
12. **TypeScript API client** → `clients/typescript/` (`@nlindstedt/zig-api-client` — HTTP client for `zig-serve`)

## Documentation sync points

| Change type | Files to update |
|-------------|----------------|
| New CLI command | `zig-cli/src/cli.rs` (Command or WorkflowCommand enum), `zig-cli/src/main.rs`, `manpages/<cmd>.md`, `zig-core/src/man.rs`, `README.md` |
| New CLI flag | `zig-cli/src/cli.rs`, relevant `manpages/*.md`, `README.md` |
| New pattern | `zig-cli/src/cli.rs` (Pattern enum), `docs/patterns.md`, `manpages/workflow.md` |
| Workflow format change | `zig-core/src/workflow/`, `docs/zwf.md`, `docs/variables.md`, `docs/conditions.md` |
| New concept doc | `docs/<topic>.md`, `zig-core/src/docs.rs` (embed via `include_str!`) |
| New crate or binding | `scripts/update-versions.sh` (version bumps), `.github/workflows/release.yml` (publish steps) |
| CLI or model change | Run `update-bindings` skill — syncs TypeScript binding with Rust source |
| README staleness | Run `update-readme` skill — tracks last update via `.agent/skills/update-readme/.last-updated` |
| Manpage staleness | Run `update-manpages` skill — tracks last update via `.agent/skills/update-manpages/.last-updated` |
| Docs staleness | Run `update-docs` skill — tracks last update via `.agent/skills/update-docs/.last-updated` |
| Website staleness | Run `update-website` skill — tracks last update via `.agent/skills/update-website/.last-updated` |

## Prompt versioning

Prompt templates live in `prompts/<name>/<major>_<minor>.md` and are embedded via `include_str!` in `zig-core/src/prompt.rs`.

**Rules:**
- **Never edit an existing version file.** Always create a new file and update the `include_str!` path in `prompt.rs`.
- Version using SemVer-style major/minor: bump **minor** (`1_1` → `1_2`) for small adjustments (wording tweaks, adding a guideline). Bump **major** (`1_2` → `2_0`) for rewrites or structural changes that fundamentally alter the prompt.
- Every prompt file must have YAML front matter with `name`, `description`, `version`, and `references` (files that use it). Keep front matter up to date when creating a new version — update the description to reflect what changed and list current references.

## Website staleness policy

The `website/` directory contains generated source data. Run the `update-website` skill whenever commands, patterns, workflow model, or the version number changes. The skill tracks its own last-update timestamp at `.agent/skills/update-website/.last-updated`.

## Parity and checklist rules

Cross-cutting invariants that must hold across the entire repo:

- **TypeScript bindings** — run the `update-bindings` skill whenever the Rust workflow model (`zig-core/src/workflow/model.rs`) or CLI surface changes. The TypeScript types and builder in `bindings/typescript/` must stay in sync with Rust.
- **Agent tool files** — `CLAUDE.md`, `.cursorrules`, `.windsurfrules`, `GEMINI.md`, and `.github/copilot-instructions.md` must remain POSIX symlinks to `AGENTS.md`. Never replace them with independent copies.
- **CHANGELOG.md** — never edit manually. It is auto-generated by `scripts/generate-changelog.sh` from conventional commits. All changes flow through commit messages and the release script.
- **Prompt versioning** — never edit an existing prompt version file. Always create a new version file and update the `include_str!` path in `zig-core/src/prompt.rs`.
- **CLI ↔ manpage parity** — every flag and subcommand visible in `zig --help` must have a corresponding entry in the relevant `manpages/*.md` file. Run `update-manpages` skill when CLI changes.
- **Version consistency** — all crate `Cargo.toml` files, the npm `package.json` in `bindings/typescript/`, and the `clients/typescript/` package must share the same version number. `scripts/update-versions.sh` handles this; never bump versions manually.
- **Website source data** — `website/` is generated. Run `update-website` skill after any release or CLI/model change.

## Maintenance skills

Skills live under `.agent/skills/` (`.claude/skills` is a symlink to it). Each skill directory contains a `SKILL.md` describing when and how to invoke it.

| Skill | Trigger |
|-------|---------|
| `maintenance` | General repo health checks; run periodically |
| `update-readme` | README.md is stale after CLI/model changes |
| `update-docs` | docs/ is stale after workflow format changes |
| `update-website` | website/ source data is stale after any release |
| `update-manpages` | manpages/*.md are stale after CLI changes |
| `update-bindings` | TypeScript bindings are stale after Rust model changes |
| `commit` | Commit, push, and open a PR |
