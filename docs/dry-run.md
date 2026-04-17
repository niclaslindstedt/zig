# Dry Run

`zig run --dry-run` previews what a workflow would do **without invoking
`zag`**. It walks the fully-resolved plan — every step, rendered prompt,
condition outcome, and the exact `zag` command line that would be spawned
— and prints it to stdout. Nothing is executed. No session is recorded.
No storage directories are created. `zag` does not need to be installed.

Use it when iterating on a `.zwf` file, when reviewing a workflow in
code review, or when building tooling and CI around zig.

## When to use it

- **Before a real run.** Confirm which steps will fire, in what order,
  and with what prompts — without burning tokens or producing side effects.
- **Authoring `.zwf` files.** See exactly what each step's system prompt
  will look like after `<resources>`, `<memory>`, and `<storage>` blocks
  are injected.
- **Debugging conditions.** Understand whether `when:` expressions will
  evaluate to `true`, `false`, or `unknown` (because a referenced variable
  isn't set yet) for a given set of inputs.
- **CI and tooling.** With `--format json`, emit a stable machine-readable
  plan and assert against it in tests.
- **On machines without `zag`.** Dry runs skip the `zag` check on PATH.

## What a dry run does NOT do

- Does not spawn `zag` or any agent process.
- Does not create `~/.zig/sessions/<id>/` — no session log is written.
- Does not create storage directories declared in `[storage.*]`.
- Does not write to memory.
- Does not evaluate `next:` loops or bump iteration counters.

It *does*:

- Parse and validate the workflow (same validation `zig validate` runs).
- Resolve the step DAG into tiers via topological sort.
- Render each step's prompt and system prompt with `${var}` substitution.
- Resolve roles, collect resources / memory / storage blocks.
- Compute the exact argv that would be passed to `zag`.

## Unresolved `${steps.X.Y}` references

Prior-step outputs don't exist in a dry run. Any `${steps.A.foo}` placeholder
in a downstream step's prompt stays **verbatim** in the rendered output. This
is deliberate — the literal placeholder is the truthful answer to "what
would this prompt look like at this moment?" and keeps dry-run output
visually distinct from runtime-substituted values.

## Condition outcomes

Each step's `when:` condition is reported as one of four outcomes:

| Outcome   | Meaning                                                        |
|-----------|----------------------------------------------------------------|
| `none`    | The step has no `when:` condition.                             |
| `true`    | All referenced variables are resolvable and the expression is true. |
| `false`   | All referenced variables are resolvable and the expression is false. |
| `unknown` | At least one referenced variable is missing. The `missing` field lists the unresolved names. |

## Interaction with `--no-resources`, `--no-memory`, `--no-storage`

The three block-suppression flags compose cleanly with `--dry-run`. In the
text output the block is labeled `(omitted — --no-resources)` etc. In JSON
the block's `omitted_reason` field is set to `"no_resources" | "no_memory"
| "no_storage"`.

## Text output

Default format. Human-readable, grouped per tier:

```
workflow: blog-post-pipeline  (4 steps in 4 tiers)
path:     prompts/examples/sequential.zwf
vars:
  audience = software engineers
  topic = climate change

=== Tier 0 ===
[1] step: research   command: run
    failure: fail
    condition: <none>
    prompt:
      Research the topic: climate change
      ...
    resources: (none)
    memory: (none)
    storage: (none)
    zag args: ["run", "Research the topic: climate change...", "--name", "zig-blog-post-pipeline-research", ...]
```

## JSON output

```bash
zig run my-workflow --dry-run --format json | jq .
```

The field names are zig's public contract. The top-level shape is:

```jsonc
{
  "workflow": {
    "name": "blog-post-pipeline",
    "path": "prompts/examples/sequential.zwf",
    "provider": null,
    "model": null,
    "step_count": 4,
    "tier_count": 4
  },
  "disabled": { "resources": false, "memory": false, "storage": false },
  "vars": { "topic": "climate change", "audience": "software engineers" },
  "tiers": [
    {
      "index": 0,
      "steps": [
        {
          "name": "research",
          "command": "run",
          "provider": null,
          "model": null,
          "failure": "fail",
          "depends_on": [],
          "condition": { "expr": null, "outcome": "none" },
          "saves": [],
          "prompt": "<rendered prompt>",
          "system_prompt": null,
          "blocks": {
            "resources": { "omitted_reason": null, "content": null },
            "memory":    { "omitted_reason": null, "content": null },
            "storage":   { "omitted_reason": null, "content": null }
          },
          "zag_args": ["run", "<rendered prompt>", "--name", "..."]
        }
      ]
    }
  ]
}
```

### JSON schema contract

| Field                                        | Type                     | Notes                                                                 |
|---------------------------------------------|--------------------------|-----------------------------------------------------------------------|
| `workflow.name`                              | string                   | Workflow name from `[workflow]`.                                      |
| `workflow.path`                              | string                   | Path passed to `zig run`.                                             |
| `workflow.provider` / `workflow.model`       | string \| null           | Workflow-level defaults, if any.                                      |
| `workflow.step_count`                        | integer                  | Total step count (before tier grouping).                              |
| `workflow.tier_count`                        | integer                  | Number of tiers produced by the topo sort.                            |
| `disabled.{resources,memory,storage}`        | boolean                  | Whether each `--no-*` flag was set.                                   |
| `vars`                                       | `{string: string}`       | Final variable map fed into substitution.                             |
| `tiers[].index`                              | integer                  | 0-based tier index.                                                   |
| `tiers[].steps[].name`                       | string                   | Step name.                                                            |
| `tiers[].steps[].command`                    | string                   | One of `"run"`, `"review"`, `"plan"`, `"pipe"`, `"collect"`, `"summary"`. |
| `tiers[].steps[].failure`                    | string                   | One of `"fail"`, `"continue"`, `"retry"`.                             |
| `tiers[].steps[].condition.expr`             | string \| null           | The raw `when:` expression, if any.                                   |
| `tiers[].steps[].condition.outcome`          | string                   | One of `"true"`, `"false"`, `"unknown"`, `"none"`.                    |
| `tiers[].steps[].condition.missing`          | `[string]`               | Omitted when empty. Populated for `"unknown"` outcomes.               |
| `tiers[].steps[].saves[].{name,selector}`    | string                   | Sorted by `name`.                                                     |
| `tiers[].steps[].prompt`                     | string                   | Fully rendered step prompt.                                           |
| `tiers[].steps[].system_prompt`              | string \| null           | Rendered role / system prompt, if any.                                |
| `tiers[].steps[].blocks.{resources,memory,storage}.omitted_reason` | string \| null | `"no_resources"` / `"no_memory"` / `"no_storage"` when suppressed.   |
| `tiers[].steps[].blocks.{...}.content`       | string \| null           | Rendered block text, or `null` when the block is empty or suppressed. |
| `tiers[].steps[].zag_args`                   | `[string]`               | Exact argv that `zag` would receive.                                  |

Breaking changes to this schema are signaled via conventional-commit
`feat!:` / `BREAKING CHANGE:` footers and the `CHANGELOG`.

## See also

- `zig man run` — the full flag reference for `zig run`.
- `zig docs variables` — how `${var}` substitution works.
- `zig docs conditions` — condition expression syntax.
- `zig docs storage` — why storage directories are NOT created in a dry run.
