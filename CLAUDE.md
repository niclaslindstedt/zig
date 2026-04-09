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
2. Add tests
3. `make build` — compile cleanly
4. `make clippy` — zero warnings
5. `make fmt` — format
6. Update docs if needed (README.md, this file)
7. Commit using conventional commit style
