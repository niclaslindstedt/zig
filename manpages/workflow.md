# zig workflow

Manage workflows — create, delete, list, and show `.zug` workflow files.

## Synopsis

```
zig workflow <subcommand> [options]
```

## Description

The `workflow` command groups operations for managing `.zug` workflow files.
Use its subcommands to create new workflows interactively, delete existing
ones, list available workflows, or show workflow details.

## Subcommands

| Subcommand          | Description                                          |
|---------------------|------------------------------------------------------|
| `create [name]`     | Create a new workflow interactively with an AI agent |
| `delete <workflow>` | Delete a workflow file                               |
| `list`              | List available workflows                             |
| `show <workflow>`   | Show details of a workflow                           |

## Examples

```bash
# Create a workflow interactively
zig workflow create my-workflow

# Create with a specific pattern
zig workflow create deploy --pattern sequential

# Delete a workflow
zig workflow delete old-pipeline

# List all workflows
zig workflow list

# Show workflow details
zig workflow show deploy
```

## See Also

- `zig man create` — create a workflow interactively
- `zig man delete` — delete a workflow file
- `zig man list` — list available workflows
- `zig man show` — show details of a workflow
- `zig man zug` — the `.zug` workflow format
