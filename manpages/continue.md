# zig continue

Re-open the most recent step's agent conversation from the latest `zig run`.

## Synopsis

```
zig continue
zig continue <WORKFLOW>
zig continue --session <SESSION_ID>
```

## Description

`zig continue` resolves the most recent zig session in the current directory
(optionally filtered by workflow name), reads its event log, and resumes the
last step's agent conversation interactively via `zag`'s resume mechanism.

The terminal attaches directly to the resumed conversation — type your
follow-up message there. Exit the agent session as you normally would (the
provider's quit shortcut, e.g. `Ctrl-D`).

This MVP does not replay workflow orchestration. It re-opens *one*
conversation: the agent of the last step the previous run had reached.
Future work may add full step-by-step resumption with variable rehydration
once `zag` exposes a builder API for resume-with-prompt and zig persists
inter-step variable state.

## Arguments

| Argument     | Description                                              |
|--------------|----------------------------------------------------------|
| `WORKFLOW`   | Workflow name to filter on. Omit to use the most recent run regardless of workflow. |

## Flags

| Flag                 | Description                                          |
|----------------------|------------------------------------------------------|
| `--session <ID>`     | Resume a specific zig session by id (or unique prefix). Mutually exclusive with `<WORKFLOW>`. |

## Resolution Order

1. `--session <ID>` → exact match or unique prefix in the project session
   index.
2. `<WORKFLOW>` → most recent project entry whose `workflow_name` matches.
3. No argument → most recent project entry.

In every case the chosen entry's JSONL log is scanned and the **last**
recorded `step_started` event determines the zag session id to resume.

## Session Storage

Resolution reads the same per-project index as `zig listen`:

```
~/.zig/projects/<sanitized-project-path>/logs/
  index.json
  sessions/<zig_session_id>.jsonl
```

If the project has never produced a zig session, `zig continue` errors with
a hint to run `zig run` first.

## Examples

```bash
# Re-open the last conversation from the most recent zig run in cwd
zig continue

# Same, but filter to a specific workflow's most recent run
zig continue code-review

# Resume a specific zig session by id prefix
zig continue --session 9c3f2b
```

## See Also

- `zig man run` — start a workflow (produces the sessions this command resumes)
- `zig man listen` — tail a running or completed session log
