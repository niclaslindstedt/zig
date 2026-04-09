# zig describe

Generate a `.zug` workflow file from a natural language prompt.

## Synopsis

```
zig describe <prompt> [--output <path>]
```

## Description

Takes a natural language description of a workflow and launches a zag
interactive session that translates it into a `.zug` orchestration file.
Unlike `zig create`, which is fully interactive, `describe` starts from
a specific prompt and generates the workflow with less back-and-forth.

> **Note:** This command is not yet fully implemented.

## Arguments

| Argument | Description                                      |
|----------|--------------------------------------------------|
| `prompt` | Natural language description of the workflow     |

## Options

| Option            | Short | Description                                       |
|-------------------|-------|---------------------------------------------------|
| `--output <path>` | `-o`  | Output file path (defaults to `workflow.zug`)     |

## Examples

```bash
# Generate a workflow from a description
zig describe "review all PRs, run tests, and generate a summary report"

# Specify output path
zig describe "lint, test, and deploy" --output ci-pipeline.zug
```

## See Also

- `zig man create` — interactive workflow creation
- `zig man zug` — the `.zug` file format
