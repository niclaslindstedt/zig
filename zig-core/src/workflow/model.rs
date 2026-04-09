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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

    // --- Input binding ---
    /// Bind this variable to an input source. Currently only `"prompt"` is
    /// supported, which assigns the CLI user prompt to this variable.
    #[serde(default)]
    pub from: Option<String>,

    // --- Constraints ---
    /// If true, the variable must have a non-empty value before execution.
    #[serde(default)]
    pub required: bool,

    /// Minimum string length (only valid for `type = "string"`).
    #[serde(default)]
    pub min_length: Option<u32>,

    /// Maximum string length (only valid for `type = "string"`).
    #[serde(default)]
    pub max_length: Option<u32>,

    /// Minimum numeric value (only valid for `type = "number"`).
    #[serde(default)]
    pub min: Option<f64>,

    /// Maximum numeric value (only valid for `type = "number"`).
    #[serde(default)]
    pub max: Option<f64>,

    /// Regex pattern the value must match (only valid for `type = "string"`).
    #[serde(default)]
    pub pattern: Option<String>,

    /// Restrict value to one of these specific values.
    #[serde(default)]
    pub allowed_values: Option<Vec<toml::Value>>,
}

/// Supported variable types.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VarType {
    #[default]
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

    /// Output format override: "text", "json", "json-pretty", "stream-json", "native-json".
    /// When set, maps to `-o <FORMAT>` on zag and overrides the `json` bool field.
    #[serde(default)]
    pub output: Option<String>,

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

    // --- Context injection ---
    /// Session IDs to inject as context (beyond depends_on).
    /// Maps to `--context <SESSION_ID>` flags on zag.
    #[serde(default)]
    pub context: Vec<String>,

    /// Path to a plan file to prepend as context.
    /// Maps to `--plan <PATH>` on zag.
    #[serde(default)]
    pub plan: Option<String>,

    /// Per-step MCP configuration (JSON string or file path, Claude only).
    /// Maps to `--mcp-config <CONFIG>` on zag.
    #[serde(default)]
    pub mcp_config: Option<String>,

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

    // --- Command step types ---
    /// Zag command to invoke for this step. Default (None) uses `zag run`.
    /// Other options: "review", "plan", "pipe", "collect", "summary".
    #[serde(default)]
    pub command: Option<StepCommand>,

    /// Review uncommitted changes (only valid when `command = "review"`).
    #[serde(default)]
    pub uncommitted: bool,

    /// Base branch for review diff (only valid when `command = "review"`).
    #[serde(default)]
    pub base: Option<String>,

    /// Specific commit to review (only valid when `command = "review"`).
    #[serde(default)]
    pub commit: Option<String>,

    /// Title for the review (only valid when `command = "review"`).
    #[serde(default)]
    pub title: Option<String>,

    /// Output path for generated plan (only valid when `command = "plan"`).
    #[serde(default)]
    pub plan_output: Option<String>,

    /// Additional instructions for plan generation (only valid when `command = "plan"`).
    #[serde(default)]
    pub instructions: Option<String>,
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

/// Zag command type for a step. When set, changes which zag subcommand
/// is invoked instead of the default `zag run`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepCommand {
    /// Code review: `zag review`.
    Review,
    /// Implementation plan generation: `zag plan`.
    Plan,
    /// Chain session results into new agent: `zag pipe`.
    Pipe,
    /// Gather results from multiple sessions: `zag collect`.
    Collect,
    /// Log-based summary/stats: `zag summary`.
    Summary,
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
