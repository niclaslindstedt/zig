# zig workflow

Manage workflows — list, show, create, update, and delete `.zwf`/`.zwfz`
workflow files.

## Synopsis

```
zig workflow <command> [options]
```

## Commands

| Command              | Description                                                    |
|----------------------|----------------------------------------------------------------|
| `list`               | List available workflows                                       |
| `show <workflow>`    | Show details of a workflow                                     |
| `create [name]`      | Create a new workflow interactively with an AI agent           |
| `update <workflow>`  | Revise an existing workflow interactively with an AI agent     |
| `delete <workflow>`  | Delete a workflow file                                         |
| `pack <dir>`         | Pack a workflow directory into a `.zwfz` zip archive           |

## zig workflow list

List all `.zwf` and `.zwfz` workflow files discovered in the project-local
`.zig/workflows/` directory (walking up from the current directory to the git
root) and the global `~/.zig/workflows/` directory.

Displays a table with the workflow name, description, step count, and file
path. When a project-local workflow has the same filename as a global
workflow, the local copy takes precedence and is marked in the output with a
trailing `*`; the global version it shadows is hidden from the listing.

```bash
zig workflow list
```

### Options

| Option   | Description                                            |
|----------|--------------------------------------------------------|
| `--json` | Output the workflow list as JSON instead of a table    |

The JSON form is intended for scripts and external tooling: it emits an array
of objects with `name`, `description`, `step_count`, `path`, and (when a
local override is in effect) `is_local = true`.

## zig workflow show

Show detailed information about a workflow: metadata, variables, and steps.

```bash
zig workflow show my-workflow
zig workflow show workflows/deploy.zwf
```

### Output

- **Name** and **description**
- **Tags** for discovery and filtering
- **Variables** with types and defaults
- **Steps** with dependencies, conditions, and provider info

## zig workflow create

Create a new workflow interactively with an AI agent.

```
zig workflow create [name] [--output <path>] [--pattern <pattern>]
```

Launches an interactive zag session where an AI agent guides you through
designing a workflow. The agent understands the `.zwf` format, zag's
orchestration primitives, and common workflow patterns, and can read the
canonical example workflows from `~/.zig/examples/` (which are written to
disk at the start of the session).

### Options

| Option              | Short | Description                                   |
|---------------------|-------|-----------------------------------------------|
| `--output <path>`   | `-o`  | Output file path (defaults to `<name>.zwf` or `workflow.zwf`) |
| `--pattern <pattern>` | `-p` | Orchestration pattern to guide the agent     |

### Patterns

| Pattern                        | Description                                    |
|--------------------------------|------------------------------------------------|
| `sequential`                   | Steps run in order, each feeding the next      |
| `fan-out`                      | Parallel independent steps, then synthesize    |
| `generator-critic`             | Generate, evaluate, iterate until threshold    |
| `coordinator-dispatcher`       | Classify input, route to specialized handlers  |
| `hierarchical-decomposition`   | Break down into sub-tasks, delegate, synthesize|
| `human-in-the-loop`           | Automated steps with human approval gates      |
| `inter-agent-communication`    | Agents collaborate via shared variables        |

### Prerequisites

- `zag` must be installed and available on PATH

## zig workflow update

Revise an existing workflow interactively with an AI agent.

```
zig workflow update <workflow>
```

Launches an interactive zag session focused on editing an existing workflow in
place. The zig binary stages a safe scratch copy of the workflow for the
agent — the original file is not touched until the session succeeds:

1. **Plain `.zwf`** files are copied into a tempdir.
2. **Zipped `.zwfz`** bundles are unzipped into a tempdir.
3. The agent is given the absolute path to the staged file and instructed to
   edit it in place (never rename, move, or copy it elsewhere).
4. When the session ends, zig validates the edited file and moves the plain
   file (or re-zips the bundle) back over the original workflow path using an
   atomic rename via a sibling temp file.

If the session fails or the agent removes the staging file, the original
workflow is left untouched.

```bash
# Update a workflow in the global workflows directory
zig workflow update cover-letter

# Update a bundled workflow by path
zig workflow update ./workflows/healthcare.zwfz
```

### Prerequisites

- `zag` must be installed and available on PATH

## zig workflow pack

Pack a workflow directory into a `.zwfz` zip archive for distribution.

```
zig workflow pack <dir> [--output <path>]
```

The directory must contain exactly one workflow file (`.toml` or `.zwf`).
All files in the directory are included in the archive, preserving directory
structure. The resulting zip file works directly with `zig run`, `zig validate`,
and `zig workflow update`.

### Options

| Option              | Short | Description                                   |
|---------------------|-------|-----------------------------------------------|
| `--output <path>`   | `-o`  | Output file path (defaults to `<workflow-name>.zwfz`) |

### Example

```bash
# Pack a healthcare workflow with 20 prompt files
zig workflow pack examples/healthcare/ -o healthcare.zwfz

# Validate the packed archive
zig validate healthcare.zwfz

# Run it
zig run healthcare.zwfz "I have a headache"
```

## zig workflow delete

Delete a `.zwf` or `.zwfz` workflow file.

```bash
zig workflow delete my-workflow
zig workflow delete workflows/old-workflow.zwf
```

## Workflow Discovery

Workflows are discovered in these locations:

1. The project-local `.zig/workflows/` directory — located by walking up from
   the current working directory to the surrounding git root
2. The global `~/.zig/workflows/` directory

When referencing a workflow by name (e.g., `my-workflow`), zig tries these
extensions in order: `my-workflow`, `my-workflow.zwf`, `my-workflow.zwfz`,
first as a literal path (including under the project-local
`.zig/workflows/`), then under `~/.zig/workflows/`.

### Local Overrides

When a project-local workflow and a global workflow share the same filename,
the local copy wins for both `zig workflow list` and `zig run`. The
overridden global workflow is hidden from the listing. In `zig workflow list`
output, local overrides are marked with a trailing `*` next to the workflow
name and a `* local override` legend is printed at the bottom of the table.

## Examples

```bash
# List all workflows
zig workflow list

# List all workflows as JSON
zig workflow list --json

# Show details of a workflow
zig workflow show code-review

# Create a workflow interactively
zig workflow create my-workflow

# Create with a specific pattern
zig workflow create deploy --pattern sequential

# Update an existing workflow interactively
zig workflow update my-workflow

# Delete a workflow
zig workflow delete old-workflow
```

## See Also

- `zig docs zwf` — the `.zwf`/`.zwfz` file format
- `zig docs patterns` — orchestration patterns
