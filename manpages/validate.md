# zig validate

Validate a `.zug` workflow file for structural correctness.

## Synopsis

```
zig validate <workflow>
```

## Description

Parses and validates a `.zug` workflow file without executing it. Reports
any errors found during validation.

## Arguments

| Argument   | Description                          |
|------------|--------------------------------------|
| `workflow` | Path to the `.zug` file to validate  |

## Validation Checks

The validator performs the following checks:

- **Step existence** — at least one step must be defined
- **Unique step names** — no duplicate step names
- **Dependency references** — every `depends_on` entry must reference an existing step
- **No self-dependencies** — a step cannot depend on itself
- **No dependency cycles** — the step graph must be a DAG (detected via DFS)
- **Next references** — the `next` field must reference an existing step
- **Variable references** — every `${var}` in prompts must refer to a declared variable
- **Saves references** — variables in `saves` must be declared in `[vars]`
- **Condition references** — variables in conditions must be declared

## Exit Codes

| Code | Meaning                            |
|------|------------------------------------|
| `0`  | Workflow is valid                  |
| `1`  | Workflow has validation errors     |

## Examples

```bash
# Validate a workflow file
zig validate my-workflow.zug

# Validate and check output
zig validate workflows/deploy.zug && echo "Valid!"
```

## See Also

- `zig man zug` — the `.zug` file format
- `zig man variables` — variable declarations and references
