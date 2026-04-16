# Storage

Storage is a first-class workflow concept for **writable, structured working
data** that a workflow's steps produce and consume — essentially the output
files an agent creates over the course of a run.

## How storage fits with other data concepts

Zig has four distinct data concepts, each serving a different purpose:

| Concept | Purpose | Lifetime | Read/Write |
|---------|---------|----------|------------|
| **Variables** | Short-lived scalar state passed between steps | Single run | Read/Write |
| **Resources** | Read-only reference files advertised to agents | Permanent (ship with workflow) | Read-only |
| **Storage** | Structured files that agents produce and consume | Persists across runs | Read/Write |
| **Memory** | Anything agents want to remember across runs | Persists across runs | Read/Write |

- **Variables** are small key-value pairs (`${var}`) used to pass state between steps within a single run — a score, a status, a file path.
- **Resources** are read-only reference files (a style guide, API docs, a CV) that agents are told about so they can read them on demand. They ship with the workflow.
- **Storage** is the structured output — folders and files that steps create, grow, and build upon. A book workflow's `chapters/` folder, a research workflow's `summaries/` folder, a code review's `reports/` directory.
- **Memory** is a scratch pad of accumulated knowledge — notes, observations, and lessons learned that agents carry from one run to the next.

Neither variables (too small) nor resources (read-only) fit the case where a
workflow wants a growing body of files that later steps build on. A
book-writing workflow, for example, wants to keep character sheets,
world-building notes, summaries, and a consistency bible around as the run
progresses — that is what storage is for.

## Declaring storage

Storage entries live in the `.zwf` file under `[storage.<name>]`. Each entry
is either a **folder** (default) or a single **file**:

```toml
[workflow]
name = "book"

[storage.characters]
type = "folder"
path = "./characters"
description = "Character profiles, one file per character"
hint = """
Each file: name, age, background, personality traits, key relationships.
Filename: <slug>.md
"""

# Optional: concrete file hints for folder-typed storage.
[[storage.characters.files]]
name = "README.md"
description = "Character index"

[storage.world]
type = "folder"
path = "./world"
description = "World-building notes"

[[storage.world.files]]
name = "geography.md"
description = "Maps, regions, climate"

[[storage.world.files]]
name = "magic-system.md"
description = "How magic works in this world"

[storage.bible]
type = "file"
path = "./bible.md"
description = "Single source of truth for the whole book"
```

Fields:

| Field         | Type      | Description                                                  |
|---------------|-----------|--------------------------------------------------------------|
| `type`        | `"folder"` \| `"file"` | Defaults to `"folder"`.                         |
| `path`        | string    | Required. Relative paths resolve against `<cwd>/.zig/`; absolute paths pass through. |
| `description` | string    | One-line description shown to the agent alongside the path.  |
| `hint`        | string    | Free-form guidance about what should live here.              |
| `files`       | table[]   | Optional expected-file hints. Only valid for folder storage. |

`hint` and `files` are **hints**, not schemas — they are shown to the agent
in the system prompt. How much detail you provide is up to you.

## Path resolution

Storage paths resolve against **`<cwd>/.zig/`** — the directory you invoked
`zig run` from, not the `.zwf` file's directory. This is different from
`resources`, and it is intentional:

- Resources are read-only artifacts that ship with the workflow.
- Storage is writable working data that belongs to the run.

A shared `book.zwf` can therefore be invoked from any project, and each run
writes its own `characters/`, `world/`, and `bible.md` under that project's
`.zig/` directory without touching the workflow file.

Absolute paths (`/tmp/shared-store`, `~/books/current`) pass through
unchanged and are used verbatim. The `.zig/` root is created on demand.

## Step-level scoping

By default every step in the workflow sees every declared storage entry. A
step can narrow that with a `storage` field listing the names it wants:

```toml
[[step]]
name = "write_chapter"
prompt = "Draft chapter ${chapter}"
storage = ["characters", "world", "bible"]
```

Rules:

- Field **omitted** → step sees every declared entry.
- `storage = []` → step sees none (the `<storage>` block is suppressed).
- `storage = ["a", "b"]` → step sees only those entries. Unknown names fail
  validation.

Scoping is advisory: the agent is simply shown fewer items in its system
prompt. Combined with the step's `add_dirs` (which zig extends automatically
for scoped storage), it also narrows the agent sandbox.

## Rendered block

When a step has at least one storage entry in scope, zig prepends a
`<storage>` block to that step's system prompt:

```
<storage>
  <item name="characters" type="folder" path="/.../.zig/characters">
    <description>Character profiles, one file per character</description>
    <hint>Each file: name, age, background...</hint>
    <expected>
      - README.md: Character index
    </expected>
    <contents>
      - alice.md
      - bob.md
    </contents>
  </item>
  <item name="bible" type="file" path="/.../.zig/bible.md">
    <description>Single source of truth for the whole book</description>
  </item>
</storage>
```

The `<contents>` listing is refreshed when each step's system prompt is
rendered, so later steps see files that earlier steps in the same run just
wrote. Agents read and write with their normal file tools — zig does not
interpose.

## Lifecycle

- Storage folders and files are created on demand at the start of a run,
  before the first step executes.
- `ensure` is idempotent — existing folders and files are left alone, so
  previous runs' data persists across invocations by default.
- There is no automatic cleanup. If you want a clean slate, delete the
  relevant paths under `.zig/` yourself.

## Disabling storage at runtime

Pass `--no-storage` to `zig run` to suppress storage entirely for a single
invocation:

```bash
zig run my-workflow --no-storage
```

When `--no-storage` is active:

- Storage directories and files are **not** created on disk.
- The `<storage>` block is omitted from every step's system prompt.
- Step sandbox directories are not expanded for storage paths.

This is useful for dry runs, debugging, or when you want to run a workflow
that declares storage without actually writing to disk.

## Backends

The initial backend is filesystem-only. The architecture leaves room for
alternative backends (sqlite, remote object stores) behind a common
`StorageBackend` trait without changing the `.zwf` format. Future backends
will surface via an optional `backend = "..."` field on each storage entry.
