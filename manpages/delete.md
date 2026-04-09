# zig workflow delete

Delete a `.zug` workflow file.

## Synopsis

```
zig workflow delete <workflow>
```

## Description

Resolves a workflow by name or path and deletes it from disk. Uses the same
resolution logic as `zig run`:

1. Literal path as given (e.g., `./my-workflow.zug`)
2. With `.zug` extension appended (e.g., `my-workflow` -> `my-workflow.zug`)
3. Under `./workflows/` directory (e.g., `workflows/my-workflow`)
4. Under `./workflows/` with `.zug` appended (e.g., `workflows/my-workflow.zug`)

## Arguments

| Argument   | Description                            |
|------------|----------------------------------------|
| `workflow` | Name or path of the workflow to delete |

## Examples

```bash
# Delete by name (resolves to my-workflow.zug or workflows/my-workflow.zug)
zig workflow delete my-workflow

# Delete by explicit path
zig workflow delete workflows/old-pipeline.zug
```

## See Also

- `zig man workflow` — workflow management commands
- `zig man run` — execute a workflow
