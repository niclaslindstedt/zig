# zig create

Create a new workflow interactively with an AI agent.

## Synopsis

```
zig create [name] [--output <path>] [--pattern <pattern>]
```

## Description

Launches an interactive zag session where an AI agent guides you through
designing a workflow. The agent understands the `.zug` format, zag's
orchestration primitives, and common workflow patterns. It asks clarifying
questions about your process and produces a valid `.zug` file.

After the session completes, the generated file is automatically validated.

## Arguments

| Argument | Description                                              |
|----------|----------------------------------------------------------|
| `name`   | Optional workflow name (used as default output filename) |

## Options

| Option              | Short | Description                                   |
|---------------------|-------|-----------------------------------------------|
| `--output <path>`   | `-o`  | Output file path (defaults to `<name>.zug` or `workflow.zug`) |
| `--pattern <pattern>` | `-p` | Orchestration pattern to guide the agent     |

## Patterns

The `--pattern` flag provides pattern-specific guidance to the agent:

| Pattern                        | Description                                    |
|--------------------------------|------------------------------------------------|
| `sequential`                   | Steps run in order, each feeding the next      |
| `fan-out`                      | Parallel independent steps, then synthesize    |
| `generator-critic`             | Generate, evaluate, iterate until threshold    |
| `coordinator-dispatcher`       | Classify input, route to specialized handlers  |
| `hierarchical-decomposition`   | Break down into sub-tasks, delegate, synthesize|
| `human-in-the-loop`           | Automated steps with human approval gates      |
| `inter-agent-communication`    | Agents collaborate via shared variables        |

## Examples

```bash
# Create a workflow interactively
zig create

# Create a named workflow
zig create code-review

# Create with a specific pattern
zig create deploy --pattern sequential

# Create with a custom output path
zig create my-workflow --output workflows/my-workflow.zug
```

## Prerequisites

- `zag` must be installed and available on PATH

## See Also

- `zig man patterns` — detailed orchestration pattern descriptions
- `zig man zug` — the `.zug` file format
