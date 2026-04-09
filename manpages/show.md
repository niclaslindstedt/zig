# zig workflow show

Show detailed information about a workflow.

## Synopsis

```
zig workflow show <workflow>
```

## Description

Resolves a workflow by name or path and displays its full details including
metadata, variables, and steps. Uses the same resolution logic as `zig run`:

1. Literal path as given (e.g., `./my-workflow.zug`)
2. With `.zug` extension appended (e.g., `my-workflow` -> `my-workflow.zug`)
3. Under `./workflows/` directory (e.g., `workflows/my-workflow`)
4. Under `./workflows/` with `.zug` appended (e.g., `workflows/my-workflow.zug`)

## Arguments

| Argument   | Description                          |
|------------|--------------------------------------|
| `workflow` | Name or path of the workflow to show |

## Output Sections

- **Metadata**: workflow name, path, description, and tags
- **Variables**: shared variables with types, defaults, and descriptions
- **Steps**: ordered list of steps with dependencies, providers, and conditions

## Examples

```bash
# Show by name
zig workflow show my-workflow

# Show by explicit path
zig workflow show workflows/deploy.zug
```

## See Also

- `zig man list` — list available workflows
- `zig man workflow` — workflow management commands
- `zig man run` — execute a workflow
