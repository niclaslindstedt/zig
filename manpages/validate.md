# zig validate

Validate a `.zwf` workflow file for structural correctness.

## Synopsis

```
zig validate <workflow>
```

## Description

Parses and validates a `.zwf` workflow file without executing it. Reports
any errors found during validation.

## Arguments

| Argument   | Description                          |
|------------|--------------------------------------|
| `workflow` | Path to the `.zwf` file to validate  |

## Validation Checks

The validator performs the following checks:

- **Step existence** — at least one step must be defined
- **Unique step names** — no duplicate step names
- **Dependency references** — every `depends_on` entry must reference an existing step
- **No self-dependencies** — a step cannot depend on itself
- **No dependency cycles** — the step graph must be a DAG (detected via DFS)
- **Next references** — the `next` field must reference an existing step
- **Variable references** — every `${var}` in prompts and system prompts must refer to a declared variable
- **Saves references** — variables in `saves` must be declared in `[vars]`
- **Condition references** — variables in conditions must be declared
- **Role/system_prompt exclusion** — a step cannot set both `role` and `system_prompt`
- **Role references** — static role names must exist in `[roles]`; dynamic `${var}` role refs must reference declared variables
- **Role definitions** — `system_prompt` and `system_prompt_file` are mutually exclusive per role
- **Variable constraints** — type-specific constraints (`min_length`, `max_length`, `min`, `max`, `pattern`, `allowed_values`) must be appropriate for the variable type; ranges must be consistent; regex patterns must compile; default values must satisfy constraints
- **Input binding** — only one variable may use `from = "prompt"`; `default` and `default_file` are mutually exclusive
- **retry_model** — requires `on_failure = "retry"`
- **mcp_config** — requires the `claude` provider (or no explicit provider)
- **Output format** — must be one of: `text`, `json`, `json-pretty`, `stream-json`, `native-json`
- **Review fields** — `uncommitted`, `base`, `commit`, `title` require `command = "review"`
- **Plan fields** — `plan_output`, `instructions` require `command = "plan"`
- **Pipe/collect/summary** — require `depends_on` (they operate on prior session outputs)
- **Race groups** — steps in the same `race_group` must not depend on each other
- **Storage paths** — every `[storage.*]` entry must have a non-empty `path`
- **Storage file hints** — `files` hints are only valid for `type = "folder"` entries; file hint names must be bare filenames (no path separators)
- **Storage scoping** — step `storage` entries must reference names declared in `[storage.*]`

## Exit Codes

| Code | Meaning                            |
|------|------------------------------------|
| `0`  | Workflow is valid                  |
| `1`  | Workflow has validation errors     |

## Examples

```bash
# Validate a workflow file
zig validate my-workflow.zwf

# Validate and check output
zig validate workflows/deploy.zwf && echo "Valid!"
```

## See Also

- `zig docs zwf` — the `.zwf`/`.zwfz` file format
- `zig docs variables` — variable declarations and references
- `zig docs storage` — writable structured working data for workflows
