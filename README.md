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
# Describe a workflow — an agent helps you create a .zwf file
zig describe "review all PRs, run tests, and generate a summary report"

# See what workflows are available
zig workflow list

# Run a workflow
zig run code-review

# Initialize a new zig project
zig init

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

- **Rust 1.85+** (edition 2024) — for building from source
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

## Commands

```
zig run <workflow>              Execute a .zwf workflow file
zig listen [session_id]         Tail a running or completed zig session
zig workflow list               List available workflows
zig workflow show <workflow>    Show details of a workflow
zig workflow create [name]      Create a new workflow interactively with an AI agent
zig workflow delete <workflow>  Delete a workflow file
zig workflow pack <path>        Pack a workflow directory into a .zwfz zip archive
zig resources list              List discovered resource files (global + cwd tiers)
zig resources add <file>        Register a file as a global / cwd / per-workflow resource
zig resources remove <name>     Remove a registered resource
zig resources show <name>       Print the absolute path and contents of a resource
zig resources where             Print the directories the collector searches
zig validate <file>             Validate a .zwf workflow file
zig man [topic]                 Show manual pages for zig topics
zig describe <prompt>           Generate a .zwf file from a prompt (not yet implemented)
zig init                        Initialize a new zig project (not yet implemented)
```

### `zig run`

Execute a `.zwf` workflow. Zig resolves the workflow by name or file path, parses the orchestration steps, and delegates execution to zag.

```bash
zig run code-review
zig run ./workflows/deploy.zwf
zig run code-review "focus on the authentication module"
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

### `zig workflow delete`

Delete a `.zwf` workflow file by name or path.

```bash
zig workflow delete my-workflow
zig workflow delete workflows/old-pipeline.zwf
```

### `zig workflow pack`

Pack a workflow directory into a `.zwf` zip archive. The directory must contain a workflow TOML file and can include prompt files referenced via `system_prompt_file` or `default_file`. The resulting archive works with `zig run` and `zig validate`.

```bash
zig workflow pack ./my-workflow/
zig workflow pack ./my-workflow/ --output custom-name.zwf
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

### `zig validate`

Validate a `.zwf` workflow file for structural correctness without executing it.

```bash
zig validate my-workflow.zwf
```

### `zig man`

Show built-in manual pages for zig commands (zig, run, listen, workflow, resources, describe, validate, serve).

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
| `--output <path>` | `-o` | `workflow create`, `describe`, `workflow pack` | Output file path |
| `--pattern <name>` | `-p` | `workflow create` | Orchestration pattern (sequential, fan-out, generator-critic, etc.) |
| `--latest` | | `listen` | Tail the most recently started session |
| `--active` | | `listen` | Tail the most recently active (still-running) session |
| `--no-resources` | | `run` | Skip the `<resources>` block injected into each step's system prompt |
| `--global` | | `resources add/remove/list` | Target the global tier (`~/.zig/resources/_shared/`) |
| `--cwd` | | `resources add/remove/list` | Target the project tier (`<git-root>/.zig/resources/`) |
| `--workflow <name>` | | `resources` | Restrict to a specific workflow's global tier |

## The `.zwf` / `.zwfz` format

A `.zwf` file is a TOML workflow definition that describes a DAG of AI agent steps with shared variables, conditional routing, and data flow. It is generated by `zig workflow create` and executed by `zig run`. The format captures:

- **Workflow metadata** — Name, description, tags, version, and default provider/model for all steps
- **Roles** — Reusable role definitions with system prompts (inline or loaded from files) that steps can reference
- **Steps** — The ordered (or parallel) operations to perform, each mapping to a zag agent invocation
- **Agent configuration** — Provider (`claude`, `codex`, `gemini`, `copilot`, `ollama`) and model per step, with workflow-level defaults
- **Dependencies** — How steps relate to and depend on each other via `depends_on`
- **Variables & data flow** — Shared state between steps via `${var}` references, `saves` selectors, input bindings (`from = "prompt"`), and variable constraints (required, min/max, patterns, allowed values)
- **Resources** — Reference files advertised to step agents through the system prompt (paths only — agents read them on demand). Inline `resources = [...]` is merged with global (`~/.zig/resources/`) and project (`<git-root>/.zig/resources/`) tiers at run time
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

- **zig-core** — Core library that handles `.zwf` file parsing, workflow validation, execution engine, session tracking, and archive packing. Key modules:
  - `workflow/` — Model, parser, and validator for the `.zwf`/`.zwfz` format
  - `run` — Workflow execution (DAG resolution, step parallelization, streaming output)
  - `listen` — Real-time session tailing
  - `session` — Session lifecycle and log coordination
  - `pack` — Zip archive creation for workflow directories
  - `create` — Interactive workflow creation via zag
  - `manage` — Workflow listing, display, and deletion
  - `prompt` — Prompt templates for workflow generation
  - `man` — Built-in manpage system
- **zig-cli** — Thin CLI wrapper that handles argument parsing (via clap) and dispatches commands to `zig-core`.

Under the hood, `zig-core` delegates to zag's orchestration primitives (`spawn`, `wait`, `collect`, `pipe`, etc.) to execute workflows.

## Development

```bash
make build          # dev build
make test           # run tests
make clippy         # lint (zero warnings)
make fmt            # format
make release        # optimized build
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development workflow.

## License

[MIT](LICENSE)
