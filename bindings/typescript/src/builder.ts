import type {
  Pattern,
  RunOutput,
  ValidationResult,
  WorkflowInfo,
} from "./types.js";

/** Output format for `zig run --dry-run`. */
export type DryRunFormat = "text" | "json";

/** Options accepted by the run / runInteractive / stream methods. */
export interface RunOptions {
  /** Additional context prompt injected into every step. */
  prompt?: string;
  /** Disable the `<resources>` block injected into each step's system prompt. */
  noResources?: boolean;
  /** Disable the `<memory>` block injected into each step's system prompt. */
  noMemory?: boolean;
  /** Disable the `<storage>` block and skip creating storage directories. */
  noStorage?: boolean;
  /**
   * Preview the resolved plan without invoking zag. Prints rendered prompts,
   * condition outcomes, and the exact zag invocation for each step.
   */
  dryRun?: boolean;
  /** Output format for `--dry-run` (`text` is the default). */
  format?: DryRunFormat;
}

/** Tier scope for `resources list`. */
export type ResourceScope = "all" | "global" | "cwd";

/** Target tier for `resources add` / `resources delete`. */
export interface ResourceTargetOptions {
  /** Place the resource in the global tier (~/.zig/resources/_shared/). */
  global?: boolean;
  /** Place the resource in the project tier (./.zig/resources/). */
  cwd?: boolean;
  /** Target a specific named workflow's global tier. */
  workflow?: string;
}
import { ZigError } from "./types.js";
import {
  defaultBin,
  execZig,
  runZig,
  streamZig,
  streamWithInput,
} from "./process.js";
import type { StreamingSession } from "./process.js";
import {
  checkVersion,
  type VersionRequirement,
} from "./version.js";

/**
 * Fluent builder for configuring and running zig workflow operations.
 *
 * @example
 * ```ts
 * import { ZigBuilder } from "@nlindstedt/zig-workflow";
 *
 * // Run a workflow
 * const output = await new ZigBuilder()
 *   .debug()
 *   .run("deploy-pipeline");
 *
 * // Validate a .zwf file
 * const result = await new ZigBuilder()
 *   .validate("workflow.zwf");
 * ```
 */
export class ZigBuilder {
  private _bin: string = defaultBin();
  private _debug = false;
  private _quiet = false;
  private _autoCleanup = false;

  /** Override the zig binary path (default: `ZIG_BIN` env or `"zig"`). */
  bin(path: string): this {
    this._bin = path;
    return this;
  }

  /** Enable debug logging. */
  debug(d = true): this {
    this._debug = d;
    return this;
  }

  /** Enable quiet mode (suppress all output except errors). */
  quiet(q = true): this {
    this._quiet = q;
    return this;
  }

  /**
   * Opt in to automatic orphan-process cleanup for streaming sessions.
   *
   * When enabled, any `StreamingSession` produced by this builder is tracked
   * and process-wide shutdown handlers are installed to SIGTERM every tracked
   * child on parent exit.
   */
  autoCleanup(enabled = true): this {
    this._autoCleanup = enabled;
    return this;
  }

  /** Collect version requirements for features added after the initial release. */
  private versionRequirements(): VersionRequirement[] {
    // All features are available since 0.4.0; extend here as new CLI
    // features require minimum versions.
    return [];
  }

  /** Run version preflight checks before spawning. */
  private async preflight(): Promise<void> {
    await checkVersion(this._bin, this.versionRequirements());
  }

  /** Build global CLI flags (--debug, --quiet). */
  private buildGlobalArgs(): string[] {
    const args: string[] = [];
    if (this._debug) args.push("--debug");
    if (this._quiet) args.push("--quiet");
    return args;
  }

  // -----------------------------------------------------------------------
  // Terminal methods — each spawns a `zig` subprocess
  // -----------------------------------------------------------------------

  /**
   * Execute a workflow by name or path.
   *
   * @param workflow - Workflow name or path to a .zwf file
   * @param promptOrOptions - Optional additional context prompt or options object
   *
   * @example
   * ```ts
   * const output = await new ZigBuilder().run("deploy-pipeline");
   * console.log(output);
   * ```
   */
  async run(
    workflow: string,
    promptOrOptions?: string | RunOptions,
  ): Promise<string> {
    await this.preflight();
    const args = [
      ...this.buildGlobalArgs(),
      ...this.buildRunArgs(workflow, promptOrOptions),
    ];
    return execZig(this._bin, args);
  }

  /**
   * Execute a workflow interactively with inherited stdio.
   *
   * @param workflow - Workflow name or path to a .zwf file
   * @param promptOrOptions - Optional additional context prompt or options object
   */
  async runInteractive(
    workflow: string,
    promptOrOptions?: string | RunOptions,
  ): Promise<void> {
    await this.preflight();
    const args = [
      ...this.buildGlobalArgs(),
      ...this.buildRunArgs(workflow, promptOrOptions),
    ];
    return runZig(this._bin, args);
  }

  /**
   * Execute a workflow and stream stdout lines as they arrive.
   *
   * @param workflow - Workflow name or path to a .zwf file
   * @param promptOrOptions - Optional additional context prompt or options object
   *
   * @example
   * ```ts
   * for await (const line of new ZigBuilder().stream("my-workflow")) {
   *   console.log(line);
   * }
   * ```
   */
  async *stream(
    workflow: string,
    promptOrOptions?: string | RunOptions,
  ): AsyncGenerator<string> {
    await this.preflight();
    const args = [
      ...this.buildGlobalArgs(),
      ...this.buildRunArgs(workflow, promptOrOptions),
    ];
    yield* streamZig(this._bin, args);
  }

  /**
   * Start a workflow with bidirectional stdio piping.
   *
   * Returns a `StreamingSession` for sending input and reading output lines.
   *
   * @param workflow - Workflow name or path to a .zwf file
   * @param promptOrOptions - Optional additional context prompt or options object
   *
   * @example
   * ```ts
   * const session = new ZigBuilder()
   *   .autoCleanup()
   *   .runStreaming("interactive-wf");
   *
   * for await (const line of session.lines()) {
   *   console.log(line);
   * }
   * await session.close({ timeout: "5s" });
   * ```
   */
  runStreaming(
    workflow: string,
    promptOrOptions?: string | RunOptions,
  ): StreamingSession {
    const args = [
      ...this.buildGlobalArgs(),
      ...this.buildRunArgs(workflow, promptOrOptions),
    ];
    return streamWithInput(this._bin, args, {
      autoCleanup: this._autoCleanup,
    });
  }

  /** Build the positional and flag arguments for a `zig run` invocation. */
  private buildRunArgs(
    workflow: string,
    promptOrOptions?: string | RunOptions,
  ): string[] {
    const opts: RunOptions =
      typeof promptOrOptions === "string"
        ? { prompt: promptOrOptions }
        : promptOrOptions ?? {};
    const args = ["run", workflow];
    if (opts.prompt) args.push(opts.prompt);
    if (opts.noResources) args.push("--no-resources");
    if (opts.noMemory) args.push("--no-memory");
    if (opts.noStorage) args.push("--no-storage");
    if (opts.dryRun) args.push("--dry-run");
    if (opts.format) args.push("--format", opts.format);
    return args;
  }

  /**
   * Validate a .zwf workflow file.
   *
   * Returns the raw stdout from `zig validate`. The CLI exits with code 0
   * on a valid workflow and non-zero on validation errors; in the latter case
   * this method throws `ZigError` with the validation output on stderr.
   *
   * @param workflow - Path to the .zwf file to validate
   *
   * @example
   * ```ts
   * try {
   *   const msg = await new ZigBuilder().validate("deploy.zwf");
   *   console.log(msg); // "workflow 'deploy' is valid (3 steps)"
   * } catch (err) {
   *   if (err instanceof ZigError) {
   *     console.error("Validation failed:", err.stderr);
   *   }
   * }
   * ```
   */
  async validate(workflow: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "validate", workflow];
    return execZig(this._bin, args);
  }

  /**
   * List available workflows.
   *
   * Returns the raw stdout from `zig workflow list`.
   *
   * @example
   * ```ts
   * const listing = await new ZigBuilder().workflowList();
   * console.log(listing);
   * ```
   */
  async workflowList(): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "workflow", "list"];
    return execZig(this._bin, args);
  }

  /**
   * Show details of a specific workflow.
   *
   * @param workflow - Name or path of the workflow to show
   *
   * @example
   * ```ts
   * const details = await new ZigBuilder().workflowShow("deploy");
   * console.log(details);
   * ```
   */
  async workflowShow(workflow: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "workflow", "show", workflow];
    return execZig(this._bin, args);
  }

  /**
   * Delete a workflow.
   *
   * @param workflow - Name or path of the workflow to delete
   */
  async workflowDelete(workflow: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "workflow", "delete", workflow];
    return execZig(this._bin, args);
  }

  /**
   * Create a new workflow interactively with an AI agent.
   *
   * Runs with inherited stdio since the creation process is interactive.
   *
   * @param options.name - Workflow name
   * @param options.output - Output file path
   * @param options.pattern - Orchestration pattern to use
   */
  async workflowCreate(options: {
    name?: string;
    output?: string;
    pattern?: Pattern;
  } = {}): Promise<void> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "workflow", "create"];
    if (options.name) args.push(options.name);
    if (options.output) args.push("--output", options.output);
    if (options.pattern) args.push("--pattern", options.pattern);
    return runZig(this._bin, args);
  }

  /**
   * Revise an existing workflow interactively with an AI agent.
   *
   * Runs with inherited stdio since the update process is interactive.
   *
   * @param workflow - Name or path of the workflow to update
   */
  async workflowUpdate(workflow: string): Promise<void> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "workflow", "update", workflow];
    return runZig(this._bin, args);
  }

  /**
   * Resume the most recent step's agent conversation from the latest
   * `zig run` in the current directory.
   *
   * Runs with inherited stdio so the resumed agent session attaches to
   * the terminal — interactively when no prompt is given, or driven by
   * the supplied follow-up prompt non-interactively.
   *
   * @param options.workflow - Filter to the most recent run for this workflow name
   * @param options.session - Resume a specific zig session by id or unique prefix (mutually exclusive with `workflow`)
   * @param options.prompt - Optional follow-up prompt to send into the resumed agent turn
   */
  async continueRun(
    options: { workflow?: string; session?: string; prompt?: string } = {},
  ): Promise<void> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "continue"];
    if (options.session) {
      args.push("--session", options.session);
    } else if (options.workflow) {
      args.push(options.workflow);
    }
    if (options.prompt) {
      args.push(options.prompt);
    }
    return runZig(this._bin, args);
  }

  /**
   * Tail a running or completed zig session.
   *
   * Runs with inherited stdio to display session output live.
   *
   * @param options.sessionId - Session ID (full UUID or unique prefix)
   * @param options.latest - Tail the most recently started session
   * @param options.active - Tail the most recently active session
   */
  async listen(options: {
    sessionId?: string;
    latest?: boolean;
    active?: boolean;
  } = {}): Promise<void> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "listen"];
    if (options.sessionId) {
      args.push(options.sessionId);
    } else if (options.latest) {
      args.push("--latest");
    } else if (options.active) {
      args.push("--active");
    }
    return runZig(this._bin, args);
  }

  /**
   * Pack a workflow directory into a .zwfz zip archive.
   *
   * @param path - Path to directory containing the workflow and its prompt files
   * @param output - Output file path (defaults to <workflow-name>.zwfz)
   *
   * @example
   * ```ts
   * await new ZigBuilder().workflowPack("./my-workflow");
   * ```
   */
  async workflowPack(path: string, output?: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "workflow", "pack", path];
    if (output) args.push("--output", output);
    return execZig(this._bin, args);
  }

  /**
   * Show a manual page topic.
   *
   * @param topic - Topic name (e.g., "run", "zwf", "patterns"). Omit to list all topics.
   *
   * @example
   * ```ts
   * const content = await new ZigBuilder().man("zwf");
   * console.log(content);
   * ```
   */
  async man(topic?: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "man"];
    if (topic) args.push(topic);
    return execZig(this._bin, args);
  }

  /**
   * Show a conceptual documentation topic (e.g. "zwf", "patterns", "dry-run").
   *
   * @param topic - Topic name. Omit to list all topics.
   */
  async docs(topic?: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "docs"];
    if (topic) args.push(topic);
    return execZig(this._bin, args);
  }

  /**
   * List discovered resources from one or more tiers.
   *
   * @param options.scope - "all" (default), "global", or "cwd"
   * @param options.workflow - Restrict the global tier to a specific workflow name
   */
  async resourcesList(
    options: { scope?: ResourceScope; workflow?: string } = {},
  ): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "resources", "list"];
    if (options.scope === "global") args.push("--global");
    if (options.scope === "cwd") args.push("--cwd");
    if (options.workflow) args.push("--workflow", options.workflow);
    return execZig(this._bin, args);
  }

  /**
   * Register a file as a resource in one of the tiers.
   *
   * @param file - Path to the source file to register
   * @param options - Target tier and optional rename
   */
  async resourcesAdd(
    file: string,
    options: ResourceTargetOptions & { name?: string } = {},
  ): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "resources", "add", file];
    if (options.global) args.push("--global");
    if (options.cwd) args.push("--cwd");
    if (options.workflow) args.push("--workflow", options.workflow);
    if (options.name) args.push("--name", options.name);
    return execZig(this._bin, args);
  }

  /**
   * Delete a resource by name from one of the tiers.
   *
   * @param name - Registered resource name to delete
   * @param options - Target tier
   */
  async resourcesDelete(
    name: string,
    options: ResourceTargetOptions = {},
  ): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "resources", "delete", name];
    if (options.global) args.push("--global");
    if (options.cwd) args.push("--cwd");
    if (options.workflow) args.push("--workflow", options.workflow);
    return execZig(this._bin, args);
  }

  /**
   * Print the absolute path and contents of a resource by name.
   *
   * @param name - Resource name to show
   * @param workflow - Optionally restrict the global tier to a specific workflow
   */
  async resourcesShow(name: string, workflow?: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "resources", "show", name];
    if (workflow) args.push("--workflow", workflow);
    return execZig(this._bin, args);
  }

  /**
   * Print the directories the collector would search for the current cwd.
   *
   * @param workflow - Optionally print directories for a specific workflow
   */
  async resourcesWhere(workflow?: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "resources", "where"];
    if (workflow) args.push("--workflow", workflow);
    return execZig(this._bin, args);
  }
}
