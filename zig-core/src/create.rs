use std::path::Path;
use std::process::Command;

use crate::error::ZigError;

/// The `.zug` format specification given to the agent so it can produce valid workflows.
const ZUG_FORMAT_SPEC: &str = r#"
# .zug Workflow Format Specification

A `.zug` file is a TOML file that describes a workflow — a DAG of AI agent steps
with shared variables, conditional routing, and data flow. Zig compiles this
into zag orchestration commands at execution time.

## Structure

```toml
[workflow]
name = "my-workflow"           # Required: unique workflow name
description = "What it does"   # Optional: human-readable description
tags = ["tag1", "tag2"]        # Optional: discovery/filtering tags

# Variables: shared state between steps.
# Agents read/write these via the zig MCP server during execution.
# Types: string, number, bool, json
[vars]
target = { type = "string", default = ".", description = "Path to analyze" }
threshold = { type = "number", default = 8 }
status = { type = "string" }

# Steps: each step is one zag agent invocation.
# Steps form a DAG via depends_on and can branch via condition.

[[step]]
name = "analyze"                           # Required: unique step name
prompt = "Analyze ${target}"               # Required: prompt template (${var} refs)
provider = "claude"                        # Optional: zag provider
model = "sonnet"                           # Optional: model or size alias
depends_on = ["previous-step"]             # Optional: wait for these steps
inject_context = true                      # Optional: inject dependency outputs
condition = "score < threshold"            # Optional: only run if expression is true
json = true                                # Optional: request JSON output
json_schema = '{"type":"object"}'          # Optional: validate JSON output
saves = { score = "$.score" }              # Optional: extract values into variables
timeout = "5m"                             # Optional: step timeout
tags = ["review"]                          # Optional: zag session tags
on_failure = "retry"                       # Optional: fail (default), continue, retry
max_retries = 3                            # Optional: retry limit
next = "quality-gate"                      # Optional: explicit next step (for loops)
system_prompt = "You are a security expert" # Optional: agent system prompt
max_turns = 10                             # Optional: max agentic turns
```

## Variable References

Use `${var_name}` in prompts to reference variables. Dotted paths like
`${result.score}` access nested JSON values.

## Conditions

Simple expressions evaluated against variable state:
- `score < 8` — numeric comparison
- `status == "done"` — string equality
- `retries < max_retries` — variable-to-variable comparison
- `approved` — truthy check

## Data Flow

Steps communicate through:
1. **inject_context** — dependency outputs are prepended to the prompt
2. **saves** — extract values from step output into variables
3. **Variable references** — `${var}` in prompts are resolved at runtime

## Saves Selectors

The `saves` field maps variable names to JSONPath-like selectors:
- `"$"` — the entire output (text or JSON)
- `"$.field"` — a top-level JSON field
- `"$.nested.field"` — a nested JSON field

## Execution Model

1. Zig resolves the step DAG and identifies parallelizable groups
2. Steps with no unmet dependencies run in parallel via `zag spawn`
3. When a step completes, zig evaluates conditions on dependent steps
4. Variables are updated from `saves` selectors
5. The cycle repeats until all reachable steps complete

## Common Patterns

### Sequential Pipeline
```toml
[[step]]
name = "parse"
prompt = "Parse the API logs"

[[step]]
name = "analyze"
prompt = "Analyze the parsed data"
depends_on = ["parse"]
inject_context = true

[[step]]
name = "report"
prompt = "Write a report"
depends_on = ["analyze"]
inject_context = true
```

### Fan-Out / Gather
```toml
[[step]]
name = "security-review"
prompt = "Review for security issues"
tags = ["review"]

[[step]]
name = "perf-review"
prompt = "Review for performance"
tags = ["review"]

[[step]]
name = "synthesize"
prompt = "Combine all review findings"
depends_on = ["security-review", "perf-review"]
inject_context = true
```

### Generator / Critic Loop
```toml
[vars]
score = { type = "number", default = 0 }
threshold = { type = "number", default = 8 }
feedback = { type = "string", default = "" }

[[step]]
name = "generate"
prompt = "Write the implementation. Previous feedback: ${feedback}"

[[step]]
name = "critique"
prompt = "Score this code 1-10"
depends_on = ["generate"]
inject_context = true
json = true
saves = { score = "$.score", feedback = "$.suggestions" }

[[step]]
name = "refine"
prompt = "Improve based on: ${feedback}"
depends_on = ["critique"]
condition = "score < threshold"
next = "critique"
```

### Coordinator / Dispatcher
```toml
[vars]
complexity = { type = "string" }

[[step]]
name = "classify"
prompt = "Classify this task as simple, moderate, or complex"
json = true
saves = { complexity = "$.classification" }

[[step]]
name = "simple-handler"
prompt = "Handle this simple task"
depends_on = ["classify"]
condition = "complexity == \"simple\""
model = "small"

[[step]]
name = "complex-handler"
prompt = "Handle this complex task"
depends_on = ["classify"]
condition = "complexity == \"complex\""
model = "large"
```
"#;

/// The system prompt for the workflow creation agent.
///
/// This prompt gives the agent full context about zag capabilities and the .zug
/// format, enabling it to guide the user through interactive workflow design.
fn build_system_prompt(zag_help: &str, zag_orch: &str) -> String {
    format!(
        r#"You are a workflow design assistant for zig, a structured workflow orchestration tool built on top of zag.

Your job is to help the user design a workflow by understanding their process and producing a valid `.zug` file.

## How to help the user

1. Ask the user to describe the process they want to automate
2. Identify the key steps, decision points, and data flow
3. Determine which agents/providers are best suited for each step
4. Design the variable schema for shared state between steps
5. Map the process to the appropriate orchestration pattern(s)
6. Generate the complete `.zug` file

## Orchestration Patterns

Choose the right pattern based on the user's needs:
- **Sequential Pipeline** — Steps run in order, each feeding the next
- **Fan-Out / Gather** — Parallel independent steps, then synthesize
- **Generator / Critic** — Generate, evaluate, iterate until quality threshold
- **Coordinator / Dispatcher** — Classify input, route to specialized handlers
- **Hierarchical Decomposition** — Break down into sub-tasks, delegate, synthesize
- **Human-in-the-Loop** — Automated steps with human approval gates
- **Inter-Agent Communication** — Agents collaborate via shared variables

## .zug Format Reference
{ZUG_FORMAT_SPEC}

## Zag CLI Reference
{zag_help}

## Zag Orchestration Patterns
{zag_orch}

## Important Guidelines

- Always produce VALID TOML. Test your output mentally before presenting it.
- Every variable referenced in a prompt (${{var}}) must be declared in [vars].
- Every step referenced in depends_on must exist.
- Avoid dependency cycles (A depends on B depends on A).
- Use inject_context = true when a step needs its dependency's output.
- Use saves + json when a step needs to extract structured data for routing.
- Choose providers/models appropriate to the task complexity.
- Present the final .zug file in a ```toml code block.
- Keep prompts specific and actionable — vague prompts produce vague results.
- Start simple. Ask clarifying questions before adding complexity."#
    )
}

/// Attempt to capture zag CLI reference text via `zag --help-agent`.
fn get_zag_help() -> String {
    Command::new("zag")
        .arg("--help-agent")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "(zag --help-agent not available — zag may not be installed)".into())
}

/// Attempt to capture zag orchestration reference via `zag man orchestration`.
fn get_zag_orch_reference() -> String {
    Command::new("zag")
        .args(["man", "orchestration"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| {
            "(zag man orchestration not available — zag may not be installed)".into()
        })
}

/// Launch an interactive zag session for workflow creation.
///
/// The agent is given full context about zag and the .zug format, and guides
/// the user through designing their workflow.
pub fn run_create(name: Option<&str>, output: Option<&str>) -> Result<(), ZigError> {
    // Check that zag is available
    let zag_available = Command::new("zag")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());

    if !zag_available {
        return Err(ZigError::Zag(
            "zag is not installed or not in PATH. Install it from https://github.com/niclaslindstedt/zag".into(),
        ));
    }

    let zag_help = get_zag_help();
    let zag_orch = get_zag_orch_reference();
    let system_prompt = build_system_prompt(&zag_help, &zag_orch);

    let output_path = output
        .map(|s| s.to_string())
        .or_else(|| name.map(|n| format!("{n}.zug")))
        .unwrap_or_else(|| "workflow.zug".to_string());

    let initial_prompt = if let Some(n) = name {
        format!(
            "I want to create a workflow called \"{n}\". \
             The output will be saved to {output_path}. \
             Please help me design it — start by asking what process I want to automate."
        )
    } else {
        format!(
            "I want to create a new workflow. \
             The output will be saved to {output_path}. \
             Please help me design it — start by asking what process I want to automate."
        )
    };

    let status = Command::new("zag")
        .args(["run", &initial_prompt])
        .args(["--system-prompt", &system_prompt])
        .args(["--name", "zig-create"])
        .args(["--tag", "zig-workflow-creation"])
        .status()
        .map_err(|e| ZigError::Zag(format!("failed to launch zag: {e}")))?;

    if !status.success() {
        return Err(ZigError::Zag(format!("zag exited with status {status}")));
    }

    // Validate the output if it was created
    if Path::new(&output_path).exists() {
        match crate::workflow::parser::parse_file(Path::new(&output_path)) {
            Ok(workflow) => {
                if let Err(errors) = crate::workflow::validate::validate(&workflow) {
                    eprintln!("Warning: generated workflow has validation issues:");
                    for e in &errors {
                        eprintln!("  - {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: could not parse generated file: {e}");
            }
        }
    }

    Ok(())
}

/// Returns the .zug format specification text (for use in other contexts).
pub fn zug_format_spec() -> &'static str {
    ZUG_FORMAT_SPEC
}
