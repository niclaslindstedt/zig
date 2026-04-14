# zig run

Execute a `.zwf` or `.zwfz` workflow file.

## Synopsis

```
zig run <workflow> [prompt] [--no-resources]
```

## Description

Parses a `.zwf` or `.zwfz` workflow file, validates it, resolves the step
DAG, and executes each step by delegating to `zag`. Steps are grouped into
parallelizable tiers using topological sorting — steps within the same tier
run concurrently when their dependencies are satisfied.

## Arguments

| Argument   | Description                                              |
|------------|----------------------------------------------------------|
| `workflow` | Name or path of the workflow to run                      |
| `prompt`   | Optional context prompt injected into every workflow step|

## Flags

| Flag             | Description                                                                                  |
|------------------|----------------------------------------------------------------------------------------------|
| `--no-resources` | Disable the `<resources>` block normally injected into each step's system prompt. Useful when you want a workflow to run with no global / cwd / inline resource advertisements at all. See `zig man resources`. |

## Workflow Resolution

The `workflow` argument is resolved in this order:

1. Literal path as given (e.g., `./my-workflow.zwf`)
2. With `.zwf` extension appended (e.g., `my-workflow` → `my-workflow.zwf`)
3. With `.zwfz` extension appended (e.g., `my-workflow` → `my-workflow.zwfz`)
4. Under `./workflows/` with those three forms
5. Under `~/.zig/workflows/` (the global workflows dir) with those three forms

## Execution Model

1. The workflow file is parsed and validated
2. Variable constraints are checked before execution begins
3. Steps are sorted into tiers using Kahn's algorithm (topological sort)
4. Steps within the same tier run concurrently (auto-parallelized); a single-step tier runs sequentially with live output streaming
5. Race groups within a tier run in parallel; the first to finish wins and the rest are cancelled
6. Variable substitution (`${var}`) is applied to prompts and system prompts before execution
7. Step outputs are captured and can be:
   - Injected into dependent steps via `inject_context`
   - Extracted into variables via `saves` selectors
8. Conditions are evaluated to determine whether steps should run or be skipped
9. The `next` field enables loops by jumping back to earlier steps
10. A maximum of 100 loop iterations is enforced to prevent infinite loops
11. Each run is logged as a zig session under `~/.zig/` — use `zig listen` to tail

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
zig run ./workflows/deploy.zwf

# Run a bundled workflow archive directly
zig run ./workflows/healthcare.zwfz

# Run with additional context
zig run code-review "focus on the authentication module"

# Run without injecting any resource advertisements
zig run code-review --no-resources
```

## Prerequisites

- `zag` must be installed and available on PATH

## See Also

- `zig docs zwf` — the `.zwf`/`.zwfz` file format
- `zig docs variables` — variable substitution and data flow
- `zig docs conditions` — condition expressions
- `zig man resources` — managing reference files for agents
