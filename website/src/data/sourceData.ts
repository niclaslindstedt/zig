// AUTO-GENERATED from Rust source — do not edit manually.
// To regenerate: npm run extract (from website/) or make extract-website-data
// Source files:
//   - zig-cli/Cargo.toml (version)
//   - zig-cli/src/cli.rs (commands, patterns)
//   - zig-core/src/workflow/model.rs (step fields, variable types)
//   - manpages/*.md (manpage topics)

// --- Types ---

export interface CommandData {
  name: string;
  description: string;
}

export interface PatternData {
  name: string;
  displayName: string;
  description: string;
}

export interface StepField {
  name: string;
  type: string;
  description: string;
}

// --- Data ---

export const version = "0.4.1";

export const commands: CommandData[] = [
  {
    "name": "run",
    "description": "Execute a .zug workflow file"
  },
  {
    "name": "workflow",
    "description": "Manage workflows (list, show, create, delete)"
  },
  {
    "name": "describe",
    "description": "Describe a workflow to an agent and generate a .zug file"
  },
  {
    "name": "validate",
    "description": "Validate a .zug workflow file"
  },
  {
    "name": "init",
    "description": "Initialize a new zig project in the current directory"
  },
  {
    "name": "man",
    "description": "Show manual pages for zig topics"
  },
  {
    "name": "listen",
    "description": "Tail a running or completed zig session"
  }
];

export const workflowSubcommands: CommandData[] = [
  {
    "name": "list",
    "description": "List available workflows"
  },
  {
    "name": "show",
    "description": "Show details of a workflow"
  },
  {
    "name": "delete",
    "description": "Delete a workflow file"
  },
  {
    "name": "create",
    "description": "Create a new workflow interactively with an AI agent"
  },
  {
    "name": "pack",
    "description": "Pack a workflow directory into a .zug zip archive"
  }
];

export const patterns: PatternData[] = [
  {
    "name": "sequential",
    "displayName": "Sequential",
    "description": "Steps run in order, each feeding the next"
  },
  {
    "name": "fan-out",
    "displayName": "Fan Out",
    "description": "Parallel independent steps, then synthesize"
  },
  {
    "name": "generator-critic",
    "displayName": "Generator Critic",
    "description": "Generate, evaluate, iterate until quality threshold"
  },
  {
    "name": "coordinator-dispatcher",
    "displayName": "Coordinator Dispatcher",
    "description": "Classify input, route to specialized handlers"
  },
  {
    "name": "hierarchical-decomposition",
    "displayName": "Hierarchical Decomposition",
    "description": "Break down into sub-tasks, delegate, synthesize"
  },
  {
    "name": "human-in-the-loop",
    "displayName": "Human In The Loop",
    "description": "Automated steps with human approval gates"
  },
  {
    "name": "inter-agent-communication",
    "displayName": "Inter Agent Communication",
    "description": "Agents collaborate via shared variables"
  }
];

export const stepFields: StepField[] = [
  {
    "name": "name",
    "type": "String",
    "description": "Unique step identifier (used in `depends_on` references)."
  },
  {
    "name": "prompt",
    "type": "String",
    "description": "Prompt template sent to the agent. May contain `${var_name}` references that are resolved against workflow variables before execution."
  },
  {
    "name": "provider",
    "type": "String?",
    "description": "Zag provider to use (claude, codex, gemini, copilot, ollama). Falls back to the project/global zag default if not set."
  },
  {
    "name": "model",
    "type": "String?",
    "description": "Model name or size alias (small, medium, large)."
  },
  {
    "name": "depends_on",
    "type": "list",
    "description": "Steps that must complete before this step starts."
  },
  {
    "name": "inject_context",
    "type": "bool",
    "description": "If true, dependency outputs are automatically injected into the prompt."
  },
  {
    "name": "condition",
    "type": "String?",
    "description": "Condition expression that must evaluate to true for this step to run. Uses a simple expression language: `var < 8`, `status == \"done\"`, etc. If the condition is false, the step is skipped."
  },
  {
    "name": "json",
    "type": "bool",
    "description": "Request structured JSON output from the agent."
  },
  {
    "name": "json_schema",
    "type": "String?",
    "description": "JSON schema to validate agent output against (implies `json = true`)."
  },
  {
    "name": "output",
    "type": "String?",
    "description": "Output format override: \"text\", \"json\", \"json-pretty\", \"stream-json\", \"native-json\". When set, maps to `-o <FORMAT>` on zag and overrides the `json` bool field."
  },
  {
    "name": "saves",
    "type": "HashMap<String",
    "description": "Map of variable names to save from this step's output. Values are JSONPath-like selectors (e.g., `\"$.score\"`). If the output is plain text, use `\"$\"` to capture the full output."
  },
  {
    "name": "timeout",
    "type": "String?",
    "description": "Step timeout (e.g., \"5m\", \"30s\", \"1h\")."
  },
  {
    "name": "tags",
    "type": "list",
    "description": "Tags applied to the spawned zag session."
  },
  {
    "name": "on_failure",
    "type": "FailurePolicy?",
    "description": "Behavior on step failure: \"fail\" (default), \"continue\", or \"retry\"."
  },
  {
    "name": "max_retries",
    "type": "u32?",
    "description": "Maximum retry attempts when `on_failure = \"retry\"`."
  },
  {
    "name": "next",
    "type": "String?",
    "description": "Explicit next step to jump to after completion (enables loops). Without this, execution follows the DAG order."
  },
  {
    "name": "system_prompt",
    "type": "String?",
    "description": "System prompt override for this step's agent. Mutually exclusive with `role`."
  },
  {
    "name": "role",
    "type": "String?",
    "description": "Role name or `${var}` reference — resolved to a role from `[roles]` at runtime. The role's system prompt is used as this step's system prompt. Mutually exclusive with `system_prompt`."
  },
  {
    "name": "max_turns",
    "type": "u32?",
    "description": "Maximum number of agentic turns for this step."
  },
  {
    "name": "description",
    "type": "String",
    "description": "Human-readable description of this step's purpose."
  },
  {
    "name": "interactive",
    "type": "bool",
    "description": "If true, spawn a long-lived interactive session (FIFO-based). Enables Human-in-the-Loop and Inter-Agent Communication patterns."
  },
  {
    "name": "auto_approve",
    "type": "bool",
    "description": "If true, auto-approve all agent actions (skip permission prompts)."
  },
  {
    "name": "root",
    "type": "String?",
    "description": "Working directory override for this step's agent."
  },
  {
    "name": "add_dirs",
    "type": "list",
    "description": "Additional directories to include in the agent's scope."
  },
  {
    "name": "env",
    "type": "HashMap<String",
    "description": "Per-step environment variables."
  },
  {
    "name": "files",
    "type": "list",
    "description": "Files to attach to the agent prompt."
  },
  {
    "name": "context",
    "type": "list",
    "description": "Session IDs to inject as context (beyond depends_on). Maps to `--context <SESSION_ID>` flags on zag."
  },
  {
    "name": "plan",
    "type": "String?",
    "description": "Path to a plan file to prepend as context. Maps to `--plan <PATH>` on zag."
  },
  {
    "name": "mcp_config",
    "type": "String?",
    "description": "Per-step MCP configuration (JSON string or file path, Claude only). Maps to `--mcp-config <CONFIG>` on zag."
  },
  {
    "name": "worktree",
    "type": "bool",
    "description": "If true, run this step in an isolated git worktree."
  },
  {
    "name": "sandbox",
    "type": "String?",
    "description": "Docker sandbox name. If set, the step runs inside a sandbox."
  },
  {
    "name": "race_group",
    "type": "String?",
    "description": "Race group name. Steps sharing a race_group run in parallel; when the first completes, the rest are cancelled."
  },
  {
    "name": "retry_model",
    "type": "String?",
    "description": "Model to use when retrying this step (only applies when on_failure = \"retry\"). Enables escalation to a larger model."
  },
  {
    "name": "command",
    "type": "StepCommand?",
    "description": "Zag command to invoke for this step. Default (None) uses `zag run`. Other options: \"review\", \"plan\", \"pipe\", \"collect\", \"summary\"."
  },
  {
    "name": "uncommitted",
    "type": "bool",
    "description": "Review uncommitted changes (only valid when `command = \"review\"`)."
  },
  {
    "name": "base",
    "type": "String?",
    "description": "Base branch for review diff (only valid when `command = \"review\"`)."
  },
  {
    "name": "commit",
    "type": "String?",
    "description": "Specific commit to review (only valid when `command = \"review\"`)."
  },
  {
    "name": "title",
    "type": "String?",
    "description": "Title for the review (only valid when `command = \"review\"`)."
  },
  {
    "name": "plan_output",
    "type": "String?",
    "description": "Output path for generated plan (only valid when `command = \"plan\"`)."
  },
  {
    "name": "instructions",
    "type": "String?",
    "description": "Additional instructions for plan generation (only valid when `command = \"plan\"`)."
  }
];

export const varTypes: string[] = ["string","number","bool","json"];

export const manpageTopics: string[] = ["conditions","describe","listen","patterns","run","validate","variables","workflow","zig","zug"];
