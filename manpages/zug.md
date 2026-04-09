# The .zug Format

A `.zug` file is a TOML file that describes a workflow — a DAG of AI agent
steps with shared variables, conditional routing, and data flow. Zig compiles
this into zag orchestration commands at execution time.

## File Structure

```toml
[workflow]
name = "my-workflow"           # Required: unique workflow name
description = "What it does"   # Optional: human-readable description
tags = ["tag1", "tag2"]        # Optional: discovery/filtering tags

[vars.target]                  # Variable declarations
type = "string"
default = "."
description = "Path to analyze"

[[step]]                       # Step definitions (one or more)
name = "analyze"
prompt = "Analyze ${target}"
```

## Sections

### `[workflow]` — Metadata

| Field         | Required | Description                        |
|---------------|----------|------------------------------------|
| `name`        | Yes      | Unique workflow name               |
| `description` | No       | Human-readable description         |
| `tags`        | No       | Tags for discovery and filtering   |

### `[vars.<name>]` — Variables

Variables are shared state between steps. They can be referenced in prompts
via `${var_name}` and updated by steps via the `saves` field.

| Field         | Required | Description                             |
|---------------|----------|-----------------------------------------|
| `type`        | Yes      | `string`, `number`, `bool`, or `json`   |
| `default`     | No       | Default value (TOML literal)            |
| `description` | No       | Human-readable purpose                  |

### `[[step]]` — Steps

Each step is one zag agent invocation.

| Field            | Required | Default | Description                              |
|------------------|----------|---------|------------------------------------------|
| `name`           | Yes      |         | Unique step identifier                   |
| `prompt`         | Yes      |         | Prompt template (`${var}` refs allowed)  |
| `provider`       | No       |         | Zag provider (claude, codex, gemini, copilot, ollama) |
| `model`          | No       |         | Model name or size alias (small, medium, large) |
| `depends_on`     | No       | `[]`    | Steps that must complete first           |
| `inject_context` | No       | `false` | Inject dependency outputs into prompt    |
| `condition`      | No       |         | Only run if expression evaluates to true |
| `json`           | No       | `false` | Request structured JSON output           |
| `json_schema`    | No       |         | JSON schema to validate output           |
| `saves`          | No       | `{}`    | Extract values from output into variables|
| `timeout`        | No       |         | Step timeout (e.g., `5m`, `30s`, `1h`)   |
| `tags`           | No       | `[]`    | Zag session tags                         |
| `on_failure`     | No       | `fail`  | `fail`, `continue`, or `retry`           |
| `max_retries`    | No       |         | Retry limit (when `on_failure = "retry"`)|
| `next`           | No       |         | Explicit next step (enables loops)       |
| `system_prompt`  | No       |         | Agent system prompt override             |
| `max_turns`      | No       |         | Maximum agentic turns                    |

## Saves Selectors

The `saves` field maps variable names to JSONPath-like selectors:

| Selector         | Description                    |
|------------------|--------------------------------|
| `"$"`            | The entire output              |
| `"$.field"`      | A top-level JSON field         |
| `"$.nested.field"` | A nested JSON field         |

## Minimal Example

```toml
[workflow]
name = "hello"

[[step]]
name = "greet"
prompt = "Say hello to the user"
```

## Full Example

```toml
[workflow]
name = "code-review"
description = "Multi-perspective code review with synthesis"
tags = ["review", "quality"]

[vars.target]
type = "string"
default = "."
description = "Path to review"

[vars.score]
type = "number"

[vars.threshold]
type = "number"
default = 8

[vars.feedback]
type = "string"
default = ""

[[step]]
name = "analyze"
prompt = "Analyze the code structure of ${target}"
provider = "claude"
model = "sonnet"

[[step]]
name = "security-review"
prompt = "Review for security vulnerabilities"
depends_on = ["analyze"]
inject_context = true

[[step]]
name = "perf-review"
prompt = "Review for performance issues"
depends_on = ["analyze"]
inject_context = true

[[step]]
name = "synthesize"
prompt = "Create a unified code review report"
depends_on = ["security-review", "perf-review"]
inject_context = true

[[step]]
name = "quality-gate"
prompt = "Score this report 1-10"
depends_on = ["synthesize"]
inject_context = true
json = true
saves = { score = "$.score", feedback = "$.suggestions" }

[[step]]
name = "refine"
prompt = "Improve based on: ${feedback}"
depends_on = ["quality-gate"]
condition = "score < threshold"
next = "quality-gate"
on_failure = "retry"
max_retries = 2
```

## See Also

- `zig man variables` — variable references and data flow
- `zig man conditions` — condition expression syntax
- `zig man patterns` — common orchestration patterns
- `zig man run` — executing workflows
