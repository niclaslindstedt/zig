# zig continue

Re-open the most recent step's agent conversation from the latest `zig run`.

## Synopsis

```
zig continue
zig continue <WORKFLOW> [PROMPT]
zig continue --session <SESSION_ID> [PROMPT]
```

## Description

`zig continue` resolves the most recent zig session in the current directory
(optionally filtered by workflow name), reads its event log, and resumes the
last step's agent conversation via `zag`'s resume mechanism.

Without a `[PROMPT]`, the terminal attaches directly to the resumed
conversation — type your follow-up message there and exit as usual (the
provider's quit shortcut, e.g. `Ctrl-D`). With a `[PROMPT]`, the resumed
turn is driven non-interactively: the prompt is sent to the agent and live
event output streams to stderr, the same way `zig run` renders steps.

This is an agent-level resume, not a workflow replay. It re-opens *one*
conversation: the agent of the last step the previous run had reached.
Full step-by-step resumption with variable rehydration would still require
zig to persist inter-step variable state.

## Arguments

| Argument     | Description                                              |
|--------------|----------------------------------------------------------|
| `WORKFLOW`   | Workflow name to filter on. Omit to use the most recent run regardless of workflow. When `--session` is set, this positional becomes the prompt instead. |
| `PROMPT`     | Optional follow-up prompt sent into the resumed agent turn. Omit to attach interactively. |

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

# Resume and immediately send a follow-up prompt (non-interactive)
zig continue code-review "now also do X"

# Resume a specific zig session by id prefix
zig continue --session 9c3f2b

# Resume a specific session and send a follow-up prompt
zig continue --session 9c3f2b "now also do X"
```

## See Also

- `zig man run` — start a workflow (produces the sessions this command resumes)
- `zig man listen` — tail a running or completed session log
