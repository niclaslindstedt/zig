# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.11.0] - 2026-05-01

### Added

- Add zig continue to resume the last step's agent session (#130)
- Register interactive steps with zag's process store (#129)
## [0.10.1] - 2026-04-20

### Fixed

- Bump @vitejs/plugin-react to ^6.0.1 for vite 8 peer dep (#115)

### Documentation

- Sync OSS_SPEC.md to v2.3.0
## [0.10.0] - 2026-04-19

### Added

- Add `zig self terminate` and instruct interactive agents to self-exit (#112)
- Inject validation report into agent user prompt (#111)

### Fixed

- Render heartbeats as live waiting indicator (#110)
## [0.9.0] - 2026-04-18

### Added

- Wire interactive step flag to inherit tty (#106)
- Add --dry-run flag to preview workflow execution (#105)
- Add storage, memory, and resources to healthcare example (#102)

### Fixed

- Restore live per-step agent output via on_log_event (#108)
- Harden zig-serve and zig-core against traversal, DoS, timing, and CORS issues (#104)

### Documentation

- Add storage, memory, and resources to embedded examples (#103)
- Add related repositories section to AGENTS.md (#101)
## [0.8.0] - 2026-04-16

### Added

- Add --no-storage flag to disable storage injection (#100)
## [0.7.1] - 2026-04-16

### Fixed

- Change release trigger from workflow_run to tag push (#92)
## [0.7.0] - 2026-04-15

### Added

- Add storage phase to create and update prompts (#90)
- Add workflow storage for structured working data (#89)

## [0.6.3] - 2026-04-15

### Fixed

- Use tilde-collapsed paths in workflow display output (#65)

## [0.6.2] - 2026-04-14

### Fixed

- Patch zig-core in workspace when verifying zig-serve publish (#61)

## [0.6.1] - 2026-04-14

### Added

- Expand ~/ and $HOME in workflow path fields (#59)
- Overhaul create and update prompts with phased conversation flow (#58)

### Fixed

- Make expand_path tests pass on Windows (#60)
- Stage embedded web UI via OUT_DIR so cargo publish succeeds (#57)

### Documentation

- Sync manpages with memory, serve TLS, and local workflows (#56)
- Document memory field on workflows and steps (#54)
- Sync README with memory, update, serve, and workflow changes (#53)

## [0.6.0] - 2026-04-14

### Added

- Add interactive update command and rename .zug → .zwf/.zwfz (breaking change) (#49)
- Add embedded orchestration pattern examples for workflow creation (#47)
- Add local workflow discovery from .zig/workflows/ (#46)
- Add memory scratch pad for workflows and steps (#45)
- Add resource discovery to create prompt and versioning system
- Add --web flag with embedded React chat UI (#44)

### Fixed

- Strip YAML front matter before sending prompts to agents (#48)

## [0.5.7] - 2026-04-13

### Fixed

- Publish zig-serve crate and sync its dependency version
- Include npm package.json in version bump script

### Documentation

- Add release sync points for new crates and bindings

## [0.5.6] - 2026-04-13

### Fixed

- Fix npm and crates.io publishing in release workflow

## [0.5.5] - 2026-04-13

No notable changes.

## [0.5.4] - 2026-04-13

No notable changes.

## [0.5.3] - 2026-04-13

### Fixed

- Trigger release on tag push and fix npm self-upgrade (#43)

## [0.5.2] - 2026-04-13

### Fixed

- Use npm trusted publishing for @nlindstedt/zig-workflow (#42)

## [0.5.1] - 2026-04-13

### Fixed

- Bump zig-serve dependency to 0.5.0 (#41)

## [0.5.0] - 2026-04-13

### Added

- Advertise inline resource files in system prompt (#40)

## [0.4.5] - 2026-04-13

No notable changes.

## [0.4.4] - 2026-04-13

### Added

- Improve workflow list output and add --json flag (#38)

### Fixed

- Include zig-serve in version bump script (#39)

## [0.4.3] - 2026-04-12

### Fixed

- Resolve manpage and prompt paths for crate publishing (#37)

## [0.4.2] - 2026-04-12

### Added

- Add TLS, user accounts, rate limiting, SSE streaming, and graceful shutdown (#35)
- Add HTTP API server for workflow orchestration (#34)
- Sync TypeScript binding with Rust source (#31)
- Add version, provider, and model to workflow metadata (#28)
- Add roles, file injection, zip archives, and pack command (#27)
- Add TypeScript bindings package (@nlindstedt/zig-workflow) (#26)
- Support variable substitution in step system_prompt (#25)

### Fixed

- Pass tag_name to gh release action

### Documentation

- Sync manpages and gap analysis with current implementation (#32)
- Update README with listen, pack, roles, and architecture changes (#30)

## [0.4.1] - 2026-04-11

No notable changes.

## [0.4.0] - 2026-04-10

### Added

- Add zig sessions and `zig listen` command (#24)
- Stream step output live and auto-parallelize tier-mates (#22)
- Wire step fields to zag and close gap analysis batch (#20)
- Wire step fields to zag CLI and add race group execution (#19)

### Fixed

- Bump versions before tagging in release script (#21)
