# Contributing to zig

Thank you for your interest in contributing to zig! This document covers the development workflow and guidelines.

## Prerequisites

- **Rust 1.85+** (edition 2024)
- **zag CLI** — zig uses zag behind the scenes
- **GNU Make** — for build automation

## Getting started

```bash
git clone https://github.com/niclaslindstedt/zig.git
cd zig
make build
make test
```

## Quality checks

Before submitting code, run all checks:

```bash
make build && make test && make clippy && make fmt
```

## Development workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Make your changes
4. Add tests for new functionality
5. Run all quality checks (see above)
6. Update documentation if needed (README.md, CLAUDE.md)
7. Commit using conventional commit messages
8. Open a pull request

## Project architecture

```
zig-core (library crate)
  .zwf/.zwfz file parsing, workflow validation, execution engine

zig-cli (binary crate)
  CLI argument parsing (clap) → dispatch to zig-core
  Delegates to zag for agent interactions
```

The dependency flow is: `zig-core ← zig-cli`.

- **zig-cli/src/**: Thin CLI wrapper — argument parsing and command dispatch
- **zig-core/src/**: Core library — .zwf/.zwfz format, workflow engine, zag integration

## Commit conventions

Follow the [conventional commit](https://www.conventionalcommits.org/) style:

```
type(scope): summary
```

**Types**: `feat`, `fix`, `refactor`, `docs`, `test`, `perf`, `chore`

Scopes should be lowercase. Use comma-separated scopes when a change spans multiple areas.

PRs are squash-merged, so the PR title becomes the commit message and drives changelog generation. Breaking changes use `!` (e.g., `feat!: ...`) or a `BREAKING CHANGE:` footer.

## Code standards

- Format with `make fmt`
- Zero clippy warnings (`make clippy`)
- Keep tests in separate `_tests.rs` files
- Place core logic in `zig-core`, not the CLI crate

## Testing

```bash
# Run all tests
make test

# Run a specific test
cargo test -p zig-core test_name

# Coverage summary
make coverage

# HTML coverage report
make coverage-report
# Open .coverage/html/index.html
```

## Release process

Releases are automated via `scripts/release.sh`, which bumps the version, creates a git tag, and pushes it. The CI release workflow then builds binaries and creates a GitHub release.

```bash
make release-tag              # auto-detect bump from commits
make release-tag BUMP=minor   # explicit bump
```

## Reporting issues

Open an issue on [GitHub Issues](https://github.com/niclaslindstedt/zig/issues) with:

- Your Rust version (`rustc --version`)
- Your zag version (`zag --version`)
- Steps to reproduce the problem

## License

All contributions are licensed under [MIT](LICENSE).
