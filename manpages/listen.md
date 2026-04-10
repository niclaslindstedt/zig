# zig listen

Tail a running or completed zig session log.

## Synopsis

```
zig listen [SESSION_ID]
zig listen --latest
zig listen --active
```

## Description

Each `zig run` invocation records its execution as a *zig session* — a parent
layer above the per-step zag sessions it spawns. `zig listen` tails the JSONL
event log of one of these zig sessions in real time, printing step events and
the live stdout/stderr from each child zag process.

The architecture mirrors `zag listen`: append-only JSONL files polled at
100ms intervals, no IPC, no sockets. A listener can attach mid-run from any
terminal and will exit cleanly when the session ends.

## Arguments

| Argument     | Description                                              |
|--------------|----------------------------------------------------------|
| `SESSION_ID` | Full session UUID or unique prefix. Omit with --latest/--active. |

## Flags

| Flag       | Description                                                  |
|------------|--------------------------------------------------------------|
| `--latest` | Tail the most recently started session                       |
| `--active` | Tail the most recently active (still-running) session        |

When no flag is provided, `--latest` is used as the default.

## Session Storage

Sessions are stored under `~/.zig/`, mirroring zag's layout:

```
~/.zig/
  projects/<sanitized-project-path>/logs/
    index.json
    sessions/<zig_session_id>.jsonl
  sessions_index.json
```

Each `zig run` invocation generates a new UUID and writes its events to
`sessions/<id>.jsonl`. Both the per-project and global indexes are upserted
when the session starts and stamped with `ended_at`/`status` when it
finishes (including via crash/panic via the writer's `Drop` impl).

## Event Types

| Type                  | Meaning                                            |
|-----------------------|----------------------------------------------------|
| `zig_session_started` | A new `zig run` invocation began                   |
| `tier_started`        | The next tier of steps is about to execute        |
| `step_started`        | A step is being dispatched to zag                  |
| `step_output`         | A line of stdout/stderr from a child zag process   |
| `step_completed`      | A step finished successfully                       |
| `step_failed`         | A step (or attempt) failed                         |
| `step_skipped`        | A step's `condition` evaluated false               |
| `heartbeat`           | Liveness indicator emitted every 10s               |
| `zig_session_ended`   | The session finished; the listener exits          |

Each `step_started` event includes the child `zag_session_id` so you can
drill into the agent's dialogue with `zag listen <id>`.

## Examples

```bash
# Tail the currently running zig session from another terminal
zig listen --active

# Replay the most recent session
zig listen --latest

# Tail by id (full or unique prefix)
zig listen 9c3f2b
```

## See Also

- `zig man run` — the command that produces zig sessions
- `zag listen` — the analogous command for child agent sessions
