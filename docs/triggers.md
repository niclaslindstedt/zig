# Event-driven workflows

A normal `.zwf` workflow runs once: `zig run` parses the file, executes the
step DAG, and exits. An **event-driven** workflow turns the same file into a
long-running listener — `zig run` subscribes to an external source, executes
the DAG once per inbound event, and emits each step's outcome to a sink so
another process can route it onward (e.g. back to the user as a chat reply).

A workflow opts into event mode by declaring a `[trigger]` table. Without it,
`zig run` behaves exactly as before.

## Pipeline

```sh
external-source | zig run bot.zwf | external-sink
```

zig is intentionally agnostic about the source and sink. The first release
supports `source = "stdin"` (newline-delimited JSON, one object per line)
for input and `kind = "stdout-jsonl"` (newline-delimited JSON) for output,
so any tool that can produce or consume JSONL on a pipe plugs in directly.

For example, with [`zad`](https://github.com/niclaslindstedt/zad) wired to a
Telegram chat as the source and sink:

```sh
zad telegram listen --chat ops --json | zig run bot.zwf | zad telegram send --chat ops --stdin
```

## Workflow shape

```toml
[workflow]
name = "echo-bot"

[trigger]
source = "stdin"          # required; v1: only "stdin"
format = "jsonl"          # required for stdin; v1: only "jsonl"
bind   = "event"          # name of the var receiving each event payload

[trigger.output]
kind         = "stdout-jsonl"  # v1: only "stdout-jsonl"
mode         = "all-steps"     # "all-steps" (default) | "final" | "opt-in"
include_vars = false           # attach a vars_snapshot to each emit

[vars.event]
type = "json"
from = "event"             # required; marks the bound variable

[[step]]
name   = "reply"
prompt = "Reply to the user message: ${event}"
```

The `[trigger]` table is validated at parse time:

- `source` must be `"stdin"` and `format` must be `"jsonl"`.
- `bind` must name a `[vars.<name>]` entry whose `type = "json"` and
  `from = "event"`.
- A workflow cannot mix `from = "prompt"` and `from = "event"`.
- If `[trigger.output].mode = "opt-in"`, at least one step must set
  `emit = true`.

## Emit modes

The `[trigger.output].mode` field controls which step outcomes are written
to the sink:

| Mode        | Emits                                                       |
|-------------|-------------------------------------------------------------|
| `all-steps` | Every step outcome (ok / failed / skipped) per event.       |
| `final`     | One record per event: the last step that completed with `ok`. |
| `opt-in`    | Only steps with `emit = true` in their `[[step]]` table.    |

A terminal `__workflow__` record is also emitted when an event aborts
mid-DAG, so consumers can detect partial events.

## Emit record shape

Each emitted JSONL line is one object. Fields:

| Field           | Type            | Notes                                     |
|-----------------|-----------------|-------------------------------------------|
| `event_id`      | string          | Per-process id, e.g. `evt-000001`         |
| `event_seq`     | number          | Monotonic 1-based counter                 |
| `step`          | string          | Step name, or `__workflow__` for aborts   |
| `status`        | string          | `"ok"`, `"failed"`, or `"skipped"`        |
| `output`        | string?         | Captured step output (status `"ok"`)      |
| `error`         | string?         | Failure message (status `"failed"`)       |
| `reason`        | string?         | Skip reason (status `"skipped"`)          |
| `ts`            | string          | RFC 3339 UTC timestamp with ms            |
| `vars_snapshot` | object?         | Present when `include_vars = true`        |

Example:

```json
{"event_id":"evt-000001","event_seq":1,"step":"reply","status":"ok","output":"Hi!","ts":"2026-05-12T14:33:12.456Z"}
```

## Per-event isolation

Variables reset to their declared defaults at the start of each event before
the bound payload is inserted. Steps cannot carry state across events
through `vars`; if you need cross-event memory, write it to `storage`
(filesystem-backed and shared across events for the same listener).

## Lifecycle

- **Startup**: zig parses the workflow, validates the trigger, opens its
  session log, and starts reading stdin.
- **Per event**: each non-empty line is JSON-parsed. Malformed lines are
  logged to stderr and skipped. Valid lines bump the sequence counter, run
  the DAG, and flush emit records to stdout.
- **Per-event errors**: a DAG failure inside one event is logged to stderr,
  emitted as a `__workflow__` failed record, and the listener continues with
  the next event. The listener never exits because of a single bad event.
- **Shutdown**: stdin EOF or `SIGINT` ends the loop cleanly. The session log
  is finalized and the process exits with status 0.

zig's own logging (zag streaming, step lifecycle messages) stays on stderr,
so stdout is reserved for pure JSONL emit records.

## Limitations (v1)

- Only `source = "stdin"` and `kind = "stdout-jsonl"` are supported. Other
  variants (`http`, `file`, server SSE/WebSocket) are reserved for future
  releases — the field is a string so new sources land additively.
- Events are processed serially. Concurrent per-event execution will land
  later via a `[trigger.concurrency]` table.
- `--dry-run` is rejected for triggered workflows for now.
- `--prompt` and `[trigger]` are mutually exclusive: prompt mode runs once,
  event mode runs once per event.
