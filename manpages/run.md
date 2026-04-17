# zig run

Execute a `.zwf` or `.zwfz` workflow file.

## Synopsis

```
zig run <workflow> [prompt] [--no-resources] [--no-memory] [--no-storage]
                              [--dry-run] [--format <text|json>]
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
| `--no-memory`    | Disable the `<memory>` block normally injected into each step's system prompt. Suppresses all tiers of the memory scratch pad for this invocation. See `zig man memory`. |
| `--no-storage`   | Disable the `<storage>` block normally injected into each step's system prompt. Storage directories are not created and no storage listings are shown to agents. See `zig docs storage`. |
| `--dry-run`      | Preview what the workflow would do without invoking `zag`. Prints each step's resolved prompt, system prompt, condition outcome, and the exact `zag` command-line that would be spawned. No sessions are recorded, no storage directories are created, and `zag` itself is not required on PATH. See `zig docs dry-run`. |
| `--format <fmt>` | Output format for `--dry-run`: `text` (default, human-readable) or `json` (stable schema suitable for piping into `jq`). Only meaningful with `--dry-run`. |

## Workflow Resolution

The `workflow` argument is resolved in this order:

1. Literal path as given (e.g., `./my-workflow.zwf`)
2. With `.zwf` extension appended (e.g., `my-workflow` → `my-workflow.zwf`)
3. With `.zwfz` extension appended (e.g., `my-workflow` → `my-workflow.zwfz`)
4. Under the project-local `.zig/workflows/` directory (walking up from the
   current directory to the git root) with those three forms
5. Under `~/.zig/workflows/` (the global workflows dir) with those three forms

## Execution Model

1. The workflow file is parsed and validated
2. Storage directories and files declared in `[storage.*]` are created on demand (idempotent — existing data is preserved)
3. Variable constraints are checked before execution begins
4. Steps are sorted into tiers using Kahn's algorithm (topological sort)
5. Steps within the same tier run concurrently (auto-parallelized); a single-step tier runs sequentially with live output streaming
6. Race groups within a tier run in parallel; the first to finish wins and the rest are cancelled
7. Variable substitution (`${var}`) is applied to prompts and system prompts before execution
8. A `<storage>` block listing in-scope storage entries is prepended to each step's system prompt (contents are refreshed per step so later steps see files written by earlier ones)
9. Step outputs are captured and can be:
   - Injected into dependent steps via `inject_context`
   - Extracted into variables via `saves` selectors
10. Conditions are evaluated to determine whether steps should run or be skipped
11. The `next` field enables loops by jumping back to earlier steps
12. A maximum of 100 loop iterations is enforced to prevent infinite loops
13. Each run is logged as a zig session under `~/.zig/` — use `zig listen` to tail

## Failure Handling

Each step can configure its failure behavior with `on_failure`:

| Policy     | Behavior                                        |
|------------|-------------------------------------------------|
| `fail`     | Abort the entire workflow (default)              |
| `continue` | Skip the failed step and continue                |
| `retry`    | Retry the step up to `max_retries` times         |

## Storage

When a workflow declares `[storage.*]` entries, zig creates the corresponding
directories and files under `<cwd>/.zig/` before the first step executes
(absolute paths like `~/books/current` are used verbatim). Creation is
idempotent — existing data from previous runs is preserved.

Each step receives a `<storage>` block in its system prompt listing the entries
it can see. Steps can narrow their view with `storage = ["name", ...]`; omitting
the field exposes all entries, and `storage = []` suppresses the block entirely.
The `<storage>` block includes a live `<contents>` listing refreshed per step,
so later steps see files that earlier steps just wrote. Agents read and write
storage with their normal file tools — zig does not interpose.

See `zig docs storage` for the full model (types, path resolution, hints, and
step-level scoping).

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

# Run without injecting memory scratch pad entries
zig run code-review --no-memory

# Run without injecting storage block or creating storage directories
zig run code-review --no-storage

# Preview what the workflow would do, without invoking zag
zig run code-review --dry-run

# Emit a stable JSON plan for tooling / CI
zig run code-review --dry-run --format json | jq '.tiers[].steps[].name'
```

## Prerequisites

- `zag` must be installed and available on PATH (not required for `--dry-run`)

## See Also

- `zig docs zwf` — the `.zwf`/`.zwfz` file format
- `zig docs variables` — variable substitution and data flow
- `zig docs conditions` — condition expressions
- `zig man resources` — managing reference files for agents
- `zig man memory` — managing the memory scratch pad for workflows
- `zig docs storage` — writable structured working data for workflows
- `zig docs dry-run` — preview workflow execution without running `zag`
