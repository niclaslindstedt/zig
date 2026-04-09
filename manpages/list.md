# zig workflow list

List available `.zug` workflow files.

## Synopsis

```
zig workflow list
```

## Description

Discovers and lists all `.zug` workflow files in the current directory and the
`./workflows/` subdirectory. For each workflow, displays its name, file path,
number of steps, and description.

Workflows that fail to parse are still listed, with a `(parse error)` marker
in the description column.

## Output Columns

| Column      | Description                                |
|-------------|--------------------------------------------|
| NAME        | Workflow name from the `[workflow]` section |
| PATH        | File path relative to the current directory |
| STEPS       | Number of steps in the workflow            |
| DESCRIPTION | Short description of the workflow          |

## Examples

```bash
# List all workflows
zig workflow list
```

## See Also

- `zig man show` — show details of a workflow
- `zig man workflow` — workflow management commands
- `zig man create` — create a new workflow
