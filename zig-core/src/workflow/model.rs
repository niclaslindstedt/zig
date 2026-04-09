use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A complete workflow definition parsed from a `.zug` file.
///
/// A workflow describes a DAG of agent steps with shared variables,
/// conditional routing, and data flow between steps. It maps directly
/// to zag orchestration commands at execution time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow metadata.
    pub workflow: WorkflowMeta,

    /// Shared variables that flow between steps.
    /// Keys are variable names; values define type, default, and description.
    #[serde(default)]
    pub vars: HashMap<String, Variable>,

    /// Ordered list of workflow steps. Each step maps to a zag agent invocation.
    #[serde(default, rename = "step")]
    pub steps: Vec<Step>,
}

/// Workflow-level metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowMeta {
    /// Human-readable workflow name (used as filename if not overridden).
    pub name: String,

    /// Short description of what this workflow does.
    #[serde(default)]
    pub description: String,

    /// Tags for discovery and filtering.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A workflow variable — shared state between steps.
///
/// Variables can be referenced in step prompts via `${var_name}` and updated
/// by agents through the zig MCP server during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// Variable type: "string", "number", "bool", or "json".
    #[serde(rename = "type")]
    pub var_type: VarType,

    /// Default value (as a TOML value). If absent, the variable must be
    /// provided at runtime or set by a preceding step.
    #[serde(default)]
    pub default: Option<toml::Value>,

    /// Human-readable description of this variable's purpose.
    #[serde(default)]
    pub description: String,
}

/// Supported variable types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VarType {
    String,
    Number,
    Bool,
    Json,
}

/// A single workflow step — one agent invocation.
///
/// Each step maps to a `zag spawn` (or `zag exec` for terminal steps).
/// Steps form a DAG via `depends_on` and can conditionally execute based
/// on workflow variable values.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Step {
    /// Unique step identifier (used in `depends_on` references).
    pub name: String,

    /// Prompt template sent to the agent. May contain `${var_name}` references
    /// that are resolved against workflow variables before execution.
    pub prompt: String,

    /// Zag provider to use (claude, codex, gemini, copilot, ollama).
    /// Falls back to the project/global zag default if not set.
    #[serde(default)]
    pub provider: Option<String>,

    /// Model name or size alias (small, medium, large).
    #[serde(default)]
    pub model: Option<String>,

    /// Steps that must complete before this step starts.
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// If true, dependency outputs are automatically injected into the prompt.
    #[serde(default)]
    pub inject_context: bool,

    /// Condition expression that must evaluate to true for this step to run.
    /// Uses a simple expression language: `var < 8`, `status == "done"`, etc.
    /// If the condition is false, the step is skipped.
    #[serde(default)]
    pub condition: Option<String>,

    /// Request structured JSON output from the agent.
    #[serde(default)]
    pub json: bool,

    /// JSON schema to validate agent output against (implies `json = true`).
    #[serde(default)]
    pub json_schema: Option<String>,

    /// Map of variable names to save from this step's output.
    /// Values are JSONPath-like selectors (e.g., `"$.score"`).
    /// If the output is plain text, use `"$"` to capture the full output.
    #[serde(default)]
    pub saves: HashMap<String, String>,

    /// Step timeout (e.g., "5m", "30s", "1h").
    #[serde(default)]
    pub timeout: Option<String>,

    /// Tags applied to the spawned zag session.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Behavior on step failure: "fail" (default), "continue", or "retry".
    #[serde(default)]
    pub on_failure: Option<FailurePolicy>,

    /// Maximum retry attempts when `on_failure = "retry"`.
    #[serde(default)]
    pub max_retries: Option<u32>,

    /// Explicit next step to jump to after completion (enables loops).
    /// Without this, execution follows the DAG order.
    #[serde(default)]
    pub next: Option<String>,

    /// System prompt override for this step's agent.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Maximum number of agentic turns for this step.
    #[serde(default)]
    pub max_turns: Option<u32>,

    // --- Observability ---
    /// Human-readable description of this step's purpose.
    #[serde(default)]
    pub description: String,

    // --- Execution environment ---
    /// If true, spawn a long-lived interactive session (FIFO-based).
    /// Enables Human-in-the-Loop and Inter-Agent Communication patterns.
    #[serde(default)]
    pub interactive: bool,

    /// If true, auto-approve all agent actions (skip permission prompts).
    #[serde(default)]
    pub auto_approve: bool,

    /// Working directory override for this step's agent.
    #[serde(default)]
    pub root: Option<String>,

    /// Additional directories to include in the agent's scope.
    #[serde(default)]
    pub add_dirs: Vec<String>,

    /// Per-step environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Files to attach to the agent prompt.
    #[serde(default)]
    pub files: Vec<String>,

    // --- Isolation ---
    /// If true, run this step in an isolated git worktree.
    #[serde(default)]
    pub worktree: bool,

    /// Docker sandbox name. If set, the step runs inside a sandbox.
    #[serde(default)]
    pub sandbox: Option<String>,

    // --- Advanced orchestration ---
    /// Race group name. Steps sharing a race_group run in parallel;
    /// when the first completes, the rest are cancelled.
    #[serde(default)]
    pub race_group: Option<String>,

    /// Model to use when retrying this step (only applies when
    /// on_failure = "retry"). Enables escalation to a larger model.
    #[serde(default)]
    pub retry_model: Option<String>,
}

/// What to do when a step fails.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailurePolicy {
    /// Abort the workflow (default).
    #[default]
    Fail,
    /// Skip this step and continue.
    Continue,
    /// Retry the step up to `max_retries` times.
    Retry,
}

impl std::fmt::Display for VarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VarType::String => write!(f, "string"),
            VarType::Number => write!(f, "number"),
            VarType::Bool => write!(f, "bool"),
            VarType::Json => write!(f, "json"),
        }
    }
}
