# CLAUDE.md

## Build commands

```bash
make build          # dev build
make release        # optimized release build
make test           # run all tests
make fmt            # format code
make clippy         # lint (zero warnings required)
make check          # type-check without building
make coverage       # coverage summary
make coverage-report # HTML coverage report
```

Run all checks before committing: `make build && make test && make clippy && make fmt`

## Commit conventions

Follow conventional commit style: `type(scope): summary`

**Types**: `feat`, `fix`, `refactor`, `docs`, `test`, `perf`, `chore`

PRs are squash-merged — the PR title becomes the commit message and drives changelog generation. Breaking changes use `!` (e.g., `feat!: ...`) or a `BREAKING CHANGE:` footer.

Do not manually edit `CHANGELOG.md` — it is auto-generated from conventional commits.

## Architecture

```
zig-core (library crate)
  .zug file parsing, workflow validation, execution engine

zig-cli (binary crate)
  CLI argument parsing (clap) → dispatch to zig-core
  Delegates to zag for agent interactions
```

Dependency flow: `zig-core ← zig-cli`

- **zig-cli/src/**: Thin CLI wrapper — argument parsing and command dispatch
- **zig-core/src/**: Core library — .zug format, workflow engine, zag integration

Zig uses `zag` (specifically `zag-orch` orchestration primitives) behind the scenes. The `zig describe` command invokes zag in interactive mode to generate `.zug` workflow files. The `zig run` command parses and executes `.zug` files by delegating to zag orchestration.

## Development workflow

1. Write code
2. Add tests (in separate `*_tests.rs` files, not inline `#[cfg(test)]` blocks)
3. `make build` — compile cleanly
4. `make test` — all tests pass
5. `make clippy` — zero warnings
6. `make fmt` — format
7. Update docs if needed (README.md, manpages, this file)
8. Commit using conventional commit style — use `/commit` to handle the full workflow

## Where new code goes

1. **CLI flags/commands** → `zig-cli/src/cli.rs`
2. **Command dispatch** → `zig-cli/src/main.rs`
3. **Workflow model** → `zig-core/src/workflow/model.rs`
4. **Workflow parsing** → `zig-core/src/workflow/parser.rs`
5. **Workflow validation** → `zig-core/src/workflow/validate.rs`
6. **Workflow execution** → `zig-core/src/run.rs`
7. **Workflow creation** → `zig-core/src/create.rs`
8. **Workflow management** → `zig-core/src/manage.rs` (list, show, delete)
9. **Prompt templates** → `zig-core/src/prompt.rs` + `prompts/`
10. **Manpages** → `manpages/*.md` + `zig-core/src/man.rs`
11. **TypeScript bindings** → `bindings/typescript/` (types, builder, workflow parser)

## Documentation sync points

| Change type | Files to update |
|-------------|----------------|
| New CLI command | `zig-cli/src/cli.rs` (Command or WorkflowCommand enum), `zig-cli/src/main.rs`, `manpages/<cmd>.md`, `zig-core/src/man.rs`, `README.md` |
| New CLI flag | `zig-cli/src/cli.rs`, relevant `manpages/*.md`, `README.md` |
| New pattern | `zig-cli/src/cli.rs` (Pattern enum), `manpages/patterns.md`, `manpages/workflow.md` |
| Workflow format change | `zig-core/src/workflow/`, `manpages/zug.md`, `docs/zug-vs-zag-gap-analysis.md` |
| CLI or model change | Run `update-bindings` skill — syncs TypeScript binding with Rust source |
| README staleness | Run `update-readme` skill — tracks last update via `.claude/skills/update-readme/.last-updated` |
| Manpage staleness | Run `update-manpages` skill — tracks last update via `.claude/skills/update-manpages/.last-updated` |
| Docs staleness | Run `update-docs` skill — tracks last update via `.claude/skills/update-docs/.last-updated` |
