# TypeScript Binding Reference -- @nlindstedt/zig-workflow

Comprehensive reference for the TypeScript binding of zig, a workflow orchestration engine for AI coding agents.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Builder API](#builder-api)
  - [Configuration Methods](#configuration-methods)
  - [Terminal Methods](#terminal-methods)
- [StreamingSession](#streamingsession)
- [Workflow Parsing](#workflow-parsing)
- [Types](#types)
  - [Workflow](#workflow)
  - [WorkflowMeta](#workflowmeta)
  - [Variable](#variable)
  - [Step](#step)
  - [Enums and Unions](#enums-and-unions)
  - [Output Types](#output-types)
  - [Error Types](#error-types)
- [Examples](#examples)
  - [Run a Workflow](#run-a-workflow)
  - [Stream Workflow Output](#stream-workflow-output)
  - [Bidirectional Streaming](#bidirectional-streaming)
  - [Validate a Workflow](#validate-a-workflow)
  - [Parse a .zug File](#parse-a-zug-file)
  - [Manage Workflows](#manage-workflows)
  - [Error Handling](#error-handling)
- [Internals](#internals)
  - [CLI Arg Construction](#cli-arg-construction)
  - [Version Checking](#version-checking)

---

## Quick Start

**Prerequisites:** Node.js 18+, `zig` CLI binary on `PATH` (or set via `ZIG_BIN` environment variable).

```bash
npm install @nlindstedt/zig-workflow
```

```typescript
import { ZigBuilder } from "@nlindstedt/zig-workflow";

const output = await new ZigBuilder()
  .run("deploy-pipeline");

console.log(output);
```

The package has zero external dependencies (Node.js built-ins only). It works by spawning the `zig` CLI as a subprocess.

---

## Builder API

Constructor: `new ZigBuilder()`

All setter methods return `this` for chaining.

### Configuration Methods

| Method | Signature | CLI Flag | Description |
|--------|-----------|----------|-------------|
| `bin` | `bin(path: string): this` | N/A (binding-only) | Override zig binary path. Default: `ZIG_BIN` env var, or `"zig"`. |
| `debug` | `debug(d = true): this` | `--debug` | Enable debug logging. Pass `false` to disable. |
| `quiet` | `quiet(q = true): this` | `--quiet` | Suppress all output except errors. Pass `false` to disable. |
| `autoCleanup` | `autoCleanup(enabled = true): this` | *(binding-only)* | Opt in to process-wide orphan cleanup for `StreamingSession`s. Installs idempotent `exit` / `SIGINT` / `SIGTERM` / `SIGHUP` / `uncaughtException` handlers that SIGTERM every tracked live child on parent exit. Off by default. |

### Terminal Methods

These methods execute the builder configuration. Each spawns a `zig` subprocess.

| Method | Signature | Description |
|--------|-----------|-------------|
| `run` | `async run(workflow: string, prompt?: string): Promise<string>` | Run a workflow non-interactively. Returns stdout. |
| `runInteractive` | `async runInteractive(workflow: string, prompt?: string): Promise<void>` | Run a workflow interactively. Inherits stdin/stdout/stderr. |
| `stream` | `async *stream(workflow: string, prompt?: string): AsyncGenerator<string>` | Stream stdout lines as they arrive. |
| `runStreaming` | `runStreaming(workflow: string, prompt?: string): StreamingSession` | Bidirectional streaming with piped stdin/stdout. |
| `validate` | `async validate(workflow: string): Promise<string>` | Validate a .zug file. Returns success message or throws `ZigError`. |
| `workflowList` | `async workflowList(): Promise<string>` | List available workflows. |
| `workflowShow` | `async workflowShow(workflow: string): Promise<string>` | Show workflow details. |
| `workflowDelete` | `async workflowDelete(workflow: string): Promise<string>` | Delete a workflow. |
| `workflowCreate` | `async workflowCreate(options?): Promise<void>` | Create a workflow interactively. Options: `name?`, `output?`, `pattern?`. |
| `describe` | `async describe(prompt: string, output?: string): Promise<void>` | Generate a .zug file from natural language. |
| `listen` | `async listen(options?): Promise<void>` | Tail a running/completed session. Options: `sessionId?`, `latest?`, `active?`. |
| `man` | `async man(topic?: string): Promise<string>` | Show a manual page topic. |

---

## StreamingSession

Returned by `runStreaming()`. Provides bidirectional communication with a running zig process.

```typescript
interface StreamingSession {
  send(message: string): void;
  closeInput(): void;
  lines(): AsyncGenerator<string>;
  readonly isRunning: boolean;
  terminate(): void;
  wait(): Promise<void>;
  close(options?: { timeout?: number | string }): Promise<void>;
}
```

| Method / Property | Signature | Description |
|-------------------|-----------|-------------|
| `send` | `send(message: string): void` | Write a line to the process stdin. A trailing newline is appended automatically. |
| `closeInput` | `closeInput(): void` | Close stdin to signal that no more input will be sent. |
| `lines` | `lines(): AsyncGenerator<string>` | Async iterator over lines from stdout. Empty lines are skipped. |
| `isRunning` | `readonly isRunning: boolean` | Whether the child process is still running. |
| `terminate` | `terminate(): void` | Send `SIGTERM` to the child process. No-op if already exited. |
| `wait` | `wait(): Promise<void>` | Wait for the process to exit. Resolves on exit code 0. Throws `ZigError` on non-zero exit. |
| `close` | `close(options?: { timeout?: number \| string }): Promise<void>` | Graceful shutdown helper. Closes stdin, waits up to half the budget, escalates to `SIGTERM`, and finally `SIGKILL`. `timeout` is a number in ms or a humantime string (`"5s"`, `"500ms"`, `"1m"`, `"1h"`); defaults to 5000 ms. Resolves on exit regardless of exit code. Idempotent. |

---

## Workflow Parsing

Two standalone functions for reading `.zug` workflow files directly from Node.js:

```typescript
import { parseWorkflow, parseWorkflowFile } from "@nlindstedt/zig-workflow";
```

### parseWorkflow

```typescript
function parseWorkflow(content: string): Workflow
```

Parse a TOML `.zug` workflow string into a typed `Workflow` object. This is a lightweight parser covering the standard `.zug` subset. For authoritative validation, use `ZigBuilder.validate()`.

### parseWorkflowFile

```typescript
async function parseWorkflowFile(path: string): Promise<Workflow>
```

Read a `.zug` file from disk and parse it into a `Workflow` object. Throws `ZigError` if the file cannot be read.

---

## Types

All types are importable from `@nlindstedt/zig-workflow`:

```typescript
import type {
  Workflow,
  WorkflowMeta,
  Variable,
  VarType,
  Step,
  FailurePolicy,
  StepCommand,
  Pattern,
  RunOutput,
  StepResult,
  ValidationResult,
  WorkflowInfo,
} from "@nlindstedt/zig-workflow";

import { ZigError, ZigVersionError } from "@nlindstedt/zig-workflow";
```

### Workflow

A complete workflow definition parsed from a `.zug` file.

```typescript
interface Workflow {
  /** Workflow metadata (name, description, tags). */
  workflow: WorkflowMeta;

  /** Shared variables that flow between steps, keyed by name. */
  vars: Record<string, Variable>;

  /** Ordered list of workflow steps. */
  steps: Step[];
}
```

### WorkflowMeta

```typescript
interface WorkflowMeta {
  /** Human-readable workflow name. */
  name: string;

  /** Short description of what this workflow does. */
  description: string;

  /** Tags for discovery and filtering. */
  tags: string[];
}
```

### Variable

A workflow variable — shared state between steps. Variables can be referenced
in step prompts via `${var_name}`.

```typescript
interface Variable {
  /** Variable type: "string", "number", "bool", or "json". */
  type: VarType;

  /** Default value. If absent, the variable must be provided at runtime. */
  default?: unknown;

  /** Human-readable description. */
  description: string;

  /** Input binding source. Currently only "prompt" is supported. */
  from?: string;

  /** If true, the variable must have a non-empty value. */
  required?: boolean;

  /** Minimum string length (string type only). */
  min_length?: number;

  /** Maximum string length (string type only). */
  max_length?: number;

  /** Minimum numeric value (number type only). */
  min?: number;

  /** Maximum numeric value (number type only). */
  max?: number;

  /** Regex pattern the value must match (string type only). */
  pattern?: string;

  /** Restrict value to one of these values. */
  allowed_values?: unknown[];
}
```

### Step

A single workflow step — one agent invocation.

```typescript
interface Step {
  // --- Core ---
  name: string;                       // Unique step identifier
  prompt: string;                     // Prompt template with ${var} refs
  provider?: string;                  // claude, codex, gemini, copilot, ollama
  model?: string;                     // Model name or size alias
  depends_on: string[];               // Prerequisite step names
  inject_context: boolean;            // Auto-inject dependency outputs
  condition?: string;                 // Conditional execution expression
  description: string;                // Human-readable purpose

  // --- Output ---
  json: boolean;                      // Request JSON output
  json_schema?: string;               // JSON schema for validation
  output?: string;                    // Format: text/json/json-pretty/stream-json/native-json
  saves: Record<string, string>;      // Variable saves (name → JSONPath selector)

  // --- Failure handling ---
  on_failure?: FailurePolicy;         // fail, continue, or retry
  max_retries?: number;               // Retry limit
  retry_model?: string;               // Escalation model for retries
  timeout?: string;                   // Step timeout (e.g., "5m")
  next?: string;                      // Explicit next step (for loops)

  // --- Agent configuration ---
  system_prompt?: string;             // System prompt override
  max_turns?: number;                 // Maximum agentic turns
  interactive: boolean;               // Long-lived interactive session
  auto_approve: boolean;              // Skip permission prompts
  tags: string[];                     // Session tags

  // --- Execution environment ---
  root?: string;                      // Working directory override
  add_dirs: string[];                 // Additional directories
  env: Record<string, string>;        // Per-step environment variables
  files: string[];                    // Files to attach

  // --- Context injection ---
  context: string[];                  // Session IDs to inject
  plan?: string;                      // Plan file path
  mcp_config?: string;                // MCP configuration (Claude only)

  // --- Isolation ---
  worktree: boolean;                  // Git worktree isolation
  sandbox?: string;                   // Docker sandbox name

  // --- Advanced orchestration ---
  race_group?: string;                // Race group name (first wins)

  // --- Command step types ---
  command?: StepCommand;              // review, plan, pipe, collect, summary
  uncommitted: boolean;               // Review uncommitted (review only)
  base?: string;                      // Base branch (review only)
  commit?: string;                    // Specific commit (review only)
  title?: string;                     // Review title (review only)
  plan_output?: string;               // Plan output path (plan only)
  instructions?: string;              // Plan instructions (plan only)
}
```

### Enums and Unions

```typescript
/** Supported variable types. */
type VarType = "string" | "number" | "bool" | "json";

/** What to do when a step fails. */
type FailurePolicy = "fail" | "continue" | "retry";

/** Zag command type for a step. */
type StepCommand = "review" | "plan" | "pipe" | "collect" | "summary";

/** Orchestration pattern for workflow creation. */
type Pattern =
  | "sequential"
  | "fan-out"
  | "generator-critic"
  | "coordinator-dispatcher"
  | "hierarchical-decomposition"
  | "human-in-the-loop"
  | "inter-agent-communication";
```

### Output Types

```typescript
/** Output from a zig run session. */
interface RunOutput {
  workflow: string;
  success: boolean;
  steps: Record<string, StepResult>;
  duration_ms: number | null;
  error?: string;
}

/** Result of a single workflow step execution. */
interface StepResult {
  name: string;
  success: boolean;
  output: string | null;
  error?: string;
  duration_ms: number | null;
  provider?: string;
  model?: string;
}

/** Validation result from zig validate. */
interface ValidationResult {
  valid: boolean;
  workflow_name?: string;
  step_count?: number;
  errors: string[];
}

/** Summary info for a workflow. */
interface WorkflowInfo {
  name: string;
  description: string;
  step_count: number;
  tags: string[];
}
```

### Error Types

```typescript
/** Error thrown when the zig process fails. */
class ZigError extends Error {
  /** Process exit code, or null if the process failed to spawn. */
  readonly exitCode: number | null;

  /** Captured stderr output from the process. */
  readonly stderr: string;

  constructor(message: string, exitCode: number | null, stderr: string);
}

/** Error thrown when a feature requires a newer CLI version. */
class ZigVersionError extends ZigError {
  /** The minimum version required by the feature. */
  readonly requiredVersion: string;

  /** The version that is currently installed. */
  readonly installedVersion: string;

  constructor(message: string, requiredVersion: string, installedVersion: string);
}
```

---

## Examples

### Run a Workflow

Run a workflow non-interactively and capture output:

```typescript
import { ZigBuilder } from "@nlindstedt/zig-workflow";

const output = await new ZigBuilder()
  .debug()
  .run("deploy-pipeline", "deploy to staging");

console.log(output);
```

### Stream Workflow Output

Process output lines as they arrive:

```typescript
import { ZigBuilder } from "@nlindstedt/zig-workflow";

for await (const line of new ZigBuilder().stream("build-and-test")) {
  console.log(line);
}
```

### Bidirectional Streaming

Send input during a workflow execution:

```typescript
import { ZigBuilder } from "@nlindstedt/zig-workflow";

const session = new ZigBuilder()
  .autoCleanup()
  .runStreaming("interactive-review");

// Read lines in the background
const lineStream = session.lines();

(async () => {
  for await (const line of lineStream) {
    console.log(line);
  }
})();

// Send input
session.send("approve step 2");

// Graceful shutdown
await session.close({ timeout: "5s" });
```

### Validate a Workflow

```typescript
import { ZigBuilder, ZigError } from "@nlindstedt/zig-workflow";

try {
  const msg = await new ZigBuilder().validate("deploy.zug");
  console.log(msg); // "workflow 'deploy' is valid (3 steps)"
} catch (err) {
  if (err instanceof ZigError) {
    console.error("Validation failed:");
    console.error("  Exit code:", err.exitCode);
    console.error("  Stderr:", err.stderr);
  }
}
```

### Parse a .zug File

Read and inspect a workflow definition without spawning the CLI:

```typescript
import { parseWorkflow, parseWorkflowFile } from "@nlindstedt/zig-workflow";

// From a string
const wf = parseWorkflow(`
[workflow]
name = "ci-pipeline"
description = "Run CI checks"
tags = ["ci", "automated"]

[vars.branch]
type = "string"
default = "main"
from = "prompt"

[[step]]
name = "lint"
prompt = "Run linters on \${branch}"
provider = "claude"
model = "sonnet"
auto_approve = true

[[step]]
name = "test"
prompt = "Run test suite"
depends_on = ["lint"]
inject_context = true
on_failure = "continue"
`);

console.log(wf.workflow.name);            // "ci-pipeline"
console.log(wf.workflow.tags);            // ["ci", "automated"]
console.log(wf.vars.branch.type);         // "string"
console.log(wf.vars.branch.from);         // "prompt"
console.log(wf.steps.length);             // 2
console.log(wf.steps[0].provider);        // "claude"
console.log(wf.steps[1].depends_on);      // ["lint"]
console.log(wf.steps[1].on_failure);      // "continue"

// From a file
const fromFile = await parseWorkflowFile("./deploy.zug");
console.log(fromFile.steps.length);
```

### Manage Workflows

```typescript
import { ZigBuilder } from "@nlindstedt/zig-workflow";

const zig = new ZigBuilder();

// List all workflows
const listing = await zig.workflowList();
console.log(listing);

// Show details
const details = await zig.workflowShow("deploy");
console.log(details);

// Create interactively with a pattern
await zig.workflowCreate({
  name: "new-pipeline",
  pattern: "fan-out",
  output: "./pipelines/new-pipeline.zug",
});

// Delete
await zig.workflowDelete("old-pipeline");
```

### Error Handling

```typescript
import { ZigBuilder, ZigError, ZigVersionError } from "@nlindstedt/zig-workflow";

try {
  const output = await new ZigBuilder().run("my-workflow");
  console.log(output);
} catch (err) {
  if (err instanceof ZigVersionError) {
    console.error(
      `Requires zig CLI >= ${err.requiredVersion}, ` +
      `but installed version is ${err.installedVersion}`,
    );
  } else if (err instanceof ZigError) {
    console.error("Process failed:");
    console.error("  Exit code:", err.exitCode);
    console.error("  Stderr:", err.stderr);
    console.error("  Message:", err.message);
  } else {
    throw err;
  }
}
```

A `ZigError` is thrown when:

- The `zig` binary cannot be spawned (exit code is `null`).
- The process exits with a non-zero exit code.
- A version requirement is not met (`ZigVersionError` subclass).
- A workflow file cannot be read (`parseWorkflowFile`).

---

## Internals

### CLI Arg Construction

The builder constructs a CLI argument array. Global flags (`--debug`, `--quiet`) are placed first, followed by the subcommand and its specific arguments.

The final CLI invocation has the form:

```
zig [--debug] [--quiet] run <workflow> [prompt]
zig [--debug] [--quiet] validate <workflow>
zig [--debug] [--quiet] workflow list
zig [--debug] [--quiet] workflow show <workflow>
zig [--debug] [--quiet] workflow create [name] [--output path] [--pattern pat]
zig [--debug] [--quiet] workflow delete <workflow>
zig [--debug] [--quiet] describe <prompt> [--output path]
zig [--debug] [--quiet] listen [session_id] [--latest] [--active]
zig [--debug] [--quiet] man [topic]
```

### Version Checking

All terminal methods call `checkVersion()` before spawning the process. This function:

1. Collects active version requirements from the builder configuration.
2. If any requirements are active, detects the CLI version by running `zig --version`.
3. Parses the output (expected format: `zig-cli X.Y.Z` or just `X.Y.Z`).
4. Compares the detected version against each requirement using semver comparison.
5. Throws a `ZigVersionError` with a descriptive message if any requirement is not met.

The detected version is cached per binary path for the lifetime of the process, so `zig --version` is invoked at most once per distinct binary path. Currently all features are available since the initial release (0.4.0); the version check infrastructure is in place for future feature gates.
