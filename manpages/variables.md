# Variables

Variables are shared state that flows between workflow steps. They are declared
in the `[vars]` section and referenced in prompts, conditions, and saves.

## Declaring Variables

Variables are declared in the `[vars]` section of a `.zug` file:

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

## Variable Lifecycle

1. Variables are initialized from their `default` values (or empty string)
2. Steps execute in DAG order; `saves` updates variables after each step
3. Variable substitution in prompts uses the current value at execution time
4. Conditions evaluate against the current variable state

## See Also

- `zig man zug` — full `.zug` format reference
- `zig man conditions` — condition expressions using variables
- `zig man run` — workflow execution model
