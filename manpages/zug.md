# The .zug Format

A `.zug` file is a TOML file that describes a workflow — a DAG of AI agent
steps with shared variables, conditional routing, and data flow. Zig compiles
this into zag orchestration commands at execution time.

A `.zug` file can also be a **zip archive** containing one TOML workflow file
and associated resource files (prompt files, schemas, etc.). Use
`zig workflow pack` to create distributable zip archives.

## File Structure

```toml
[workflow]
name = "my-workflow"           # Required: unique workflow name
description = "What it does"   # Optional: human-readable description
tags = ["tag1", "tag2"]        # Optional: discovery/filtering tags
version = "1.0.0"              # Optional: workflow version
provider = "claude"            # Optional: default provider for all steps
model = "sonnet"               # Optional: default model for all steps

[roles.analyst]                # Reusable role definitions
system_prompt = "You are an analyst."

[vars.target]                  # Variable declarations
type = "string"
default = "."
description = "Path to analyze"

[[step]]                       # Step definitions (one or more)
name = "analyze"
prompt = "Analyze ${target}"
role = "analyst"
```

## Sections

### `[workflow]` — Metadata

| Field         | Required | Description                                                             |
|---------------|----------|-------------------------------------------------------------------------|
| `name`        | Yes      | Unique workflow name                                                    |
| `description` | No       | Human-readable description                                              |
| `tags`        | No       | Tags for discovery and filtering                                        |
| `version`     | No       | Workflow version string (e.g., "1.0.0")                                 |
| `provider`    | No       | Default provider for all steps (claude, codex, gemini, copilot, ollama) |
| `model`       | No       | Default model for all steps (steps can override)                        |
| `resources`   | No       | Reference files advertised to every step (see Resources below)          |

When `provider` or `model` is set on the workflow, every step inherits it as a
default. A step can override the workflow-level value by setting its own
`provider` or `model` field.

### `[roles.<name>]` — Roles

Roles define reusable system prompts that shape agent behavior. Each role
provides its prompt inline or loads it from an external file. Steps reference
roles by name via the `role` field.

| Field               | Required | Description                                    |
|---------------------|----------|------------------------------------------------|
| `system_prompt`     | No*      | Inline system prompt (`${var}` refs allowed)   |
| `system_prompt_file`| No*      | Path to file containing the system prompt      |

\* One of `system_prompt` or `system_prompt_file` should be set. They are
mutually exclusive.

```toml
[roles.doctor]
system_prompt = "You are a board-certified physician."

[roles.nurse]
system_prompt_file = "prompts/nurse.md"       # Loaded relative to the .zug file
```

Steps reference roles statically or dynamically:

```toml
[[step]]
name = "examine"
prompt = "Examine the patient"
role = "doctor"                               # Static reference

[[step]]
name = "specialist"
prompt = "Specialist examination"
role = "${specialist_type}"                   # Dynamic — resolved at runtime
```

### `[vars.<name>]` — Variables

Variables are shared state between steps. They can be referenced in prompts
and system prompts via `${var_name}` and updated by steps via the `saves` field.

| Field            | Required | Description                                         |
|------------------|----------|-----------------------------------------------------|
| `type`           | Yes      | `string`, `number`, `bool`, or `json`               |
| `default`        | No       | Default value (TOML literal)                        |
| `default_file`   | No       | Path to file whose contents become the default      |
| `description`    | No       | Human-readable purpose                              |
| `from`           | No       | Input source binding (`"prompt"`)                   |
| `required`       | No       | Must be non-empty before execution                  |
| `min_length`     | No       | Minimum string length (string vars only)            |
| `max_length`     | No       | Maximum string length (string vars only)            |
| `min`            | No       | Minimum numeric value (number vars only)            |
| `max`            | No       | Maximum numeric value (number vars only)            |
| `pattern`        | No       | Regex pattern to match (string vars only)           |
| `allowed_values` | No       | Restrict to specific values (TOML array)            |

#### Input Binding

Use `from = "prompt"` to bind the CLI user prompt to a variable. Only one
variable may use this. When set, the value from `zig run <workflow> "content"`
is assigned to the variable instead of being prepended as "User context:".

#### Constraints

Constraints are validated before step execution. If a constraint fails, zig
prints a clear error and aborts. Default values are also validated at parse
time (`zig validate`).

```toml
[vars.content]
type = "string"
from = "prompt"
required = true
min_length = 10
max_length = 5000
pattern = "^[A-Z]"
description = "Content to process (must start with uppercase)"

[vars.priority]
type = "string"
default = "medium"
allowed_values = ["low", "medium", "high"]

[vars.score]
type = "number"
min = 0.0
max = 100.0
```

### `[[step]]` — Steps

Each step is one zag agent invocation.

#### Core Fields

| Field            | Required | Default | Description                              |
|------------------|----------|---------|------------------------------------------|
| `name`           | Yes      |         | Unique step identifier                   |
| `prompt`         | Yes      |         | Prompt template (`${var}` refs allowed)  |
| `description`    | No       | `""`    | Human-readable description of this step  |
| `provider`       | No       |         | Zag provider — overrides workflow-level default |
| `model`          | No       |         | Model name or size alias — overrides workflow-level default |
| `depends_on`     | No       | `[]`    | Steps that must complete first           |
| `inject_context` | No       | `false` | Inject dependency outputs into prompt    |
| `condition`      | No       |         | Only run if expression evaluates to true |
| `system_prompt`  | No       |         | Agent system prompt override (`${var}` refs allowed) |
| `role`           | No       |         | Role name or `${var}` reference (mutually exclusive with `system_prompt`) |
| `max_turns`      | No       |         | Maximum agentic turns                    |

#### Output and Data Flow

| Field            | Required | Default | Description                              |
|------------------|----------|---------|------------------------------------------|
| `json`           | No       | `false` | Request structured JSON output           |
| `json_schema`    | No       |         | JSON schema to validate output           |
| `output`         | No       |         | Output format: `text`, `json`, `json-pretty`, `stream-json`, `native-json` |
| `saves`          | No       | `{}`    | Extract values from output into variables|

#### Failure and Retry

| Field            | Required | Default | Description                              |
|------------------|----------|---------|------------------------------------------|
| `on_failure`     | No       | `fail`  | `fail`, `continue`, or `retry`           |
| `max_retries`    | No       |         | Retry limit (when `on_failure = "retry"`)|
| `retry_model`    | No       |         | Model to escalate to on retry            |
| `next`           | No       |         | Explicit next step (enables loops)       |
| `timeout`        | No       |         | Step timeout (e.g., `5m`, `30s`, `1h`)   |
| `tags`           | No       | `[]`    | Zag session tags                         |

#### Execution Environment

| Field            | Required | Default | Description                              |
|------------------|----------|---------|------------------------------------------|
| `interactive`    | No       | `false` | Spawn a long-lived interactive session   |
| `auto_approve`   | No       | `false` | Auto-approve all agent actions           |
| `root`           | No       |         | Working directory override               |
| `add_dirs`       | No       | `[]`    | Additional directories in agent scope    |
| `env`            | No       | `{}`    | Per-step environment variables           |
| `files`          | No       | `[]`    | Files to attach to the agent prompt      |
| `resources`      | No       | `[]`    | Reference files advertised to this step's agent (see Resources below) |

#### Context Injection

| Field            | Required | Default | Description                              |
|------------------|----------|---------|------------------------------------------|
| `context`        | No       | `[]`    | Session IDs to inject as context         |
| `plan`           | No       |         | Path to a plan file to prepend as context|
| `mcp_config`     | No       |         | MCP configuration (Claude provider only) |

#### Isolation

| Field            | Required | Default | Description                              |
|------------------|----------|---------|------------------------------------------|
| `worktree`       | No       | `false` | Run in an isolated git worktree          |
| `sandbox`        | No       |         | Docker sandbox name                      |
| `race_group`     | No       |         | Race group — first to finish wins, rest cancelled |

#### Command Step Types

| Field            | Required | Default | Description                              |
|------------------|----------|---------|------------------------------------------|
| `command`        | No       |         | Zag command: `review`, `plan`, `pipe`, `collect`, `summary` |
| `uncommitted`    | No       | `false` | Review uncommitted changes (`command = "review"`) |
| `base`           | No       |         | Base branch for review diff (`command = "review"`) |
| `commit`         | No       |         | Specific commit to review (`command = "review"`) |
| `title`          | No       |         | Review title (`command = "review"`)      |
| `plan_output`    | No       |         | Output path for plan (`command = "plan"`)|
| `instructions`   | No       |         | Additional plan instructions (`command = "plan"`) |

## Resources

`resources` is a list of reference files that the workflow tells the agent
about — paths only, never inlined content. The agent reads them on demand with
its file tools when the user's request touches them. Use this for things like
CVs, style guides, and reference docs that you want available *if needed*
without burning context up front.

Each entry is either a bare path string (relative to the `.zug` file) or a
detailed table:

```toml
[workflow]
name = "cover-letter"
resources = [
  "./style-guide.md",                                # bare form
  { path = "./cv.md", name = "cv", description = "Candidate CV", required = true },
]

[[step]]
name = "draft"
prompt = "Write a cover letter for the attached job posting."
resources = [{ path = "./templates/cover-letter.md", description = "House template" }]
```

| Field         | Required | Description                                                                          |
|---------------|----------|--------------------------------------------------------------------------------------|
| `path`        | Yes      | Path relative to the `.zug` file                                                     |
| `name`        | No       | Display name (defaults to the file's basename)                                       |
| `description` | No       | Description rendered next to the path in the system prompt                           |
| `required`    | No       | When true, missing files cause the run to fail (instead of being skipped + warning)  |

Inline `resources` are merged with files discovered in the global tiers
(`~/.zig/resources/_shared/`, `~/.zig/resources/<workflow-name>/`) and the
project tier (`<git-root>/.zig/resources/`). See `zig man resources` for the
full collection model and the `zig resources` management commands.

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

## Zip Archives

A `.zug` file can be a zip archive containing a TOML workflow and associated
files (prompt files, schemas, defaults). This enables distributing self-contained
workflow packages.

### Creating Archives

```bash
# Pack a directory into a .zug zip archive
zig workflow pack examples/healthcare/ -o healthcare.zug
```

The directory must contain exactly one TOML workflow file. All files in the
directory are included in the archive.

### Using Archives

Archives work transparently with `zig run` and `zig validate`:

```bash
zig validate healthcare.zug
zig run healthcare.zug "I have chest pain"
```

File paths (`system_prompt_file`, `default_file`) are resolved relative to the
TOML file — this works identically for both plain files and zip archives.

## See Also

- `zig man variables` — variable references and data flow
- `zig man conditions` — condition expression syntax
- `zig man patterns` — common orchestration patterns
- `zig man run` — executing workflows
