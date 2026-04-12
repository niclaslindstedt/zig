# zig workflow

Manage workflows — list, show, create, and delete `.zug` workflow files.

## Synopsis

```
zig workflow <command> [options]
```

## Commands

| Command              | Description                                          |
|----------------------|------------------------------------------------------|
| `list`               | List available workflows                             |
| `show <workflow>`    | Show details of a workflow                           |
| `create [name]`      | Create a new workflow interactively with an AI agent |
| `delete <workflow>`  | Delete a workflow file                               |
| `pack <dir>`         | Pack a workflow directory into a .zug zip archive    |

## zig workflow list

List all `.zug` workflow files found in the current directory and `./workflows/`.

Displays a table with the workflow name, description, step count, and file path.

```bash
zig workflow list
```

## zig workflow show

Show detailed information about a workflow: metadata, variables, and steps.

```bash
zig workflow show my-workflow
zig workflow show workflows/deploy.zug
```

### Output

- **Name** and **description**
- **Tags** for discovery and filtering
- **Variables** with types and defaults
- **Steps** with dependencies, conditions, and provider info

## zig workflow create

Create a new workflow interactively with an AI agent.

```
zig workflow create [name] [--output <path>] [--pattern <pattern>]
```

Launches an interactive zag session where an AI agent guides you through
designing a workflow. The agent understands the `.zug` format, zag's
orchestration primitives, and common workflow patterns.

### Options

| Option              | Short | Description                                   |
|---------------------|-------|-----------------------------------------------|
| `--output <path>`   | `-o`  | Output file path (defaults to `<name>.zug` or `workflow.zug`) |
| `--pattern <pattern>` | `-p` | Orchestration pattern to guide the agent     |

### Patterns

| Pattern                        | Description                                    |
|--------------------------------|------------------------------------------------|
| `sequential`                   | Steps run in order, each feeding the next      |
| `fan-out`                      | Parallel independent steps, then synthesize    |
| `generator-critic`             | Generate, evaluate, iterate until threshold    |
| `coordinator-dispatcher`       | Classify input, route to specialized handlers  |
| `hierarchical-decomposition`   | Break down into sub-tasks, delegate, synthesize|
| `human-in-the-loop`           | Automated steps with human approval gates      |
| `inter-agent-communication`    | Agents collaborate via shared variables        |

### Prerequisites

- `zag` must be installed and available on PATH

## zig workflow pack

Pack a workflow directory into a `.zug` zip archive for distribution.

```
zig workflow pack <dir> [--output <path>]
```

The directory must contain exactly one TOML workflow file (`.toml` or `.zug`).
All files in the directory are included in the archive, preserving directory
structure. The resulting zip file works directly with `zig run` and `zig validate`.

### Options

| Option              | Short | Description                                   |
|---------------------|-------|-----------------------------------------------|
| `--output <path>`   | `-o`  | Output file path (defaults to `<workflow-name>.zug`) |

### Example

```bash
# Pack a healthcare workflow with 20 prompt files
zig workflow pack examples/healthcare/ -o healthcare.zug

# Validate the packed archive
zig validate healthcare.zug

# Run it
zig run healthcare.zug "I have a headache"
```

## zig workflow delete

Delete a `.zug` workflow file.

```bash
zig workflow delete my-workflow
zig workflow delete workflows/old-workflow.zug
```

## Workflow Discovery

Workflows are discovered in these locations:

1. Current directory — any file with a `.zug` extension
2. `./workflows/` subdirectory — any file with a `.zug` extension
3. `~/.zig/workflows/` global directory — any file with a `.zug` extension

When referencing a workflow by name (e.g., `my-workflow`), zig tries these
paths in order: `my-workflow`, `my-workflow.zug`, `workflows/my-workflow`,
`workflows/my-workflow.zug`, `~/.zig/workflows/my-workflow`,
`~/.zig/workflows/my-workflow.zug`.

## Examples

```bash
# List all workflows
zig workflow list

# Show details of a workflow
zig workflow show code-review

# Create a workflow interactively
zig workflow create my-workflow

# Create with a specific pattern
zig workflow create deploy --pattern sequential

# Delete a workflow
zig workflow delete old-workflow
```

## See Also

- `zig man zug` — the `.zug` file format
- `zig man patterns` — orchestration patterns
