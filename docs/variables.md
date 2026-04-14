# Variables

Variables are shared state that flows between workflow steps. They are declared
in the `[vars]` section and referenced in prompts, conditions, and saves.

## Declaring Variables

Variables are declared in the `[vars]` section of a `.zwf` file:

```toml
[vars.target]
type = "string"
default = "."
description = "Path to analyze"

[vars.score]
type = "number"

[vars.verbose]
type = "bool"
default = false

[vars.results]
type = "json"
description = "Structured analysis results"
```

## Variable Types

| Type     | Description                | Default value if unset |
|----------|----------------------------|------------------------|
| `string` | Text value                 | `""` (empty string)    |
| `number` | Integer or floating-point  | `""` (empty string)    |
| `bool`   | `true` or `false`          | `""` (empty string)    |
| `json`   | Structured JSON data       | `""` (empty string)    |

Variables without a `default` are initialized to an empty string and must
be set by a preceding step before use.

## Referencing Variables

Use `${var_name}` in step prompts to reference a variable:

```toml
[[step]]
name = "analyze"
prompt = "Analyze the code in ${target}"
```

### Dotted Paths

For JSON variables, use dotted paths to access nested values:

```toml
[[step]]
name = "report"
prompt = "The score was ${result.score} with level ${result.details.level}"
```

At runtime, if the variable's value is valid JSON, the path is traversed
to extract the nested value.

## Setting Variables with Saves

The `saves` field extracts values from a step's output into variables:

```toml
[[step]]
name = "evaluate"
prompt = "Score this code 1-10"
json = true
saves = { score = "$.score", feedback = "$.suggestions" }
```

### Selectors

| Selector            | Description                    |
|---------------------|--------------------------------|
| `"$"`               | The entire output              |
| `"$.field"`         | A top-level JSON field         |
| `"$.nested.field"`  | A nested JSON field            |

The step must produce JSON output for `$.field` selectors to work. Use
`json = true` on the step to request structured output.

## Data Flow Between Steps

There are three mechanisms for passing data between steps:

### 1. Variable Substitution

Variables set by earlier steps are available in later step prompts:

```toml
[[step]]
name = "classify"
json = true
saves = { category = "$.category" }

[[step]]
name = "handle"
prompt = "Handle this ${category} request"
depends_on = ["classify"]
```

### 2. Context Injection

When `inject_context = true`, the raw output of dependency steps is
prepended to the prompt:

```toml
[[step]]
name = "synthesize"
prompt = "Combine all findings"
depends_on = ["review-a", "review-b"]
inject_context = true
```

The injected output appears as:

```
--- Output from 'review-a' ---
<output>

--- Output from 'review-b' ---
<output>

Combine all findings
```

### 3. User Context

An optional prompt passed to `zig run` is injected as `User context:` at
the start of every step's prompt:

```bash
zig run code-review "focus on the authentication module"
```

## File-Backed Defaults

Instead of an inline `default`, a variable can load its default from a file:

```toml
[vars.system_instructions]
type = "string"
default_file = "prompts/instructions.md"
```

`default` and `default_file` are mutually exclusive. File paths are resolved
relative to the `.zwf` file (works for both plain files and zip archives).

## Input Binding

Use `from = "prompt"` to bind the CLI user prompt to a variable:

```toml
[vars.content]
type = "string"
from = "prompt"
required = true
```

Only one variable may use `from = "prompt"`. When set, the value from
`zig run <workflow> "content"` is assigned to this variable instead of being
prepended as "User context:" to every step.

## Constraints

Variables support constraints that are validated before execution begins:

| Constraint       | Applies to | Description                              |
|------------------|------------|------------------------------------------|
| `required`       | All types  | Must be non-empty before execution       |
| `min_length`     | `string`   | Minimum string length                    |
| `max_length`     | `string`   | Maximum string length                    |
| `min`            | `number`   | Minimum numeric value                    |
| `max`            | `number`   | Maximum numeric value                    |
| `pattern`        | `string`   | Regex pattern the value must match       |
| `allowed_values` | All types  | Restrict to specific values              |

Default values are also validated at parse time (`zig validate`).

## Variable Lifecycle

1. Variables are initialized from their `default` or `default_file` values (or empty string)
2. The `from = "prompt"` binding assigns the user prompt to the bound variable
3. Constraints are validated before execution begins
4. Steps execute in DAG order; `saves` updates variables after each step
5. Variable substitution in prompts and system prompts uses the current value at execution time
6. Conditions evaluate against the current variable state

## See Also

- `zig docs zwf` — full `.zwf`/`.zwfz` format reference
- `zig docs conditions` — condition expressions using variables
- `zig man run` — workflow execution model
