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

### 2. Interactive Sessions (High Impact)

| Zag Feature | .zug Gap |
|---|---|
| `spawn --interactive` | No interactive step type |
| `zag run` (interactive mode) | All steps are fire-and-forget |
| `run --resume` / `--continue` | No session resumption |

**Impact**: Human-in-the-Loop pattern via interactive input injection is not
possible. No ability to pause for approval or inject guidance mid-workflow.

### 3. Session Isolation (Medium Impact)

| Zag Feature | .zug Gap |
|---|---|
| `--worktree [NAME]` | No per-step git worktree isolation |
| `--sandbox [NAME]` | No per-step Docker sandbox isolation |

**Impact**: Parallel steps modifying the filesystem cannot be safely isolated.

### 4. Race Pattern / Early Exit (Medium Impact)

| Zag Feature | .zug Gap |
|---|---|
| `wait --any` | No "first completes wins" semantics |
| `cancel` (mid-workflow) | No dynamic step cancellation |

**Impact**: Cannot express "try multiple approaches, take whichever finishes
first" (early-exit race pattern).

### 5. Retry with Different Config (Medium Impact)

| Zag Feature | .zug Gap |
|---|---|
| `retry --model large` | `on_failure = "retry"` retries with same config only |

**Impact**: Cannot escalate to a larger model on failure (self-healing with
escalation pattern).

### 6. Context Injection (Low-Medium Impact)

| Zag Feature | .zug Gap |
|---|---|
| `--context <SESSION_ID>` | `inject_context` only from direct `depends_on` |
| `--plan <PATH>` | No plan file injection |

**Impact**: Cannot inject context from arbitrary prior sessions or plan files.

### 7. Per-Step Configuration (Low-Medium Impact)

| Zag Feature | .zug Gap |
|---|---|
| `--auto-approve` | No per-step auto-approve |
| `--add-dir <PATH>` | No additional directories |
| `--env KEY=VALUE` | No per-step environment variables |
| `--file <PATH>` | No file attachments |
| `--mcp-config` | No per-step MCP config |
| `--root <ROOT>` | No per-step working directory |
| `--size <SIZE>` | No Ollama model size parameter |

### 8. Output Format Control (Low Impact)

| Zag Feature | .zug Gap |
|---|---|
| `-o text/json/json-pretty/stream-json/native-json` | Only `json = true/false` |
| `--json-stream` (NDJSON) | No streaming output |

### 9. First-Class Command Step Types (Low Impact)

| Zag Command | .zug Gap |
|---|---|
| `review` (code review) | Must use generic prompt |
| `plan` (generate plan) | Must use generic prompt |
| `pipe` (explicit chaining) | No explicit pipe between arbitrary steps |
| `collect` (result aggregation) | No explicit collection step |
| `summary` (stats) | No summary aggregation step |

### 10. Session Metadata (Low Impact)

| Zag Feature | .zug Gap |
|---|---|
| `--description` per session | Steps have `name` but no `description` |
| `session update` | No runtime metadata updates |

## Orchestration Pattern Coverage

| # | Pattern | Supported in .zug? | Gap |
|---|---|---|---|
| 1 | Sequential Pipeline | **Yes** | — |
| 2 | Fan-Out / Gather | **Yes** | Missing race/early-exit variant |
| 3 | Coordinator / Dispatcher | **Yes** | — |
| 4 | Hierarchical Decomposition | **Mostly** | No parent tracking, no agent-driven sub-spawning |
| 5 | Generator & Critic | **Yes** | — |
| 6 | Iterative Refinement | **Yes** | — |
| 7 | Human-in-the-Loop | **No** | No interactive sessions, no `input`, no approval gates |
| 8 | Inter-Agent Communication | **No** | No `input`, `broadcast`, or agent-message envelopes |
| 9 | Composite / Event-Driven | **Partial** | No `watch`, no event-driven reactions |

## Recommendations (Priority Order)

1. ~~**`interactive` step flag + `input` support**~~ — **Implemented**
2. ~~**`worktree` / `sandbox` per step**~~ — **Implemented**
3. ~~**`race` mode / `cancel` support**~~ — **Implemented** (`race_group` field)
4. ~~**`auto_approve` per step**~~ — **Implemented**
5. ~~**`env` / `root` / `add_dir` per step**~~ — **Implemented**
6. ~~**`description` field on steps**~~ — **Implemented**
7. ~~**Retry with model override**~~ — **Implemented** (`retry_model` field)
8. **Event hooks / `watch` equivalent** — Not yet implemented (requires
   runtime execution engine support beyond step field additions).

## Remaining Gaps

After implementing recommendations 1-7 at the model layer and wiring them
through to zag CLI flags in the execution engine, the remaining gaps are:

- **`interactive` execution** — the field is parsed and validated, but executing
  interactive steps requires `zag spawn` (not `zag run`) which is a fundamentally
  different execution model (async session lifecycle). Deferred pending deeper
  investigation of `zag spawn` blocking behavior.
- **Event-driven automation** (`watch`, `subscribe`) — requires runtime support
- **`input` / `broadcast` at runtime** — the `interactive` field enables the
  session mode, but sending messages requires execution engine integration
- **`mcp_config` per step** — not yet added (low priority)
