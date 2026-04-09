# zig workflow

Manage workflows — create and delete `.zug` workflow files.

## Synopsis

```
zig workflow <subcommand> [options]
```

## Description

The `workflow` command groups operations for managing `.zug` workflow files.
Use its subcommands to create new workflows interactively or delete existing
ones.

## Subcommands

| Subcommand          | Description                                          |
|---------------------|------------------------------------------------------|
| `create [name]`     | Create a new workflow interactively with an AI agent |
| `delete <workflow>` | Delete a workflow file                               |

## Examples

```bash
# Create a workflow interactively
zig workflow create my-workflow

# Create with a specific pattern
zig workflow create deploy --pattern sequential

# Delete a workflow
zig workflow delete old-pipeline
```

## See Also

- `zig man create` — create a workflow interactively
- `zig man delete` — delete a workflow file
- `zig man zug` — the `.zug` workflow format
