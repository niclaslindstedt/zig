# Contributing to zig

Thank you for your interest in contributing to zig! This document covers the development workflow and guidelines.

## Prerequisites

- **Rust 1.94+** (edition 2024)
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
6. Update documentation if needed (README.md, manpages/*.md, docs/ topic files, AGENTS.md) — see the sync-points table in AGENTS.md for which surfaces need updating per change type
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

## Pull request process

PRs require **one approving review** before merge. The CODEOWNERS file routes reviews automatically based on changed paths. Only repository maintainers (currently [@niclaslindstedt](https://github.com/niclaslindstedt)) have merge rights. All PRs are squash-merged — the PR title becomes the commit message.

## Release process

Releases are automated via `scripts/release.sh`, which bumps the version, creates a git tag, and pushes it. The CI release workflow then builds binaries and creates a GitHub release.

```bash
make release-tag              # auto-detect bump from commits
make release-tag BUMP=minor   # explicit bump
```

## Branch naming

Use `<type>/<slug>` following the same types as commit conventions:

- `feat/my-feature`
- `fix/some-bug`
- `docs/update-readme`
- `chore/bump-deps`

## Code of conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating you agree to abide by its terms. Instances of unacceptable behavior may be reported as described in that document.

## Security

Do not open public issues for security vulnerabilities. Instead, use [GitHub's private vulnerability reporting](https://github.com/niclaslindstedt/zig/security/advisories/new). See [SECURITY.md](SECURITY.md) for the full policy.

## Reporting issues

Open an issue on [GitHub Issues](https://github.com/niclaslindstedt/zig/issues) with:

- Your Rust version (`rustc --version`)
- Your zag version (`zag --version`)
- Steps to reproduce the problem

## Governance

**Decision-making.** This project operates as a BDFL (Benevolent Dictator For Life) model. [@niclaslindstedt](https://github.com/niclaslindstedt) makes final decisions on direction, feature acceptance, and breaking changes. Proposals are discussed openly in GitHub Issues and PRs; consensus is preferred but not required.

**Merge rights.** Only [@niclaslindstedt](https://github.com/niclaslindstedt) has merge rights on the main branch.

**Maintainer onboarding.** New maintainers may be nominated by the current maintainer based on sustained, high-quality contribution. Nomination is at the sole discretion of the current maintainer and is formalized by granting repository write access and adding the maintainer to CODEOWNERS.

**Conflict resolution.** Disagreements about technical direction should be raised as GitHub Issues or PR comments. If a discussion reaches an impasse, the current maintainer makes the final call. Decisions are documented in the relevant issue or PR for future reference.

**Project transfer and forking.** If this project is abandoned (no activity for 12+ months, no response to issues), interested parties are encouraged to fork under a new name. The MIT license explicitly permits this. If the maintainer wishes to transfer stewardship, they will post a public announcement in GitHub Discussions and transfer repository ownership to a willing successor.

## License

All contributions are licensed under [MIT](LICENSE).
