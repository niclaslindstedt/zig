---
description: "Use when the user wants to keep the TypeScript binding in sync with the Rust zig-core and zig-cli source of truth. Guides adding new builder methods, CLI flags, workflow model types, tests, and documentation updates."
---

# Updating Language Bindings

The TypeScript binding (`bindings/typescript/`) mirrors the zig CLI via subprocess calls. The `ZigBuilder` class constructs CLI arguments and spawns the `zig` binary. It tends to fall out of sync when new CLI commands, flags, or workflow model fields are added to Rust but not propagated to the binding.

## Upstream References

- **CLI commands and flags (source of truth)**: `zig-cli/src/cli.rs` — `Cli`, `Command`, `WorkflowCommand`, `Pattern` enums
- **Workflow model (source of truth)**: `zig-core/src/workflow/model.rs` — `Workflow`, `Step`, `Variable`, `VarType`, `FailurePolicy`, `StepCommand`
- **Command dispatch**: `zig-cli/src/main.rs` — how CLI args map to zig-core operations

## Discovery Process

1. Read `zig-cli/src/cli.rs` to get the current CLI commands, subcommands, and flags
2. Read `zig-core/src/workflow/model.rs` to get the current workflow model types
3. Compare against the TypeScript binding files:
   - `bindings/typescript/src/builder.ts` — `ZigBuilder` methods
   - `bindings/typescript/src/types.ts` — type definitions
   - `bindings/typescript/src/workflow.ts` — TOML parser field handling
4. Identify: new CLI commands not wrapped in builder, new model fields not in types, new flags not supported

## Automated Discovery

Compare Rust source against TypeScript binding:

```sh
# CLI commands
grep -E '^\s+\w+\s*\{' zig-cli/src/cli.rs

# CLI flags
grep -E '#\[arg\(' zig-cli/src/cli.rs

# Workflow model fields
grep 'pub ' zig-core/src/workflow/model.rs

# TypeScript builder methods
grep -E '^\s+async\s+\w+|^\s+\w+\(.*\).*: this' bindings/typescript/src/builder.ts

# TypeScript type fields
grep -E '^\s+\w+[\?:]' bindings/typescript/src/types.ts

# TypeScript workflow parser fields
grep "case '" bindings/typescript/src/workflow.ts
```

## Implementation Files

### Primary — Source of truth

| File | Role |
|------|------|
| `zig-cli/src/cli.rs` | CLI commands, flags, Pattern enum |
| `zig-core/src/workflow/model.rs` | Workflow, Step, Variable model types |
| `zig-cli/src/main.rs` | Command dispatch |

### Primary — TypeScript binding

| File | Role |
|------|------|
| `bindings/typescript/src/types.ts` | TypeScript type definitions for Workflow, Step, Variable, error classes |
| `bindings/typescript/src/builder.ts` | `ZigBuilder` fluent API wrapping CLI commands |
| `bindings/typescript/src/workflow.ts` | Lightweight TOML parser for .zwf files |
| `bindings/typescript/src/process.ts` | Subprocess management (exec, stream, run) |
| `bindings/typescript/src/version.ts` | CLI version detection and checking |
| `bindings/typescript/src/index.ts` | Public API exports |
| `bindings/typescript/tests/builder.test.ts` | Builder, version, and workflow parsing tests |
| `bindings/typescript/tests/streaming.test.ts` | StreamingSession lifecycle tests |
| `bindings/typescript/README.md` | User documentation |
| `bindings/typescript/REFERENCE.md` | Comprehensive API reference |

## Implementation Patterns

### Architecture

The TypeScript binding follows this architecture:

1. `ZigBuilder` class with private fields for CLI configuration (`_debug`, `_quiet`, `_bin`)
2. Fluent setter methods that return `this` for chaining
3. Internal `buildGlobalArgs()` method for shared flags
4. Terminal methods that spawn `zig` subprocess: `run`, `runInteractive`, `stream`, `runStreaming`, `validate`, `workflowList`, `workflowShow`, `workflowDelete`, `workflowCreate`, `listen`, `man`

### Naming conventions

| Concept | Convention |
|---------|-----------|
| Method style | `camelCase` |
| Bool default | `(v = true)` |
| Return type | `: this` (setters), `Promise<string>` (exec), `Promise<void>` (interactive) |
| Async | `async` methods for all terminal methods |

### Global args

Global args go in `buildGlobalArgs()` — placed before the subcommand:

`--debug`, `--quiet`

### Workflow model types

Types in `bindings/typescript/src/types.ts` mirror `zig-core/src/workflow/model.rs`:

- `Workflow` → `Workflow` interface
- `WorkflowMeta` → `WorkflowMeta` interface
- `Variable` → `Variable` interface
- `VarType` → `VarType` type alias
- `Step` → `Step` interface
- `FailurePolicy` → `FailurePolicy` type alias
- `StepCommand` → `StepCommand` type alias

### Workflow parser fields

The TOML parser in `bindings/typescript/src/workflow.ts` must handle every field in the `Step` struct. When a new field is added to `model.rs`, add a corresponding `case` in `assignStepField()`.

## Adding a New CLI Command

### Step 1: Add terminal method to ZigBuilder

```typescript
// In bindings/typescript/src/builder.ts
async newCommand(arg: string): Promise<string> {
  await this.preflight();
  const args = [...this.buildGlobalArgs(), "new-command", arg];
  return execZig(this._bin, args);
}
```

### Step 2: Add tests

```typescript
// In bindings/typescript/tests/builder.test.ts
// Add to the method chaining test or create a new test case
```

### Step 3: Update documentation

- Add row to "Terminal methods" table in `README.md`
- Add entry to `REFERENCE.md` Terminal Methods table with full signature

## Adding a New Model Field

### Step 1: Add to types.ts

```typescript
// In bindings/typescript/src/types.ts — Step interface
new_field?: string;
```

### Step 2: Add to workflow parser

```typescript
// In bindings/typescript/src/workflow.ts — assignStepField()
case "new_field": step.new_field = String(value); break;

// In createDefaultStep() if the field has a default value
```

### Step 3: Add to REFERENCE.md

Update the Step type definition in REFERENCE.md.

## Adding a New Global CLI Flag

### Step 1: Add field and setter to ZigBuilder

```typescript
// Field
private _newFlag = false;

// Setter
newFlag(v = true): this {
  this._newFlag = v;
  return this;
}
```

### Step 2: Wire into buildGlobalArgs()

```typescript
if (this._newFlag) args.push("--new-flag");
```

### Step 3: Tests + docs

- Add to builder chaining test
- Add row to README "Builder methods" table
- Add to REFERENCE.md "Configuration Methods" table

## Update Checklist

- [ ] **Types**: Update `bindings/typescript/src/types.ts` with new model types/fields
- [ ] **Builder**: Add new terminal methods or configuration setters to `bindings/typescript/src/builder.ts`
- [ ] **Parser**: Add new step fields to `assignStepField()` in `bindings/typescript/src/workflow.ts`
- [ ] **Exports**: Update `bindings/typescript/src/index.ts` if new types are added
- [ ] **Tests**: Add test cases to `bindings/typescript/tests/builder.test.ts`
- [ ] **README**: Update tables in `bindings/typescript/README.md`
- [ ] **Reference**: Update `bindings/typescript/REFERENCE.md`

## Verification

```sh
# TypeScript binding
cd bindings/typescript && npm run build && npm test

# Rust (ensure source still compiles)
make build && make test && make clippy
```
