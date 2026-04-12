# Gap Analysis: .zug Workflow Format vs. Zag Capabilities

> Generated 2026-04-09 by analyzing zig-core `.zug` model against the full
> zag CLI + zag-orch orchestration surface.

## Summary

The `.zug` format covers **core DAG orchestration** well — sequential pipelines,
fan-out/gather, generator/critic loops, and coordinator/dispatcher patterns all
map cleanly to `zag spawn` with `--depends-on` and `--inject-context`. However,
there are **significant gaps** in real-time communication, session lifecycle
management, isolation, and several zag features that have no `.zug` equivalent.

## What .zug Supports Well

| .zug Feature | Zag Mapping |
|---|---|
| `depends_on` | `zag spawn --depends-on` |
| `inject_context` | `zag spawn --inject-context` |
| `condition` expressions | Shell-level conditional logic |
| `saves` (variable extraction) | `zag exec --json` + jq |
| `${var}` interpolation | Shell variable substitution |
| `provider` / `model` | `-p <provider> -m <model>` |
| `timeout` | `--timeout` |
| `max_turns` | `--max-turns` |
| `system_prompt` | `--system-prompt` |
| `json` / `json_schema` | `--json` / `--json-schema` |
| `tags` | `--tag` |
| `on_failure` (fail/continue/retry) | Shell error handling + `zag retry` |
| `max_retries` | Loop logic around retry |
| `next` (explicit jump for loops) | Shell loops with spawn/wait/pipe |
| `description` per step | `--description` |
| `roles` (reusable system prompts) | `--system-prompt` from role definitions |
| `files` (file injection) | `--file` |
| `retry_model` (escalation on retry) | `zag retry --model` |
| `race_group` (first-wins semantics) | `zag wait --any` equivalent |
| `command` step types (review, plan, pipe, collect, summary) | `zag review`, `zag plan`, `zag pipe`, `zag collect`, `zag summary` |
| `worktree` / `sandbox` isolation | `--worktree` / `--sandbox` |
| `auto_approve` | `--auto-approve` |
| `root` / `add_dirs` / `env` | `--root` / `--add-dir` / `--env` |
| `context` / `plan` injection | `--context` / `--plan` |
| `mcp_config` | `--mcp-config` |
| `output` format control | `-o <FORMAT>` |
| `default_file` for variables | File-backed variable defaults |
| `from = "prompt"` input binding | CLI prompt to variable binding |
| Variable constraints (required, min/max, pattern, allowed_values) | Pre-execution validation |

## Gaps: Zag Features NOT Expressible in .zug

### 1. Real-Time Communication (High Impact)

| Zag Command | Description | .zug Gap |
|---|---|---|
| `input` | Send message to a running session | No mid-execution communication |
| `broadcast` | Send message to all sessions | No broadcast primitive |
| `listen` | Tail session log events | No monitoring hooks |
| `subscribe` | Multiplexed event stream | No event subscription |
| `watch` | Event-driven command execution | No reactive/event-driven steps |

**Impact**: Inter-Agent Communication (A2A) and event-driven reactive pipeline
patterns are impossible. Named agent messaging, broadcast coordination, and
agent-message envelopes have no equivalent.

### 2. Interactive Sessions (High Impact) — **Partially Implemented**

| Zag Feature | Status |
|---|---|
| `spawn --interactive` | `interactive` field parsed and validated but not yet wired to execution |
| `zag run` (interactive mode) | Not yet supported — all steps still fire-and-forget |
| `run --resume` / `--continue` | No session resumption |

**Impact**: Human-in-the-Loop pattern via interactive input injection is
partially modeled but not yet executable. The `interactive` field exists at
the model layer; wiring it to `zag spawn --interactive` requires deeper
execution engine changes.

### ~~3. Session Isolation (Medium Impact)~~ — **Implemented**

`worktree` and `sandbox` fields on steps, wired to `--worktree` and
`--sandbox` flags on zag.

### ~~4. Race Pattern / Early Exit (Medium Impact)~~ — **Implemented**

`race_group` field on steps. Steps sharing a race group run in parallel;
when the first completes, the rest are cancelled. Validation ensures race
group members do not depend on each other.

### ~~5. Retry with Different Config (Medium Impact)~~ — **Implemented**

`retry_model` field on steps. When `on_failure = "retry"`, the retry uses the
specified model for escalation. Validation ensures `retry_model` is only set
when `on_failure = "retry"`.

### ~~6. Context Injection (Low-Medium Impact)~~ — **Implemented**

`context` (list of session IDs) and `plan` (file path) fields on steps,
wired to `--context` and `--plan` flags on zag.

### ~~7. Per-Step Configuration (Low-Medium Impact)~~ — **Implemented**

All fields implemented: `auto_approve`, `add_dirs`, `env`, `files`,
`mcp_config`, `root`. Only `--size` (Ollama) remains unimplemented.

### ~~8. Output Format Control (Low Impact)~~ — **Implemented**

`output` field accepts `"text"`, `"json"`, `"json-pretty"`, `"stream-json"`,
`"native-json"`. When set, overrides the `json` bool and maps to `-o <FORMAT>`.

### ~~9. First-Class Command Step Types (Low Impact)~~ — **Implemented**

`command` field with enum values: `"review"`, `"plan"`, `"pipe"`, `"collect"`,
`"summary"`. Each dispatches to the corresponding zag subcommand with
command-specific options (`uncommitted`, `base`, `commit`, `title`,
`plan_output`, `instructions`).

### ~~10. Session Metadata (Low Impact)~~ — **Partially Implemented**

Steps now have a `description` field wired to `--description` on zag.
Only `session update` (runtime metadata updates) remains unimplemented.

## Orchestration Pattern Coverage

| # | Pattern | Supported in .zug? | Gap |
|---|---|---|---|
| 1 | Sequential Pipeline | **Yes** | — |
| 2 | Fan-Out / Gather | **Yes** | Race variant now supported via `race_group` |
| 3 | Coordinator / Dispatcher | **Yes** | — |
| 4 | Hierarchical Decomposition | **Mostly** | No parent tracking, no agent-driven sub-spawning |
| 5 | Generator & Critic | **Yes** | — |
| 6 | Iterative Refinement | **Yes** | — |
| 7 | Human-in-the-Loop | **Partial** | `interactive` field parsed but execution not yet wired; no `input` at runtime |
| 8 | Inter-Agent Communication | **No** | No `input`, `broadcast`, or agent-message envelopes |
| 9 | Composite / Event-Driven | **Partial** | No `watch`, no event-driven reactions |

## Recommendations (Priority Order)

1. ~~**`interactive` step flag + `input` support**~~ — **Implemented** (field only)
2. ~~**`worktree` / `sandbox` per step**~~ — **Implemented**
3. ~~**`race` mode / `cancel` support**~~ — **Implemented** (`race_group` field)
4. ~~**`auto_approve` per step**~~ — **Implemented**
5. ~~**`env` / `root` / `add_dir` per step**~~ — **Implemented**
6. ~~**`description` field on steps**~~ — **Implemented**
7. ~~**Retry with model override**~~ — **Implemented** (`retry_model` field)
8. ~~**Context injection (`context`, `plan`)**~~ — **Implemented**
9. ~~**MCP config per step**~~ — **Implemented** (`mcp_config` field)
10. ~~**Output format control**~~ — **Implemented** (`output` field)
11. ~~**Command step types (review, plan, pipe, collect, summary)**~~ — **Implemented** (`command` field)
12. **Event hooks / `watch` equivalent** — Not yet implemented (requires
    runtime execution engine support beyond step field additions).

## Remaining Gaps

After implementing recommendations 1-11 at the model layer and wiring them
through to zag CLI flags in the execution engine, and closing gaps 3-5 and 10,
the remaining gaps are:

- **`interactive` execution** — the field is parsed and validated, but executing
  interactive steps requires `zag spawn --interactive` (not `zag run`) which is
  a fundamentally different execution model (async session lifecycle). Deferred
  pending deeper investigation of `zag spawn` blocking behavior.
- **Event-driven automation** (`watch`, `subscribe`) — requires runtime support
- **`input` / `broadcast` at runtime** — the `interactive` field enables the
  session mode, but sending messages requires execution engine integration
- **`session update`** — runtime metadata updates (low priority)
- **`--size` per step** — Ollama model size parameter (very low priority)
