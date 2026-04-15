import { readFile } from "node:fs/promises";
import type {
  Workflow,
  Role,
  Step,
  Variable,
  VarType,
  StorageSpec,
  StorageFileHint,
  StorageKind,
} from "./types.js";
import { ZigError } from "./types.js";

// ---------------------------------------------------------------------------
// Zag session name utilities
// ---------------------------------------------------------------------------

/**
 * Compute the zag session name that zig assigns to a workflow step.
 *
 * Zig names each zag session deterministically as `zig-{workflowName}-{stepName}`.
 * Use this to construct session identifiers for `@nlindstedt/zag-agent` without
 * parsing CLI output.
 *
 * @param workflowName - The workflow name (from `workflow.workflow.name`)
 * @param stepName - The step name (from `step.name`)
 * @returns The zag session name, e.g. `"zig-deploy-lint"`
 *
 * @example
 * ```ts
 * import { zagSessionName } from "@nlindstedt/zig-workflow";
 *
 * const name = zagSessionName("deploy", "lint");
 * // "zig-deploy-lint"
 * ```
 */
export function zagSessionName(workflowName: string, stepName: string): string {
  return `zig-${workflowName}-${stepName}`;
}

/**
 * Extract all zag session names from a parsed workflow.
 *
 * Returns a map of step name → zag session name for every step in the workflow.
 * Use these session names with `@nlindstedt/zag-agent` to resume, message, or
 * control individual agent sessions spawned by zig.
 *
 * @param workflow - A parsed `Workflow` object (from `parseWorkflow` or `parseWorkflowFile`)
 * @returns Record mapping step names to their zag session names
 *
 * @example
 * ```ts
 * import { parseWorkflowFile, zagSessionNames } from "@nlindstedt/zig-workflow";
 *
 * const wf = await parseWorkflowFile("deploy.zwf");
 * const sessions = zagSessionNames(wf);
 * // { lint: "zig-deploy-lint", test: "zig-deploy-test", deploy: "zig-deploy-deploy" }
 * ```
 */
export function zagSessionNames(workflow: Workflow): Record<string, string> {
  const result: Record<string, string> = {};
  for (const step of workflow.steps) {
    result[step.name] = zagSessionName(workflow.workflow.name, step.name);
  }
  return result;
}

/**
 * Parse a TOML .zwf workflow string into a typed Workflow object.
 *
 * This is a lightweight parser that handles the subset of TOML used by
 * .zwf files. For full TOML parsing, use the `zig validate` command
 * via {@link ZigBuilder.validate} or a dedicated TOML library.
 *
 * The canonical parser lives in zig-core (Rust); this TypeScript parser
 * covers the common case for read-only inspection of workflow files
 * from Node.js without spawning the CLI.
 *
 * @param content - Raw TOML string from a .zwf file
 * @returns Parsed Workflow object
 * @throws ZigError if the content cannot be parsed
 */
export function parseWorkflow(content: string): Workflow {
  // We parse the TOML structure manually to avoid external dependencies.
  // .zwf files use a predictable subset: [workflow], [vars.*], [[step]].
  const workflow: Workflow = {
    workflow: { name: "", description: "", tags: [], resources: [] },
    roles: {},
    vars: {},
    steps: [],
    storage: {},
  };

  const lines = content.split("\n");
  let currentSection:
    | "root"
    | "workflow"
    | "roles"
    | "vars"
    | "step"
    | "storage"
    | "storage_file" = "root";
  let currentRoleName: string | null = null;
  let currentVarName: string | null = null;
  let currentStep: Partial<Step> | null = null;
  let currentStorageName: string | null = null;
  let currentStorageFile: StorageFileHint | null = null;

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

    const roleMatch = /^\[roles\.(\w+)\]$/.exec(trimmed);
    if (roleMatch) {
      currentSection = "roles";
      currentRoleName = roleMatch[1];
      if (!workflow.roles[currentRoleName]) {
        workflow.roles[currentRoleName] = {};
      }
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
      currentRoleName = null;
      currentStep = null;
      continue;
    }

    const storageMatch = /^\[storage\.(\w+)\]$/.exec(trimmed);
    if (storageMatch) {
      currentSection = "storage";
      currentStorageName = storageMatch[1];
      if (!workflow.storage) workflow.storage = {};
      if (!workflow.storage[currentStorageName]) {
        workflow.storage[currentStorageName] = {
          type: "folder",
          path: "",
          files: [],
        };
      }
      currentStorageFile = null;
      currentRoleName = null;
      currentVarName = null;
      currentStep = null;
      continue;
    }

    const storageFileMatch = /^\[\[storage\.(\w+)\.files\]\]$/.exec(trimmed);
    if (storageFileMatch) {
      currentSection = "storage_file";
      currentStorageName = storageFileMatch[1];
      if (!workflow.storage) workflow.storage = {};
      if (!workflow.storage[currentStorageName]) {
        workflow.storage[currentStorageName] = {
          type: "folder",
          path: "",
          files: [],
        };
      }
      const spec = workflow.storage[currentStorageName];
      if (!spec.files) spec.files = [];
      currentStorageFile = { name: "" };
      spec.files.push(currentStorageFile);
      currentRoleName = null;
      currentVarName = null;
      currentStep = null;
      continue;
    }

    if (trimmed === "[[step]]") {
      currentSection = "step";
      currentVarName = null;
      currentStorageName = null;
      currentStorageFile = null;
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
        else if (key === "version") workflow.workflow.version = String(value);
        else if (key === "provider") workflow.workflow.provider = String(value);
        else if (key === "model") workflow.workflow.model = String(value);
        else if (key === "resources") workflow.workflow.resources = toStringArray(value);
        break;

      case "roles":
        if (currentRoleName && workflow.roles[currentRoleName]) {
          const r = workflow.roles[currentRoleName];
          if (key === "system_prompt") r.system_prompt = String(value);
          else if (key === "system_prompt_file") r.system_prompt_file = String(value);
        }
        break;

      case "vars":
        if (currentVarName && workflow.vars[currentVarName]) {
          const v = workflow.vars[currentVarName];
          if (key === "type") v.type = String(value) as VarType;
          else if (key === "default") v.default = value;
          else if (key === "default_file") v.default_file = String(value);
          else if (key === "description") v.description = String(value);
          else if (key === "from") v.from = String(value);
          else if (key === "required") v.required = Boolean(value);
          else if (key === "min_length") v.min_length = Number(value);
          else if (key === "max_length") v.max_length = Number(value);
          else if (key === "min") v.min = Number(value);
          else if (key === "max") v.max = Number(value);
          else if (key === "pattern") v.pattern = String(value);
          else if (key === "allowed_values") v.allowed_values = Array.isArray(value) ? value : [value];
        }
        break;

      case "step":
        if (currentStep) {
          assignStepField(currentStep, key, value);
        }
        break;

      case "storage":
        if (currentStorageName && workflow.storage?.[currentStorageName]) {
          const s = workflow.storage[currentStorageName];
          if (key === "type") s.type = String(value) as StorageKind;
          else if (key === "path") s.path = String(value);
          else if (key === "description") s.description = String(value);
          else if (key === "hint") s.hint = String(value);
        }
        break;

      case "storage_file":
        if (currentStorageFile) {
          if (key === "name") currentStorageFile.name = String(value);
          else if (key === "description") currentStorageFile.description = String(value);
        }
        break;
    }
  }

  return workflow;
}

/**
 * Parse a .zwf workflow file from disk.
 *
 * @param path - Path to the .zwf file
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
    resources: [],
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
    case "role": step.role = String(value); break;
    case "max_turns": step.max_turns = Number(value); break;
    case "description": step.description = String(value); break;
    case "interactive": step.interactive = Boolean(value); break;
    case "auto_approve": step.auto_approve = Boolean(value); break;
    case "root": step.root = String(value); break;
    case "add_dirs": step.add_dirs = toStringArray(value); break;
    case "files": step.files = toStringArray(value); break;
    case "resources": step.resources = toStringArray(value); break;
    case "storage": step.storage = toStringArray(value); break;
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
