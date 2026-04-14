# Memory

Memory is a managed scratch pad of files that zig advertises to an agent
through its system prompt so it can accumulate and look up knowledge across
workflow runs. Each entry lives inside a tier directory with a `.manifest`
JSON index that tracks numeric IDs, names, descriptions, tags, and the
optional step scope. Like `resources`, only absolute paths are injected into
step prompts — the agent chooses when to read a file with its own file
tools, and context stays cheap.

Unlike `resources`, memory is meant for things the agent (or you) want to
remember *between* runs: notes, intermediate results, rubrics, transcripts.
The manifest carries metadata such as descriptions and tags, and entries are
addressed by numeric ID.

## Tiers

Memory shares the same tier layout as resources, rooted at `memory/`:

| Tier | Source                                   | Purpose                                                  |
|------|------------------------------------------|----------------------------------------------------------|
| 1    | `~/.zig/memory/_shared/`                 | Memory advertised to **every** workflow regardless of name |
| 2    | `~/.zig/memory/<workflow-name>/`         | Memory advertised only to a specific named workflow       |
| 3    | `<git-root>/.zig/memory/`                | Project-local memory (walks up from cwd to git root)      |

The project-local tier is discovered by walking up from the current working
directory until either `.zig/memory/` is found or the surrounding git root
is reached. When no `.zig/memory/` directory is found inside the git repo,
one is created on demand under the current working directory.

Each tier directory is backed by a `.manifest` JSON file that carries the
next ID to hand out and a map of entry records. You normally never touch
this file directly — use the `zig memory` commands to add, update, and
delete entries so the manifest and the files on disk stay in sync.

## Memory Modes

Workflows and individual steps can control which tiers get advertised by
setting a `memory` field to one of three modes:

| Mode     | Tiers injected                                           |
|----------|----------------------------------------------------------|
| `all`    | Global shared + global per-workflow + project-local (default) |
| `global` | Only the two global tiers — the project tier is skipped  |
| `none`   | Memory is disabled for this workflow / step              |

A step-level `memory` field overrides the workflow-level setting.

## Rendered Block

When at least one memory entry is collected, zig prepends a `<memory>` block
to the step's system prompt:

```
<memory>
You have access to the following memory files — a scratch pad of accumulated knowledge. Read them with your file tools when relevant.
To add new memories: `zig memory add <path> --workflow my-workflow --step analysis`
To update metadata: `zig memory update <id> --description "..." --tags "..."`

- /home/alice/.zig/memory/my-workflow/architecture-notes.md (id: 3) — Decisions about the auth layer [arch, design]
- /home/alice/projects/zig/.zig/memory/scratch.md (id: 7, no description — run: zig memory update 7 --description "...")
</memory>
```

The block lists absolute paths, numeric IDs, descriptions (or a nudge to add
one), and tags. The hint commands come pre-filled with the current workflow
name and, when the step has a name, a `--step` flag.

## Management Commands

The `zig memory` subcommand manages entries across tiers. It mirrors the
`zig resources` tier flags: `--workflow <name>` targets a global per-workflow
tier, `--global` targets the global shared tier, and `--cwd` (the default
when nothing else is specified) targets the project-local tier.

### `zig memory add`

```
zig memory add <file> [--workflow <name>] [--step <step>]
                      [--name <display-name>] [--description <text>]
                      [--tags tag1,tag2]
```

Copies `<file>` into the chosen tier directory, assigns the next available
numeric ID, and writes a manifest entry. Each entry has:

| Field         | Source                                                   |
|---------------|----------------------------------------------------------|
| `name`        | `--name` if given, otherwise the file's basename         |
| `description` | `--description` — a hint is printed when omitted         |
| `tags`        | `--tags` comma-separated list (zero or more)             |
| `step`        | `--step` — the step this memory is scoped to (metadata)  |
| `source`      | The absolute source path the file was added from         |
| `added`       | The UTC timestamp when the entry was added               |

`add` refuses to overwrite an existing file in the tier directory — rename
it with `--name` or delete the old entry first with `zig memory delete`.

### `zig memory update`

```
zig memory update <id> [--workflow <name>]
                       [--name <new-name>] [--description <text>]
                       [--tags tag1,tag2]
```

Updates metadata for an entry identified by its numeric `id`. When `--name`
is given, the file on disk is renamed alongside the manifest update — a
conflict with an existing filename aborts the rename. `--tags` replaces the
existing tag list rather than appending to it.

### `zig memory delete`

```
zig memory delete <id> [--workflow <name>]
```

Deletes both the manifest entry and the underlying file. Walks the
project-local tier first, then global tiers, and removes the first match.

### `zig memory show`

```
zig memory show <id> [--workflow <name>]
```

Prints metadata (name, tier, source, added, description, tags, step scope)
and the contents of the memory file.

### `zig memory list`

```
zig memory list [--workflow <name>]
```

Lists every memory entry across all relevant tiers as a table with columns
`ID`, `NAME`, `TAGS`, `TIER`, and `DESCRIPTION`. When no entries exist, a
hint is printed pointing at `zig memory add`.

### `zig memory search`

```
zig memory search <query> [--scope sentence|paragraph|section|file]
                          [--workflow <name>]
```

Runs a case-insensitive substring search across every memory file visible
from the current cwd. `--scope` controls the granularity of each result:

| Scope       | Match unit                                              |
|-------------|---------------------------------------------------------|
| `sentence`  | The sentence containing the match (default)             |
| `paragraph` | The paragraph containing the match                      |
| `section`   | The markdown h2 section containing the match            |
| `file`      | The entire file containing the match                    |

## Disabling Memory

Pass `--no-memory` to `zig run` to suppress the entire `<memory>` block for
an invocation. This drops every tier regardless of the workflow or step
mode — useful for reproducing a run with a clean slate.

```bash
zig run cover-letter --no-memory
```

## See Also

- `zig docs memory` — memory concepts and the `<memory>` block
- `zig man resources` — the sibling system for advertising reference files
- `zig man run` — `--no-memory` and the run model
