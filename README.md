# zig

[![ci](https://github.com/niclaslindstedt/zig/actions/workflows/ci.yml/badge.svg)](https://github.com/niclaslindstedt/zig/actions/workflows/ci.yml)
[![release](https://github.com/niclaslindstedt/zig/actions/workflows/release.yml/badge.svg)](https://github.com/niclaslindstedt/zig/actions/workflows/release.yml)
[![crates](https://img.shields.io/crates/v/zig-cli.svg)](https://crates.io/crates/zig-cli)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Describe workflows. Share them. Run them.

`zig` is an orchestration CLI for AI coding agents. It uses [zag](https://github.com/niclaslindstedt/zag) behind the scenes to let you describe a workflow in natural language, capture it as a shareable `.zwf` file, and replay it anywhere with a single command.

## Why zig?

- **Natural language workflows** — Describe what you want done in plain English; an AI agent creates the orchestration file for you
- **Shareable `.zwf` files** — Workflow definitions are portable files you can commit, share, and version alongside your code
- **Powered by zag** — Built on zag's battle-tested orchestration primitives (spawn, wait, collect, pipe) without needing to learn them directly
- **Reproducible automation** — Run the same workflow across projects, teams, and machines with `zig run`

## How it works

1. **Describe** — Tell `zig` what you want to automate. It launches a zag interactive session where an AI agent helps you design the workflow and produces a `.zwf` orchestration file.

2. **Share** — The `.zwf` file is a self-contained workflow definition. Commit it to your repo, send it to a colleague, or publish it.

3. **Run** — Execute the workflow with `zig run <workflow>`. Zig parses the `.zwf` file and delegates to zag's orchestration engine to carry out each step.

```bash
# Create a workflow interactively — an agent helps you design a .zwf file
zig workflow create code-review

# See what workflows are available
zig workflow list

# Run a workflow
zig run code-review

# Start the HTTP API
zig serve

# Start the HTTP API with the built-in React chat web UI
zig serve --web
```

### Web UI

`zig serve --web` embeds a single-page React chat interface inside the
`zig` binary (no filesystem or Node runtime required). When the server
starts it prints a `Web UI:` URL with an authentication token baked in —
open it in a browser to start a workflow creation chat. Submit the first
message to spawn an interactive zag session, then send follow-up messages
to collaborate with the agent until your `.zwf` file is ready.

## Prerequisites

- **Rust 1.94+** (edition 2024) — for building from source
- **zag CLI** — zig delegates to zag for agent orchestration ([install zag](https://github.com/niclaslindstedt/zag#install))
- At least one AI agent CLI installed (Claude, Codex, Gemini, Copilot, or Ollama — see [zag docs](https://github.com/niclaslindstedt/zag#agent-clis))

## Install

### From crates.io

```bash
cargo install zig-cli
```

### From GitHub Releases

Download a pre-built binary from [GitHub Releases](https://github.com/niclaslindstedt/zig/releases), extract it, and place it in your `PATH`.

### From source

```bash
git clone https://github.com/niclaslindstedt/zig.git
cd zig
cargo install --path zig-cli
```

## Quick start

```bash
# 1. Install
cargo install zig-cli

# 2. Create a workflow (an AI agent helps you design it)
zig workflow create demo

# 3. Run it
zig run demo
```

## Commands

```
zig run <workflow>              Execute a .zwf/.zwfz workflow file
zig listen [session_id]         Tail a running or completed zig session
zig serve                       Start the HTTP API server (optionally with --web UI)
zig workflow list               List available workflows
zig workflow show <workflow>    Show details of a workflow
zig workflow create [name]      Create a new workflow interactively with an AI agent
zig workflow update <workflow>  Revise an existing workflow interactively with an AI agent
zig workflow delete <workflow>  Delete a workflow file
zig workflow pack <path>        Pack a workflow directory into a .zwfz zip archive
zig resources list              List discovered resource files (global + cwd tiers)
zig resources add <file>        Register a file as a global / cwd / per-workflow resource
zig resources delete <name>     Delete a registered resource
zig resources show <name>       Print the absolute path and contents of a resource
zig resources where             Print the directories the collector searches
zig memory list                 List memory scratch-pad entries
zig memory add <file>           Add a file to the memory scratch pad
zig memory update <id>          Update metadata for a memory entry
zig memory delete <id>          Delete a memory entry and its file
zig memory show <id>            Show metadata and contents of a memory entry
zig memory search <query>       Search across memory files
zig validate <file>             Validate a .zwf/.zwfz workflow file
zig man [topic]                 Show manual pages for zig topics
zig docs [topic]                Show conceptual documentation topics
```

### `zig run`

Execute a `.zwf` or `.zwfz` workflow. Zig resolves the workflow by name or file path, parses the orchestration steps, and delegates execution to zag.

```bash
zig run code-review
zig run ./workflows/deploy.zwf
zig run code-review "focus on the authentication module"
zig run code-review --no-resources   # skip injecting the <resources> block
zig run code-review --no-memory      # skip injecting the <memory> block
```

### `zig listen`

Tail a running or completed zig session. Streams step output in real time, useful for monitoring long-running workflows.

```bash
zig listen                     # tail the most recently started session
zig listen --latest            # same as above (explicit)
zig listen --active            # tail the most recently active (still-running) session
zig listen abc123              # tail a specific session by ID or prefix
```

### `zig workflow create`

Launch an interactive session where an AI agent guides you through designing a workflow and produces a `.zwf` file.

```bash
zig workflow create my-workflow
zig workflow create deploy --pattern sequential
zig workflow create --output workflows/ci.zwf
```

Supported `--pattern` values: `sequential`, `fan-out`, `generator-critic`,
`coordinator-dispatcher`, `hierarchical-decomposition`, `human-in-the-loop`,
`inter-agent-communication`.

### `zig workflow update`

Revise an existing workflow interactively with an AI agent. The agent loads
the current definition, walks through your requested changes, and writes the
updated file back in place.

```bash
zig workflow update my-workflow
zig workflow update ./workflows/deploy.zwf
```

### `zig workflow list`

List all `.zwf` / `.zwfz` workflows discoverable from the current directory
(`./`, `./workflows/`, and `.zig/workflows/` walking up to the git root).

```bash
zig workflow list
zig workflow list --json    # machine-readable output
```

### `zig workflow delete`

Delete a `.zwf` workflow file by name or path.

```bash
zig workflow delete my-workflow
zig workflow delete workflows/old-pipeline.zwf
```

### `zig workflow pack`

Pack a workflow directory into a `.zwfz` zip archive. The directory must contain a workflow TOML file and can include prompt files referenced via `system_prompt_file` or `default_file`. The resulting archive works with `zig run` and `zig validate`.

```bash
zig workflow pack ./my-workflow/
zig workflow pack ./my-workflow/ --output custom-name.zwfz
```

### `zig resources`

Manage reference files (CVs, style guides, reference docs, …) that zig advertises to step agents through their system prompt. Resources live in three on-disk tiers — `~/.zig/resources/_shared/`, `~/.zig/resources/<workflow-name>/`, and `<git-root>/.zig/resources/` — and are merged with any inline `resources = [...]` declared in the workflow file.

```bash
# Drop a CV in a workflow-specific global tier
zig resources add ./cv.md --workflow cover-letter

# Stage shared style guides for every workflow
zig resources add ./style-guide.md --global

# Project-local reference docs (walks up to the git root)
zig resources add ./architecture.md --cwd

# Inspect what the collector will see
zig resources list
zig resources where --workflow cover-letter

# Skip resource injection for a single run
zig run cover-letter --no-resources
```

Run `zig man resources` for the full collection model and tier ordering.

### `zig memory`

Manage a memory scratch pad — durable notes, summaries, and artifacts that
zig advertises to step agents through a `<memory>` block in the system prompt.
Entries are tracked by numeric ID with metadata (name, description, tags, and
optional workflow/step association) and live under `~/.zig/memory/` (global
and per-workflow tiers) plus `<git-root>/.zig/memory/` (project tier).

```bash
# Add a note for a specific workflow
zig memory add ./architecture-notes.md \
  --workflow cover-letter \
  --description "Team architecture overview" \
  --tags arch,design

# Update metadata on an existing entry
zig memory update 42 --description "Updated overview" --tags arch

# Inspect, search, or remove entries
zig memory list --workflow cover-letter
zig memory show 42
zig memory search "authentication" --scope section
zig memory delete 42

# Skip the <memory> block on a single run
zig run cover-letter --no-memory
```

Run `zig docs memory` for the full memory model and search semantics.

### `zig serve`

Start the zig HTTP API server (`zig-serve`) for orchestrating workflows
remotely. Supports TLS, bearer-token authentication, rate limiting, SSE
streaming, and graceful shutdown. Settings can also be stored in
`~/.zig/serve.toml` under a `[server]` section — precedence is
CLI flag > env var > config file > default.

```bash
zig serve                                       # bind 127.0.0.1:3000
zig serve --port 8080 --host 0.0.0.0
zig serve --token "$ZIG_SERVE_TOKEN"            # or set ZIG_SERVE_TOKEN env var
zig serve --tls                                 # auto-generated self-signed cert
zig serve --tls-cert cert.pem --tls-key key.pem # explicit cert/key
zig serve --rate-limit 100                      # 100 req/s
zig serve --web                                 # also serve the embedded React UI
```

Run `zig man serve` for the complete flag reference and API routes.

### `zig validate`

Validate a `.zwf` or `.zwfz` workflow file for structural correctness without executing it.

```bash
zig validate my-workflow.zwf
zig validate my-workflow.zwfz
```

### `zig man`

Show built-in manual pages for zig commands (zig, run, listen, serve, workflow, validate, resources).

```bash
zig man run
zig man workflow
```

### `zig docs`

Show conceptual documentation (zwf, patterns, variables, conditions, memory).

```bash
zig docs zwf
zig docs patterns
```

## Flags

| Flag | Short | Scope | Description |
|------|-------|-------|-------------|
| `--debug` | `-d` | global | Enable debug logging |
| `--quiet` | `-q` | global | Suppress all output except errors |
| `--output <path>` | `-o` | `workflow create`, `workflow pack` | Output file path |
| `--pattern <name>` | `-p` | `workflow create` | Orchestration pattern (sequential, fan-out, generator-critic, coordinator-dispatcher, hierarchical-decomposition, human-in-the-loop, inter-agent-communication) |
| `--json` | | `workflow list` | Emit the workflow listing as JSON |
| `--latest` | | `listen` | Tail the most recently started session |
| `--active` | | `listen` | Tail the most recently active (still-running) session |
| `--no-resources` | | `run` | Skip the `<resources>` block injected into each step's system prompt |
| `--no-memory` | | `run` | Skip the `<memory>` block injected into each step's system prompt |
| `--global` | | `resources add/delete/list` | Target the global tier (`~/.zig/resources/_shared/`) |
| `--cwd` | | `resources add/delete/list` | Target the project tier (`<git-root>/.zig/resources/`) |
| `--workflow <name>` | | `resources`, `memory` | Restrict to a specific workflow's tier |
| `--step <name>` | | `memory add` | Tag a memory entry with an originating step (metadata only) |
| `--name <name>` | | `resources add`, `memory add/update` | Custom display name for the entry |
| `--description <text>` | | `memory add/update` | Description attached to a memory entry |
| `--tags <list>` | | `memory add/update` | Comma-separated tags for a memory entry |
| `--scope <granularity>` | | `memory search` | Search granularity (`sentence`, `paragraph`, `section`, `file`) |
| `--port <n>` | `-p` | `serve` | Port to listen on (default: 3000) |
| `--host <addr>` | | `serve` | Host/IP to bind to (default: 127.0.0.1) |
| `--token <value>` | | `serve` | Bearer token for authentication (or `ZIG_SERVE_TOKEN`) |
| `--shutdown-timeout <s>` | | `serve` | Graceful shutdown timeout in seconds (default: 30) |
| `--tls` | | `serve` | Enable TLS with auto-generated self-signed certificates |
| `--tls-cert <path>` | | `serve` | Path to a TLS certificate PEM file (implies `--tls`) |
| `--tls-key <path>` | | `serve` | Path to a TLS private key PEM file (implies `--tls`) |
| `--rate-limit <rps>` | | `serve` | Rate limit in requests per second |
| `--web` | | `serve` | Serve the built-in React web UI from `/` alongside the API |

## The `.zwf` / `.zwfz` format

A `.zwf` file is a TOML workflow definition that describes a DAG of AI agent steps with shared variables, conditional routing, and data flow. It is generated by `zig workflow create` and executed by `zig run`. The format captures:

- **Workflow metadata** — Name, description, tags, version, and default provider/model for all steps
- **Roles** — Reusable role definitions with system prompts (inline or loaded from files) that steps can reference
- **Steps** — The ordered (or parallel) operations to perform, each mapping to a zag agent invocation
- **Agent configuration** — Provider (`claude`, `codex`, `gemini`, `copilot`, `ollama`) and model per step, with workflow-level defaults
- **Dependencies** — How steps relate to and depend on each other via `depends_on`
- **Variables & data flow** — Shared state between steps via `${var}` references, `saves` selectors, input bindings (`from = "prompt"`), and variable constraints (required, min/max, patterns, allowed values)
- **Resources** — Reference files advertised to step agents through the system prompt (paths only — agents read them on demand). Inline `resources = [...]` is merged with global (`~/.zig/resources/`) and project (`<git-root>/.zig/resources/`) tiers at run time
- **Memory** — A managed scratch pad of prior notes, summaries, and artifacts exposed to steps via a `<memory>` block in the system prompt. Entries live under `~/.zig/memory/` (global / per-workflow) and `<git-root>/.zig/memory/` (project), and can be injected, searched, or skipped per run with `--no-memory`
- **Conditions** — Expressions that control whether steps run (`var < 8`, `status == "done"`)
- **Step commands** — Steps can invoke different zag commands: `run` (default), `review`, `plan`, `pipe`, `collect`, `summary`
- **Isolation** — Steps can run in isolated git worktrees or Docker sandboxes
- **Advanced orchestration** — Race groups (first-to-finish cancels siblings), retry with model escalation, interactive sessions, file injection

Workflows can also be packaged as zip archives (via `zig workflow pack`) that bundle the TOML definition with external prompt files.

Run `zig docs zwf` for the full format specification, or see `zig docs patterns` for common orchestration patterns.

## Architecture

```
zig-core (library crate)
  .zwf/.zwfz file parsing, workflow validation, execution engine,
  session tracking, pack/archive support

zig-cli (binary crate)
  CLI argument parsing (clap) → dispatch to zig-core
  Delegates to zag for agent interactions
```

`zig` is built as a Rust workspace with two crates:

- **zig-core** — Core library that handles `.zwf`/`.zwfz` file parsing, workflow validation, execution engine, session tracking, and archive packing. Key modules:
  - `workflow/` — Model, parser, and validator for the `.zwf`/`.zwfz` format
  - `run` — Workflow execution (DAG resolution, step parallelization, streaming output)
  - `listen` — Real-time session tailing
  - `session` — Session lifecycle and log coordination
  - `pack` — Zip archive creation for workflow directories
  - `create` — Interactive workflow creation via zag
  - `update` — Interactive workflow revision via zag
  - `manage` — Workflow listing, display, and deletion (with `.zig/workflows/` discovery)
  - `resources` / `resources_manage` — Reference-file discovery and tier management
  - `memory` — Memory scratch pad (metadata store, search, and `<memory>` injection)
  - `prompt` — Versioned prompt templates for workflow generation
  - `config` — Shared configuration (e.g. `~/.zig/serve.toml`)
  - `man` — Built-in manpage system
  - `docs` — Built-in conceptual documentation
- **zig-cli** — Thin CLI wrapper that handles argument parsing (via clap) and dispatches commands to `zig-core`.
- **zig-serve** — Companion HTTP API server crate (invoked via `zig serve`) with optional embedded React web UI.

Under the hood, `zig-core` delegates to zag's orchestration primitives (`spawn`, `wait`, `collect`, `pipe`, etc.) to execute workflows.

## Development

```bash
make build          # dev build
make test           # run tests
make lint           # lint (zero warnings)
make fmt            # format
make release        # optimized build
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development workflow.

## Configuration

Configuration file locations and key names:

| Path | Purpose |
|------|---------|
| `~/.zig/serve.toml` | `zig serve` settings — `[server]` section with `port`, `host`, `token`, `tls`, `rate_limit`, `shutdown_timeout` keys |
| `~/.zig/resources/_shared/` | Global resources advertised to every workflow's step agents |
| `~/.zig/resources/<workflow>/` | Per-workflow global resource tier |
| `~/.zig/memory/` | Global and per-workflow memory scratch-pad entries |
| `<git-root>/.zig/resources/` | Project-local resource tier |
| `<git-root>/.zig/memory/` | Project-local memory tier |
| `<git-root>/.zig/workflows/` | Project-local workflow storage directory |

Environment variables:

| Variable | Scope | Description |
|----------|-------|-------------|
| `ZIG_SERVE_TOKEN` | `zig serve` | Bearer token for API authentication (alternative to `--token`) |

Precedence for `zig serve` settings: CLI flag > environment variable > `serve.toml` > built-in default.

Run `zig man serve` for the complete flag and config-file reference.

## Examples

Ready-to-use workflow files are in [`prompts/examples/`](prompts/examples/). Each file demonstrates one of the supported orchestration patterns and can be run directly with `zig run`.

## Troubleshooting

**`zig: command not found`** — make sure `~/.cargo/bin` is in your `PATH` after `cargo install zig-cli`.

**`zag: command not found`** — install zag first; see the [zag install guide](https://github.com/niclaslindstedt/zag#install).

**Workflow fails immediately** — run `zig validate <workflow>` to check the `.zwf` file for structural errors before executing it.

**Build fails with web UI errors** — ensure Node 24+ is installed; the `make build` target builds the embedded React UI first.

## Documentation

Conceptual and reference docs live in [`docs/`](docs/):

- [`docs/zwf.md`](docs/zwf.md) — full `.zwf`/`.zwfz` format specification
- [`docs/patterns.md`](docs/patterns.md) — orchestration pattern guide
- [`docs/variables.md`](docs/variables.md) — variables and data flow
- [`docs/conditions.md`](docs/conditions.md) — conditional step routing
- [`docs/memory.md`](docs/memory.md) — memory scratch pad model

Built-in docs are also available at runtime via `zig docs <topic>` and `zig man <topic>`.

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow, commit conventions, and code standards. Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before participating.

## License

[MIT](LICENSE)
