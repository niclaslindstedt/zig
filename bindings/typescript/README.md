# Zig TypeScript Binding

TypeScript binding for [zig](https://github.com/niclaslindstedt/zig) — a workflow orchestration engine for AI coding agents.

## Prerequisites

- Node.js 18+
- The `zig` CLI binary installed and on your `PATH` (or set via `ZIG_BIN` env var)
- At least one AI agent CLI (`claude`, `codex`, `gemini`, `copilot`, `ollama`) for workflow execution

## Installation

```bash
npm install @nlindstedt/zig-workflow
```

### Development setup

To work with the binding from source:

```bash
cd bindings/typescript
npm install
npm run build
```

## Quick start

```typescript
import { ZigBuilder } from "@nlindstedt/zig-workflow";

// Run a workflow
const output = await new ZigBuilder()
  .run("deploy-pipeline");

console.log(output);
```

## Running workflows

```typescript
import { ZigBuilder } from "@nlindstedt/zig-workflow";

// Non-interactive — capture stdout
const output = await new ZigBuilder()
  .run("my-workflow", "additional context");

// Interactive — inherits terminal stdio
await new ZigBuilder()
  .runInteractive("my-workflow");

// Stream output lines as they arrive
for await (const line of new ZigBuilder().stream("my-workflow")) {
  console.log(line);
}
```

### Bidirectional streaming sessions

`runStreaming()` returns a `StreamingSession` with piped stdin for sending
input mid-flight. Call `.close({ timeout })` when you are done to shut the
session down gracefully:

```typescript
const session = new ZigBuilder()
  .autoCleanup()
  .runStreaming("interactive-workflow");

for await (const line of session.lines()) {
  console.log(line);
}

await session.close({ timeout: "5s" });
```

### Automatic orphan cleanup

Long-running Node servers can leak agent subprocesses if the parent process
dies unexpectedly. Opt in to `.autoCleanup()` on the builder to install
process-wide shutdown handlers that SIGTERM every tracked live session:

```typescript
const session = new ZigBuilder()
  .autoCleanup()
  .runStreaming("my-workflow");
```

Off by default so the SDK imposes no global side effects on consumers that
don't need them.

## Validating workflows

```typescript
import { ZigBuilder, ZigError } from "@nlindstedt/zig-workflow";

try {
  const msg = await new ZigBuilder().validate("deploy.zug");
  console.log(msg); // "workflow 'deploy' is valid (3 steps)"
} catch (err) {
  if (err instanceof ZigError) {
    console.error("Validation failed:", err.stderr);
  }
}
```

## Managing workflows

```typescript
import { ZigBuilder } from "@nlindstedt/zig-workflow";

const zig = new ZigBuilder();

// List available workflows
const listing = await zig.workflowList();
console.log(listing);

// Show workflow details
const details = await zig.workflowShow("deploy");
console.log(details);

// Delete a workflow
await zig.workflowDelete("old-workflow");

// Create a workflow interactively
await zig.workflowCreate({
  name: "new-workflow",
  pattern: "fan-out",
});
```

## Parsing .zug files

The SDK includes a lightweight TOML parser for reading `.zug` workflow files
directly from Node.js without spawning the CLI:

```typescript
import { parseWorkflow, parseWorkflowFile } from "@nlindstedt/zig-workflow";

// Parse from a string
const workflow = parseWorkflow(`
[workflow]
name = "example"
description = "An example workflow"

[[step]]
name = "greet"
prompt = "Say hello"
provider = "claude"
model = "sonnet"
`);

console.log(workflow.workflow.name);     // "example"
console.log(workflow.steps[0].provider); // "claude"

// Parse from a file
const wf = await parseWorkflowFile("./deploy.zug");
console.log(wf.steps.length);
```

## Builder methods

| Method | Description |
|--------|-------------|
| `.bin(path)` | Override the `zig` binary path (default: `ZIG_BIN` env or `"zig"`) |
| `.debug()` | Enable debug logging |
| `.quiet()` | Suppress all output except errors |
| `.autoCleanup(enabled?)` | Install process-wide shutdown handlers for orphan cleanup |

## Terminal methods

| Method | Returns | Description |
|--------|---------|-------------|
| `.run(workflow, prompt?)` | `Promise<string>` | Run a workflow non-interactively, return stdout |
| `.runInteractive(workflow, prompt?)` | `Promise<void>` | Run a workflow interactively (inherits stdio) |
| `.stream(workflow, prompt?)` | `AsyncGenerator<string>` | Stream stdout lines as they arrive |
| `.runStreaming(workflow, prompt?)` | `StreamingSession` | Bidirectional streaming with piped stdin/stdout |
| `.validate(workflow)` | `Promise<string>` | Validate a .zug file |
| `.workflowList()` | `Promise<string>` | List available workflows |
| `.workflowShow(workflow)` | `Promise<string>` | Show workflow details |
| `.workflowDelete(workflow)` | `Promise<string>` | Delete a workflow |
| `.workflowCreate(options?)` | `Promise<void>` | Create a workflow interactively |
| `.describe(prompt, output?)` | `Promise<void>` | Generate a .zug file from natural language |
| `.listen(options?)` | `Promise<void>` | Tail a running/completed session |
| `.init()` | `Promise<void>` | Initialize a new zig project in the current directory |
| `.workflowPack(path, output?)` | `Promise<string>` | Pack a workflow directory into a .zug zip archive |
| `.man(topic?)` | `Promise<string>` | Show a manual page topic |

## Utility functions

| Function | Returns | Description |
|----------|---------|-------------|
| `parseWorkflow(content)` | `Workflow` | Parse a TOML `.zug` string into a typed `Workflow` object |
| `parseWorkflowFile(path)` | `Promise<Workflow>` | Read and parse a `.zug` file from disk |
| `zagSessionName(workflow, step)` | `string` | Compute the zag session name for a single step (`zig-{workflow}-{step}`) |
| `zagSessionNames(workflow)` | `Record<string, string>` | Extract all zag session names from a parsed workflow |

## Workflow types

The SDK exports TypeScript types that mirror the Rust data model:

```typescript
import type {
  Workflow,
  WorkflowMeta,
  Role,
  Variable,
  VarType,
  Step,
  FailurePolicy,
  StepCommand,
  Pattern,
} from "@nlindstedt/zig-workflow";
```

### Orchestration patterns

The `Pattern` type covers the seven orchestration patterns supported by zig:

| Pattern | Description |
|---------|-------------|
| `"sequential"` | Steps run in order, each feeding the next |
| `"fan-out"` | Parallel independent steps, then synthesize |
| `"generator-critic"` | Generate, evaluate, iterate until quality threshold |
| `"coordinator-dispatcher"` | Classify input, route to specialized handlers |
| `"hierarchical-decomposition"` | Break down into sub-tasks, delegate, synthesize |
| `"human-in-the-loop"` | Automated steps with human approval gates |
| `"inter-agent-communication"` | Agents collaborate via shared variables |

## Bridging to zag-agent

Zig names each zag session deterministically as `zig-{workflowName}-{stepName}`.
The SDK exposes helpers to compute these names so you can use
[`@nlindstedt/zag-agent`](https://github.com/niclaslindstedt/zag/tree/main/bindings/typescript)
to control individual agent sessions spawned by a workflow:

```typescript
import { parseWorkflowFile, zagSessionName, zagSessionNames } from "@nlindstedt/zig-workflow";
import { ZagBuilder } from "@nlindstedt/zag-agent";

// Single step session name
const session = zagSessionName("deploy", "lint");
// "zig-deploy-lint"

// All session names from a workflow file
const wf = await parseWorkflowFile("deploy.zug");
const sessions = zagSessionNames(wf);
// { lint: "zig-deploy-lint", test: "zig-deploy-test", deploy: "zig-deploy-deploy" }

// Use with zag-agent to control the agent session
const output = await new ZagBuilder()
  .session(sessions.lint)
  .continueLast();
```

## Error handling

```typescript
import { ZigBuilder, ZigError, ZigVersionError } from "@nlindstedt/zig-workflow";

try {
  await new ZigBuilder().run("my-workflow");
} catch (err) {
  if (err instanceof ZigVersionError) {
    console.error(`Requires zig >= ${err.requiredVersion}, have ${err.installedVersion}`);
  } else if (err instanceof ZigError) {
    console.error("Process failed:");
    console.error("  Exit code:", err.exitCode);
    console.error("  Stderr:", err.stderr);
  } else {
    throw err;
  }
}
```

## How it works

The SDK spawns the `zig` CLI as a subprocess and captures stdout/stderr.
Zero external runtime dependencies — only Node.js built-ins.

## Testing

```bash
npm run build && npm test
```

## See also

- [Zag TypeScript SDK (`@nlindstedt/zag-agent`)](https://github.com/niclaslindstedt/zag/tree/main/bindings/typescript) — Lower-level agent control
- [Zig CLI](../../README.md) — The zig command-line tool
- [.zug Format Reference](../../docs/zug.md) — Full .zug file specification

## License

[MIT](../../LICENSE)
