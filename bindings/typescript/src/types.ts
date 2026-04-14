// ---------------------------------------------------------------------------
// Workflow model types — mirrors zig-core/src/workflow/model.rs
// ---------------------------------------------------------------------------

/** A complete workflow definition parsed from a `.zwf` file. */
export interface Workflow {
  workflow: WorkflowMeta;
  /** Reusable role definitions that can be referenced by steps. */
  roles: Record<string, Role>;
  vars: Record<string, Variable>;
  steps: Step[];
}

/** A reusable role definition that can be referenced by steps. */
export interface Role {
  /** Inline system prompt for this role. Supports ${var} references. */
  system_prompt?: string;
  /** Path to a file containing the system prompt (relative to the .zwf file). */
  system_prompt_file?: string;
}

/** Workflow-level metadata. */
export interface WorkflowMeta {
  name: string;
  description: string;
  tags: string[];
  /** Workflow version string (e.g., "1.0.0"). */
  version?: string;
  /** Default provider for all steps (claude, codex, gemini, copilot, ollama). Steps can override. */
  provider?: string;
  /** Default model for all steps (small, medium, large, or specific name). Steps can override. */
  model?: string;
  /**
   * Reference files advertised to every step's agent through its system
   * prompt. The agent reads them on demand with its file tools. Supports a
   * bare path string or a detailed `{ path, name?, description?, required? }`
   * form.
   */
  resources: ResourceSpec[];
}

/**
 * Inline resource specification.
 *
 * Strings are interpreted as bare paths relative to the `.zwf` file. Use the
 * detailed form to attach a name, description, or required flag.
 */
export type ResourceSpec =
  | string
  | {
      /** Path relative to the workflow file. */
      path: string;
      /** Display name (defaults to the file's basename). */
      name?: string;
      /** Description rendered alongside the path in the system prompt. */
      description?: string;
      /** When true, missing files cause the run to fail. */
      required?: boolean;
    };

/** A workflow variable — shared state between steps. */
export interface Variable {
  /** Variable type: "string", "number", "bool", or "json". */
  type: VarType;
  /** Default value. If absent, the variable must be provided at runtime. */
  default?: unknown;
  /** Path to a file whose contents become the default value (relative to .zwf file). */
  default_file?: string;
  /** Human-readable description of this variable's purpose. */
  description: string;

  // --- Input binding ---
  /** Bind this variable to an input source. Currently only "prompt" is supported. */
  from?: string;

  // --- Constraints ---
  /** If true, the variable must have a non-empty value before execution. */
  required?: boolean;
  /** Minimum string length (only valid for type = "string"). */
  min_length?: number;
  /** Maximum string length (only valid for type = "string"). */
  max_length?: number;
  /** Minimum numeric value (only valid for type = "number"). */
  min?: number;
  /** Maximum numeric value (only valid for type = "number"). */
  max?: number;
  /** Regex pattern the value must match (only valid for type = "string"). */
  pattern?: string;
  /** Restrict value to one of these specific values. */
  allowed_values?: unknown[];
}

/** Supported variable types. */
export type VarType = "string" | "number" | "bool" | "json";

/** A single workflow step — one agent invocation. */
export interface Step {
  /** Unique step identifier. */
  name: string;
  /** Prompt template sent to the agent. May contain ${var_name} references. */
  prompt: string;
  /** Zag provider to use (claude, codex, gemini, copilot, ollama). */
  provider?: string;
  /** Model name or size alias (small, medium, large). */
  model?: string;
  /** Steps that must complete before this step starts. */
  depends_on: string[];
  /** If true, dependency outputs are automatically injected into the prompt. */
  inject_context: boolean;
  /** Condition expression that must evaluate to true for this step to run. */
  condition?: string;

  // --- Output ---
  /** Request structured JSON output from the agent. */
  json: boolean;
  /** JSON schema to validate agent output against (implies json = true). */
  json_schema?: string;
  /** Output format override: "text", "json", "json-pretty", "stream-json", "native-json". */
  output?: string;
  /** Map of variable names to JSONPath-like selectors to save from output. */
  saves: Record<string, string>;

  // --- Timeouts and failure ---
  /** Step timeout (e.g., "5m", "30s", "1h"). */
  timeout?: string;
  /** Tags applied to the spawned zag session. */
  tags: string[];
  /** Behavior on step failure: "fail" (default), "continue", or "retry". */
  on_failure?: FailurePolicy;
  /** Maximum retry attempts when on_failure = "retry". */
  max_retries?: number;
  /** Explicit next step to jump to after completion (enables loops). */
  next?: string;

  // --- Agent configuration ---
  /** System prompt override for this step's agent. Mutually exclusive with role. */
  system_prompt?: string;
  /** Role name or ${var} reference — resolved to a role from [roles] at runtime. Mutually exclusive with system_prompt. */
  role?: string;
  /** Maximum number of agentic turns for this step. */
  max_turns?: number;
  /** Human-readable description of this step's purpose. */
  description: string;

  // --- Execution environment ---
  /** Spawn a long-lived interactive session (FIFO-based). */
  interactive: boolean;
  /** Auto-approve all agent actions (skip permission prompts). */
  auto_approve: boolean;
  /** Working directory override for this step's agent. */
  root?: string;
  /** Additional directories to include in the agent's scope. */
  add_dirs: string[];
  /** Per-step environment variables. */
  env: Record<string, string>;
  /** Files to attach to the agent prompt. */
  files: string[];

  /**
   * Step-level reference files advertised to the agent's system prompt. These
   * are merged with workflow-level resources and global / cwd tiers at run
   * time. See [WorkflowMeta.resources] for the spec shape.
   */
  resources: ResourceSpec[];

  // --- Context injection ---
  /** Session IDs to inject as context (beyond depends_on). */
  context: string[];
  /** Path to a plan file to prepend as context. */
  plan?: string;
  /** Per-step MCP configuration (JSON string or file path, Claude only). */
  mcp_config?: string;

  // --- Isolation ---
  /** If true, run this step in an isolated git worktree. */
  worktree: boolean;
  /** Docker sandbox name. If set, the step runs inside a sandbox. */
  sandbox?: string;

  // --- Advanced orchestration ---
  /** Race group name. Steps sharing a race_group run in parallel; first wins. */
  race_group?: string;
  /** Model to use when retrying (enables escalation to a larger model). */
  retry_model?: string;

  // --- Command step types ---
  /** Zag command to invoke: "review", "plan", "pipe", "collect", "summary". */
  command?: StepCommand;
  /** Review uncommitted changes (only valid when command = "review"). */
  uncommitted: boolean;
  /** Base branch for review diff (only valid when command = "review"). */
  base?: string;
  /** Specific commit to review (only valid when command = "review"). */
  commit?: string;
  /** Title for the review (only valid when command = "review"). */
  title?: string;
  /** Output path for generated plan (only valid when command = "plan"). */
  plan_output?: string;
  /** Additional instructions for plan generation (only valid when command = "plan"). */
  instructions?: string;
}

/** What to do when a step fails. */
export type FailurePolicy = "fail" | "continue" | "retry";

/** Zag command type for a step. */
export type StepCommand = "review" | "plan" | "pipe" | "collect" | "summary";

/** Orchestration pattern for workflow creation. */
export type Pattern =
  | "sequential"
  | "fan-out"
  | "generator-critic"
  | "coordinator-dispatcher"
  | "hierarchical-decomposition"
  | "human-in-the-loop"
  | "inter-agent-communication";

// ---------------------------------------------------------------------------
// Execution output types — events emitted during `zig run`
// ---------------------------------------------------------------------------

/** Output from a zig run session. */
export interface RunOutput {
  /** Workflow name that was executed. */
  workflow: string;
  /** Whether the workflow completed successfully. */
  success: boolean;
  /** Per-step results, keyed by step name. */
  steps: Record<string, StepResult>;
  /** Total execution duration in milliseconds. */
  duration_ms: number | null;
  /** Error message if the run failed. */
  error?: string;
}

/** Result of a single workflow step execution. */
export interface StepResult {
  /** Step name. */
  name: string;
  /** Whether the step completed successfully. */
  success: boolean;
  /** Step output (text or parsed JSON). */
  output: string | null;
  /** Error message if the step failed. */
  error?: string;
  /** Duration of this step in milliseconds. */
  duration_ms: number | null;
  /** Provider used for this step. */
  provider?: string;
  /** Model used for this step. */
  model?: string;
}

/** Validation result from `zig validate`. */
export interface ValidationResult {
  /** Whether the workflow is valid. */
  valid: boolean;
  /** Workflow name (if parseable). */
  workflow_name?: string;
  /** Number of steps in the workflow. */
  step_count?: number;
  /** Validation error messages, if any. */
  errors: string[];
}

/** Summary info for a workflow returned by `zig workflow list`. */
export interface WorkflowInfo {
  /** Workflow file name or path. */
  name: string;
  /** Workflow description. */
  description: string;
  /** Number of steps. */
  step_count: number;
  /** Workflow tags. */
  tags: string[];
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/** Error thrown when the zig process fails. */
export class ZigError extends Error {
  constructor(
    message: string,
    public readonly exitCode: number | null,
    public readonly stderr: string,
  ) {
    super(message);
    this.name = "ZigError";
  }
}

/** Error thrown when the zig CLI version is too old for a requested feature. */
export class ZigVersionError extends ZigError {
  constructor(
    message: string,
    public readonly requiredVersion: string,
    public readonly installedVersion: string,
  ) {
    super(message, null, "");
    this.name = "ZigVersionError";
  }
}
