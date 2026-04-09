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

| Command              | Description                                          |
|----------------------|------------------------------------------------------|
| `run <workflow>`     | Execute a `.zug` workflow file                       |
| `create [name]`     | Create a new workflow interactively with an AI agent |
| `describe <prompt>`  | Generate a `.zug` file from a natural language prompt|
| `validate <file>`    | Validate a `.zug` workflow file                      |
| `list`               | List available workflows                             |
| `init`               | Initialize a new zig project                         |
| `man [topic]`        | Show manual pages for zig topics                     |

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
zig create my-workflow

# Or describe one in natural language
zig describe "review code, run tests, and generate a report"

# Run a workflow
zig run my-workflow

# Validate a workflow file
zig validate my-workflow.zug
```

## Manpages

Use `zig man` to learn more about specific topics:

```bash
zig man run          # The run command
zig man create       # The create command
zig man zug          # The .zug file format
zig man patterns     # Orchestration patterns
zig man variables    # Variable system and data flow
zig man conditions   # Condition expressions
```

## See Also

- `zig man zug` — the `.zug` workflow format
- `zig man patterns` — orchestration patterns
- zag documentation: https://github.com/niclaslindstedt/zag
