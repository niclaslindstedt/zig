# zig self

Commands that act on the currently running zig/zag session.

## Synopsis

```
zig self <command>
```

## Description

`zig self` is a namespace for commands an agent can invoke against its
own running session. The target session is resolved from the
`ZAG_PROCESS_ID` environment variable that `zag-agent` injects into
every spawned agent process — there is no way to pass a session id
explicitly, because these commands are designed to be called from
inside the session they act on.

Typical use: an interactive workflow step (e.g. a `consult` step that
hands control to an AI coding CLI) calls `zig self terminate` when the
agent finishes, so the workflow engine can proceed to the next step
without waiting on the user to close the session manually. `zig run`
automatically appends an instruction about this to the system prompt of
every interactive step.

## Subcommands

| Command                 | Description                                       |
|-------------------------|---------------------------------------------------|
| `zig self terminate`    | Terminate the current running session (SIGTERM)   |

## zig self terminate

```
zig self terminate
```

Sends SIGTERM to the process identified by `ZAG_PROCESS_ID`. The
process's status in the zag process registry is updated to `killed`
before the signal is delivered, so the liveness check sees the new
state even though the terminating process cannot write it.

### Errors

- `ZAG_PROCESS_ID is not set` — you are not running inside a zag
  session. This command is only meaningful when invoked from within a
  session spawned by `zag-agent` / `zig run`.
- `Process <id> is not running` — the registry lists the session but
  the OS process is no longer alive.

### Exit Codes

| Code | Meaning                                  |
|------|------------------------------------------|
| `0`  | Signal delivered                         |
| `1`  | Session not found or not running         |

## Examples

```bash
# Inside an interactive agent session, exit cleanly when done
zig self terminate
```

## See Also

- `zig run` — executes workflows; injects the self-terminate
  instruction into interactive agents' system prompts
- `zag ps` — inspect and manage zag processes directly (useful when
  you need to target a session other than the current one)
