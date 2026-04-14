# Memory

Memory is a scratch pad of reference files that zig injects into each step's
system prompt via a `<memory>` block so agents can accumulate and reuse
knowledge across workflow runs. Unlike `step.files`, memory entries are never
inlined into the user message — only their absolute paths, display names, and
descriptions are advertised, and the agent decides which to read with its file
tools on demand.

Each memory entry has an **id**, metadata (name, description, tags, optional
step scope), and a file on disk. Metadata is stored in a `.manifest` JSON file
alongside the files in each tier.

## Tiers

Memory mirrors the [resources](resources.md) tier layout and is collected from
three tiers at run time:

| Tier  | Source                                | Purpose                                                    |
|-------|---------------------------------------|------------------------------------------------------------|
| 1     | `~/.zig/memory/_shared/`              | Memory advertised to **every** workflow regardless of name |
| 2     | `~/.zig/memory/<workflow-name>/`      | Memory scoped to a specific named workflow                 |
| 3     | `<git-root>/.zig/memory/`             | Project-local memory (walks up from cwd to git root)       |

Memory from every enabled tier is merged into the `<memory>` block. Entries are
not deduplicated across tiers — the goal is to expose everything that is
actually on disk.

## Rendered Block

When at least one memory entry is collected, zig prepends a `<memory>` block
to the step's system prompt:

```
<memory>
You have access to the following memory files — a scratch pad of accumulated knowledge.
Read them with your file tools when relevant.
To add new memories: `zig memory add <path> --workflow my-workflow --step analysis`
To update metadata: `zig memory update <id> --description "..." --tags "..."`

- /home/alice/.zig/memory/_shared/house-style.md (id: 1) — House style guide [style]
- /home/alice/projects/my-app/.zig/memory/notes.md (id: 3) — Architecture notes [arch, design]
</memory>
```

Paths are absolute. Each entry lists its numeric id and, when set, its
description and tags. Missing descriptions render a hint telling the agent to
run `zig memory update <id> --description "..."`.

## Memory Modes

Steps and workflows can narrow the memory injection via the `memory` field:

| Mode       | Effect                                                    |
|------------|-----------------------------------------------------------|
| `all`      | Inject every tier (default)                               |
| `global`   | Inject only the global tiers, skip project-local          |
| `none`     | Disable memory injection entirely for this step/workflow  |

Step-level `memory` overrides the workflow-level setting. Pass `--no-memory`
to `zig run` to suppress the `<memory>` block for the entire invocation.

```bash
zig run my-workflow --no-memory
```

## Management Commands

The `zig memory` subcommand manages entries across all tiers. Use
`--workflow <name>` to target a specific workflow tier or omit it for the
project-local tier.

### `zig memory add`

```
zig memory add <path> [--workflow <name>] [--step <name>] [--name <display>] \
    [--description <text>] [--tags tag1,tag2]
```

Copies `<path>` into the chosen tier directory, assigns a new numeric id, and
writes a manifest entry. The `--step` flag records which workflow step the
memory is most relevant to (metadata only — it does not restrict injection).

### `zig memory update`

```
zig memory update <id> [--workflow <name>] [--name <display>] \
    [--description <text>] [--tags tag1,tag2]
```

Updates the metadata for an existing entry. Omitted flags leave existing
values unchanged; `--tags` replaces the full tag set.

### `zig memory delete`

```
zig memory delete <id> [--workflow <name>]
```

Removes the entry from the manifest and deletes its file.

### `zig memory show`

```
zig memory show <id> [--workflow <name>]
```

Prints metadata (name, description, tags, source, added timestamp) and the
contents of the memory file.

### `zig memory list`

```
zig memory list [--workflow <name>]
```

Lists every memory entry across all tiers with its id, display name,
description, and tags.

### `zig memory search`

```
zig memory search <query> [--scope sentence|paragraph|section|file] [--workflow <name>]
```

Full-text search across memory files. The `--scope` flag controls result
granularity:

| Scope       | Match Unit                                |
|-------------|-------------------------------------------|
| `sentence`  | Individual sentences containing the query |
| `paragraph` | Paragraphs containing the query           |
| `section`   | Markdown sections containing the query    |
| `file`      | Whole files containing the query          |

## See Also

- `zig docs resources` — reference files vs. accumulated memory
- `zig docs zug` — the `memory` field on `[workflow]` and `[[step]]`
- `zig man run` — `--no-memory` and the run model
