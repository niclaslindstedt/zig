# Orchestration Patterns

Common workflow patterns for structuring multi-step agent orchestration.
Use `zig workflow create --pattern <name>` to scaffold a workflow with a
specific pattern.

## Sequential Pipeline

Steps run in order, each feeding its output to the next via `inject_context`.

**Use when:** tasks must happen in a fixed order and each step builds on
the previous result.

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
prompt = "Write a summary report"
depends_on = ["analyze"]
inject_context = true
```

## Fan-Out / Gather

Multiple independent steps run in parallel, then a final step synthesizes
their results.

**Use when:** you need multiple independent perspectives or analyses that
should be combined.

```toml
[[step]]
name = "security-review"
prompt = "Review for security issues"

[[step]]
name = "perf-review"
prompt = "Review for performance"

[[step]]
name = "style-review"
prompt = "Review for code style"

[[step]]
name = "synthesize"
prompt = "Combine all review findings into a report"
depends_on = ["security-review", "perf-review", "style-review"]
inject_context = true
```

## Generator / Critic

A generation step produces output, a critic scores it, and if the score
is below a threshold the generator refines and loops back.

**Use when:** output quality matters and iterative refinement is needed.

```toml
[vars.score]
type = "number"
default = 0

[vars.threshold]
type = "number"
default = 8

[vars.feedback]
type = "string"
default = ""

[[step]]
name = "generate"
prompt = "Write the implementation. Previous feedback: ${feedback}"

[[step]]
name = "critique"
prompt = "Score this code 1-10 and provide suggestions"
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

## Coordinator / Dispatcher

An initial step classifies the input and routes to specialized handlers
based on the classification.

**Use when:** different inputs require different handling strategies.

```toml
[vars.complexity]
type = "string"

[[step]]
name = "classify"
prompt = "Classify this task as simple, moderate, or complex"
json = true
saves = { complexity = "$.classification" }

[[step]]
name = "simple-handler"
prompt = "Handle this simple task quickly"
depends_on = ["classify"]
condition = "complexity == \"simple\""
model = "small"

[[step]]
name = "complex-handler"
prompt = "Handle this complex task thoroughly"
depends_on = ["classify"]
condition = "complexity == \"complex\""
model = "large"
```

## Hierarchical Decomposition

A planning step breaks the problem into sub-tasks, workers handle each
sub-task, and a synthesis step combines the results.

**Use when:** a problem is too large for a single agent and benefits from
divide-and-conquer.

```toml
[[step]]
name = "plan"
prompt = "Break this problem into 3 independent sub-tasks"

[[step]]
name = "worker-1"
prompt = "Handle sub-task 1"
depends_on = ["plan"]
inject_context = true

[[step]]
name = "worker-2"
prompt = "Handle sub-task 2"
depends_on = ["plan"]
inject_context = true

[[step]]
name = "worker-3"
prompt = "Handle sub-task 3"
depends_on = ["plan"]
inject_context = true

[[step]]
name = "synthesize"
prompt = "Combine all sub-task results into a final answer"
depends_on = ["worker-1", "worker-2", "worker-3"]
inject_context = true
```

## Human-in-the-Loop

Automated steps with human approval gates between them. Steps that require
approval use condition expressions on approval variables.

**Use when:** certain actions require human review before proceeding.

```toml
[vars.approved]
type = "bool"
default = false

[[step]]
name = "draft"
prompt = "Draft the deployment plan"

[[step]]
name = "review"
prompt = "Present the plan for human review"
depends_on = ["draft"]
inject_context = true

[[step]]
name = "deploy"
prompt = "Execute the deployment"
depends_on = ["review"]
condition = "approved"
```

## Inter-Agent Communication

Agents collaborate via shared variables, reading and writing to common
state as the workflow progresses.

**Use when:** agents need to build on each other's work through shared
data rather than direct output injection.

```toml
[vars.findings]
type = "json"
default = "[]"

[vars.recommendations]
type = "json"
default = "[]"

[[step]]
name = "investigate"
prompt = "Investigate the issue and record findings"
json = true
saves = { findings = "$" }

[[step]]
name = "recommend"
prompt = "Based on findings: ${findings}, provide recommendations"
depends_on = ["investigate"]
json = true
saves = { recommendations = "$" }

[[step]]
name = "summarize"
prompt = "Summarize findings: ${findings} and recommendations: ${recommendations}"
depends_on = ["recommend"]
```

## See Also

- `zig man workflow` — manage workflows, including `create --pattern`
- `zig man zwf` — the `.zwf` file format
- `zig man variables` — variable system and data flow
