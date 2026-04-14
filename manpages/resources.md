# Resources

Resources are reference files (a CV, a style guide, reference docs, …) that
zig tells an agent about through its system prompt so it can choose to read
them with its file tools on demand. Unlike `step.files`, resources are never
inlined into the user message — only their absolute paths are advertised, so
context stays cheap and the agent decides what to actually pull in.

## Tiers

Resources are collected from five tiers and merged at run time in this order:

| Tier | Source                                   | Purpose                                                  |
|------|------------------------------------------|----------------------------------------------------------|
| 1    | `~/.zig/resources/_shared/`              | Files advertised to **every** workflow regardless of name |
| 2    | `~/.zig/resources/<workflow-name>/`      | Files advertised only to a specific named workflow        |
| 3    | `<git-root>/.zig/resources/`             | Project-local resources (walks up from cwd to git root)   |
| 4    | `[workflow] resources = [...]`           | Inline workflow-level resources from the `.zwf` file      |
| 5    | `[[step]] resources = [...]`             | Inline step-level resources                               |

The first tier to register a given canonical path wins for display ordering;
later tiers see the file as already present and skip it. Different files with
the same basename across tiers are *both* advertised — name collisions are not
dropped, because the goal is to expose everything that's actually on disk.

The cwd tier walks up from the current working directory until it either finds
`.zig/resources/` or hits the surrounding git repository root. It will not walk
past the git root, matching the convention used for session storage.

## Rendered Block

When at least one resource is collected, zig prepends a `<resources>` block to
the step's system prompt:

```
<resources>
You have access to the following reference files. Read them with your file
tools when the user's request relates to them.

- /home/alice/.zig/resources/_shared/style-guide.md (style-guide.md)
- /home/alice/work/me/cv.md — Candidate CV
- /home/alice/projects/cover-letter/templates/intro.md (intro.md)
</resources>
```

Paths are absolute and canonicalized. When a `description` was provided in the
detailed inline form it appears after `—`; otherwise the file's display name
appears in parentheses.

## Inline Resources in `.zwf` Files

Both `[workflow]` and `[[step]]` accept a `resources` field. Each entry is
either a bare path string or a table with `path`, `name`, `description`, and
`required`:

```toml
[workflow]
name = "cover-letter"
resources = [
  "./style-guide.md",
  { path = "./cv.md", name = "cv", description = "Candidate CV", required = true },
]

[[step]]
name = "draft"
prompt = "Write a cover letter for the attached job posting."
resources = [{ path = "./templates/cover-letter.md", description = "House template" }]
```

Inline paths are resolved relative to the `.zwf` file. A missing optional
resource is logged as a warning and skipped; a missing **required** resource
aborts the run.

## Management Commands

The `zig resources` subcommand manages files in the global and project tiers.
Inline resources in `.zwf` files are not touched — edit the workflow file to
change those.

### `zig resources list`

```
zig resources list [--global] [--cwd] [--workflow <name>]
```

Lists resources from every tier by default. `--global` and `--cwd` restrict
the listing to one side. `--workflow <name>` restricts the global tier to
`~/.zig/resources/<name>/`; without it, every per-workflow subdirectory is
shown.

### `zig resources add`

```
zig resources add <file> [--global] [--cwd] [--workflow <name>] [--name <new-name>]
```

Copies `<file>` into the chosen tier directory:

| Flag combination       | Destination                                |
|------------------------|--------------------------------------------|
| `--workflow <name>`    | `~/.zig/resources/<name>/`                 |
| `--global`             | `~/.zig/resources/_shared/`                |
| `--cwd`                | `<git-root>/.zig/resources/`               |
| (no flags)             | Same as `--cwd`                            |

`--name` lets you rename the file as it lands in the tier directory. Add
refuses to overwrite an existing file — remove it first with `zig resources
remove`.

### `zig resources remove`

```
zig resources remove <name> [--global] [--cwd] [--workflow <name>]
```

Deletes a file by its registered name from the chosen tier.

### `zig resources show`

```
zig resources show <name> [--workflow <name>]
```

Prints the absolute path and the contents of the first matching resource
across all tiers, in the same order as `collect_for_step` would walk them.

### `zig resources where`

```
zig resources where [--workflow <name>]
```

Prints the directories the collector would search for the current cwd. Each
line is a tier label and the resolved directory; missing directories are
flagged with `(missing)` so you can tell at a glance which tiers are empty.

## Disabling Resources

Pass `--no-resources` to `zig run` to suppress the entire `<resources>` block
and skip every tier for that invocation. This is useful when you want to run
a workflow with a clean slate — no global or project-wide context bleeding
into the system prompt.

```bash
zig run cover-letter --no-resources
```

## See Also

- `zig docs zwf` — the inline `resources` field in the `.zwf` format
- `zig man run` — `--no-resources` and the run model
