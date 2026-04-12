import { readFile } from "node:fs/promises";
import type { Workflow, Step, Variable, VarType } from "./types.js";
import { ZigError } from "./types.js";

/**
 * Parse a TOML .zug workflow string into a typed Workflow object.
 *
 * This is a lightweight parser that handles the subset of TOML used by
 * .zug files. For full TOML parsing, use the `zig validate` command
 * via {@link ZigBuilder.validate} or a dedicated TOML library.
 *
 * The canonical parser lives in zig-core (Rust); this TypeScript parser
 * covers the common case for read-only inspection of workflow files
 * from Node.js without spawning the CLI.
 *
 * @param content - Raw TOML string from a .zug file
 * @returns Parsed Workflow object
 * @throws ZigError if the content cannot be parsed
 */
export function parseWorkflow(content: string): Workflow {
  // We parse the TOML structure manually to avoid external dependencies.
  // .zug files use a predictable subset: [workflow], [vars.*], [[step]].
  const workflow: Workflow = {
    workflow: { name: "", description: "", tags: [] },
    vars: {},
    steps: [],
  };

  const lines = content.split("\n");
  let currentSection: "root" | "workflow" | "vars" | "step" = "root";
  let currentVarName: string | null = null;
  let currentStep: Partial<Step> | null = null;

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const trimmed = raw.trim();

    // Skip empty lines and comments
    if (!trimmed || trimmed.startsWith("#")) continue;

    // Section headers
    if (trimmed === "[workflow]") {
      currentSection = "workflow";
      currentVarName = null;
      currentStep = null;
      continue;
    }

    const varMatch = /^\[vars\.(\w+)\]$/.exec(trimmed);
    if (varMatch) {
      currentSection = "vars";
      currentVarName = varMatch[1];
      if (!workflow.vars[currentVarName]) {
        workflow.vars[currentVarName] = {
          type: "string",
          description: "",
        };
      }
      currentStep = null;
      continue;
    }

    if (trimmed === "[[step]]") {
      currentSection = "step";
      currentVarName = null;
      currentStep = createDefaultStep();
      workflow.steps.push(currentStep as Step);
      continue;
    }

    // Key-value pairs
    const kvMatch = /^(\w+)\s*=\s*(.+)$/.exec(trimmed);
    if (!kvMatch) continue;

    const [, key, rawValue] = kvMatch;
    const value = parseTomlValue(rawValue.trim());

    switch (currentSection) {
      case "workflow":
        if (key === "name") workflow.workflow.name = String(value);
        else if (key === "description") workflow.workflow.description = String(value);
        else if (key === "tags") workflow.workflow.tags = toStringArray(value);
        break;

      case "vars":
        if (currentVarName && workflow.vars[currentVarName]) {
          const v = workflow.vars[currentVarName];
          if (key === "type") v.type = String(value) as VarType;
          else if (key === "default") v.default = value;
          else if (key === "description") v.description = String(value);
          else if (key === "from") v.from = String(value);
          else if (key === "required") v.required = Boolean(value);
          else if (key === "min_length") v.min_length = Number(value);
          else if (key === "max_length") v.max_length = Number(value);
          else if (key === "min") v.min = Number(value);
          else if (key === "max") v.max = Number(value);
          else if (key === "pattern") v.pattern = String(value);
        }
        break;

      case "step":
        if (currentStep) {
          assignStepField(currentStep, key, value);
        }
        break;
    }
  }

  return workflow;
}

/**
 * Parse a .zug workflow file from disk.
 *
 * @param path - Path to the .zug file
 * @returns Parsed Workflow object
 * @throws ZigError if the file cannot be read or parsed
 */
export async function parseWorkflowFile(path: string): Promise<Workflow> {
  let content: string;
  try {
    content = await readFile(path, "utf-8");
  } catch (err) {
    throw new ZigError(
      `Failed to read workflow file '${path}': ${err instanceof Error ? err.message : String(err)}`,
      null,
      "",
    );
  }
  return parseWorkflow(content);
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function createDefaultStep(): Partial<Step> {
  return {
    name: "",
    prompt: "",
    depends_on: [],
    inject_context: false,
    json: false,
    saves: {},
    tags: [],
    description: "",
    interactive: false,
    auto_approve: false,
    add_dirs: [],
    env: {},
    files: [],
    context: [],
    worktree: false,
    uncommitted: false,
  };
}

function assignStepField(step: Partial<Step>, key: string, value: unknown): void {
  switch (key) {
    case "name": step.name = String(value); break;
    case "prompt": step.prompt = String(value); break;
    case "provider": step.provider = String(value); break;
    case "model": step.model = String(value); break;
    case "depends_on": step.depends_on = toStringArray(value); break;
    case "inject_context": step.inject_context = Boolean(value); break;
    case "condition": step.condition = String(value); break;
    case "json": step.json = Boolean(value); break;
    case "json_schema": step.json_schema = String(value); break;
    case "output": step.output = String(value); break;
    case "timeout": step.timeout = String(value); break;
    case "tags": step.tags = toStringArray(value); break;
    case "on_failure": step.on_failure = String(value) as Step["on_failure"]; break;
    case "max_retries": step.max_retries = Number(value); break;
    case "next": step.next = String(value); break;
    case "system_prompt": step.system_prompt = String(value); break;
    case "max_turns": step.max_turns = Number(value); break;
    case "description": step.description = String(value); break;
    case "interactive": step.interactive = Boolean(value); break;
    case "auto_approve": step.auto_approve = Boolean(value); break;
    case "root": step.root = String(value); break;
    case "add_dirs": step.add_dirs = toStringArray(value); break;
    case "files": step.files = toStringArray(value); break;
    case "context": step.context = toStringArray(value); break;
    case "plan": step.plan = String(value); break;
    case "mcp_config": step.mcp_config = String(value); break;
    case "worktree": step.worktree = Boolean(value); break;
    case "sandbox": step.sandbox = String(value); break;
    case "race_group": step.race_group = String(value); break;
    case "retry_model": step.retry_model = String(value); break;
    case "command": step.command = String(value) as Step["command"]; break;
    case "uncommitted": step.uncommitted = Boolean(value); break;
    case "base": step.base = String(value); break;
    case "commit": step.commit = String(value); break;
    case "title": step.title = String(value); break;
    case "plan_output": step.plan_output = String(value); break;
    case "instructions": step.instructions = String(value); break;
  }
}

/** Parse a simple TOML value (string, number, boolean, array). */
function parseTomlValue(raw: string): unknown {
  // Quoted string
  if ((raw.startsWith('"') && raw.endsWith('"')) ||
      (raw.startsWith("'") && raw.endsWith("'"))) {
    return raw.slice(1, -1);
  }
  // Multi-line strings starting with triple quotes — take the rest as-is
  if (raw.startsWith('"""') || raw.startsWith("'''")) {
    return raw.slice(3, raw.length - 3);
  }
  // Boolean
  if (raw === "true") return true;
  if (raw === "false") return false;
  // Array
  if (raw.startsWith("[") && raw.endsWith("]")) {
    const inner = raw.slice(1, -1).trim();
    if (!inner) return [];
    return inner.split(",").map((s) => parseTomlValue(s.trim()));
  }
  // Number
  const num = Number(raw);
  if (!isNaN(num)) return num;
  // Fallback: return as string
  return raw;
}

function toStringArray(value: unknown): string[] {
  if (Array.isArray(value)) return value.map(String);
  if (typeof value === "string") return [value];
  return [];
}
