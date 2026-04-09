# zig run

Execute a `.zug` workflow file.

## Synopsis

```
zig run <workflow> [prompt]
```

## Description

Parses a `.zug` workflow file, validates it, resolves the step DAG, and
executes each step by delegating to `zag`. Steps are grouped into
parallelizable tiers using topological sorting — steps within the same tier
run concurrently when their dependencies are satisfied.

## Arguments

| Argument   | Description                                              |
|------------|----------------------------------------------------------|
| `workflow` | Name or path of the workflow to run                      |
| `prompt`   | Optional context prompt injected into every workflow step|

## Workflow Resolution

The `workflow` argument is resolved in this order:

1. Literal path as given (e.g., `./my-workflow.zug`)
2. With `.zug` extension appended (e.g., `my-workflow` → `my-workflow.zug`)
3. Under `./workflows/` directory (e.g., `workflows/my-workflow`)
4. Under `./workflows/` with `.zug` appended (e.g., `workflows/my-workflow.zug`)

## Execution Model

1. The workflow file is parsed and validated
2. Steps are sorted into tiers using Kahn's algorithm (topological sort)
3. Each tier's steps run in order; steps with no unmet dependencies can run in parallel
4. Variable substitution (`${var}`) is applied to prompts before execution
5. Step outputs are captured and can be:
   - Injected into dependent steps via `inject_context`
   - Extracted into variables via `saves` selectors
6. Conditions are evaluated to determine whether steps should run or be skipped
7. The `next` field enables loops by jumping back to earlier steps
8. A maximum of 100 loop iterations is enforced to prevent infinite loops

## Failure Handling

Each step can configure its failure behavior with `on_failure`:

| Policy     | Behavior                                        |
|------------|-------------------------------------------------|
| `fail`     | Abort the entire workflow (default)              |
| `continue` | Skip the failed step and continue                |
| `retry`    | Retry the step up to `max_retries` times         |

## Examples

```bash
# Run a workflow by name
zig run code-review

# Run a workflow file directly
zig run ./workflows/deploy.zug

# Run with additional context
zig run code-review "focus on the authentication module"
```

## Prerequisites

- `zag` must be installed and available on PATH

## See Also

- `zig man zug` — the `.zug` file format
- `zig man variables` — variable substitution and data flow
- `zig man conditions` — condition expressions
