# zig

Orchestration CLI for AI coding agents — create, share, and run workflows powered by zag.

## Synopsis

```
zig <command> [options]
```

## Description

`zig` lets you create a workflow interactively with an AI agent, capture it as
a shareable `.zwf` file, and replay it anywhere with a single command. It
embeds zag (`zag-agent` + `zag-orch`) in-process for agent orchestration — no
separate `zag` binary is required.

## Commands

| Command              | Description                                              |
|----------------------|----------------------------------------------------------|
| `run <workflow>`     | Execute a `.zwf` workflow file                           |
| `listen [session]`   | Tail a running or completed zig session                  |
| `workflow <command>` | Manage workflows (list, show, create, delete, pack)      |
| `resources <command>`| Manage reference files advertised to step agents         |
| `memory <command>`   | Manage the memory scratch pad for workflows              |
| `validate <file>`    | Validate a `.zwf`/`.zwfz` workflow file                  |
| `serve`              | Start an HTTP API server                                 |
| `man [topic]`        | Show manual pages for zig commands                       |
| `docs [topic]`       | Show conceptual documentation topics                     |

## Global Flags

| Flag        | Short | Description                        |
|-------------|-------|------------------------------------|
| `--debug`   | `-d`  | Enable debug logging               |
| `--quiet`   | `-q`  | Suppress all output except errors  |
| `--version` |       | Print version information          |
| `--help`    | `-h`  | Print help                         |

## Getting Started

```bash
# Create a workflow interactively
zig workflow create my-workflow

# Run a workflow
zig run my-workflow

# Validate a workflow file
zig validate my-workflow.zwf
```

## Manpages

Use `zig man` to learn more about a specific command:

```bash
zig man run          # The run command
zig man listen       # Tail a running or completed zig session
zig man workflow     # Manage workflows (list, show, create, update, delete, pack)
zig man resources    # Manage reference files advertised to agents
zig man memory       # Manage the memory scratch pad for workflows
zig man validate     # Validate a .zwf/.zwfz workflow file
zig man serve        # Start the HTTP API server
```

## Documentation

Use `zig docs` to learn more about a concept:

```bash
zig docs zwf         # The .zwf/.zwfz file format
zig docs patterns    # Orchestration patterns
zig docs variables   # Variable system and data flow
zig docs conditions  # Condition expressions
zig docs memory      # Memory scratch pad and the <memory> block
zig docs storage     # Writable structured working data for workflows
zig docs dry-run     # Preview workflow execution without running zag
```

## See Also

- `zig docs zwf` — the `.zwf`/`.zwfz` workflow format
- `zig docs patterns` — orchestration patterns
- `zig docs storage` — writable structured working data
- zag documentation: https://github.com/niclaslindstedt/zag
