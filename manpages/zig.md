# zig

Orchestration CLI for AI coding agents — describe, share, and run workflows powered by zag.

## Synopsis

```
zig <command> [options]
```

## Description

`zig` lets you describe a workflow in natural language, capture it as a
shareable `.zug` file, and replay it anywhere with a single command. It uses
zag behind the scenes for agent orchestration.

## Commands

| Command              | Description                                              |
|----------------------|----------------------------------------------------------|
| `run <workflow>`     | Execute a `.zug` workflow file                           |
| `listen [session]`   | Tail a running or completed zig session                  |
| `workflow <command>` | Manage workflows (list, show, create, delete, pack)      |
| `resources <command>`| Manage reference files advertised to step agents         |
| `memory <command>`   | Manage the memory scratch pad for workflows              |
| `describe <prompt>`  | Generate a `.zug` file from a natural language prompt    |
| `validate <file>`    | Validate a `.zug` workflow file                          |
| `serve`              | Start an HTTP API server                                 |
| `init`               | Initialize a new zig project                             |
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

# Or describe one in natural language
zig describe "review code, run tests, and generate a report"

# Run a workflow
zig run my-workflow

# Validate a workflow file
zig validate my-workflow.zug
```

## Manpages

Use `zig man` to learn more about a specific command:

```bash
zig man run          # The run command
zig man listen       # Tail a running or completed zig session
zig man workflow     # Manage workflows (list, show, create, delete, pack)
zig man resources    # Manage reference files advertised to agents
zig man serve        # Start the HTTP API server
```

## Documentation

Use `zig docs` to learn more about a concept:

```bash
zig docs zug         # The .zug file format
zig docs patterns    # Orchestration patterns
zig docs variables   # Variable system and data flow
zig docs conditions  # Condition expressions
```

## See Also

- `zig docs zug` — the `.zug` workflow format
- `zig docs patterns` — orchestration patterns
- zag documentation: https://github.com/niclaslindstedt/zag
