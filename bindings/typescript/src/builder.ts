import type {
  Pattern,
  RunOutput,
  ValidationResult,
  WorkflowInfo,
} from "./types.js";
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
 * // Validate a .zug file
 * const result = await new ZigBuilder()
 *   .validate("workflow.zug");
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
   * @param workflow - Workflow name or path to a .zug file
   * @param prompt - Optional additional context prompt
   *
   * @example
   * ```ts
   * const output = await new ZigBuilder().run("deploy-pipeline");
   * console.log(output);
   * ```
   */
  async run(workflow: string, prompt?: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "run", workflow];
    if (prompt) args.push(prompt);
    return execZig(this._bin, args);
  }

  /**
   * Execute a workflow interactively with inherited stdio.
   *
   * @param workflow - Workflow name or path to a .zug file
   * @param prompt - Optional additional context prompt
   */
  async runInteractive(workflow: string, prompt?: string): Promise<void> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "run", workflow];
    if (prompt) args.push(prompt);
    return runZig(this._bin, args);
  }

  /**
   * Execute a workflow and stream stdout lines as they arrive.
   *
   * @param workflow - Workflow name or path to a .zug file
   * @param prompt - Optional additional context prompt
   *
   * @example
   * ```ts
   * for await (const line of new ZigBuilder().stream("my-workflow")) {
   *   console.log(line);
   * }
   * ```
   */
  async *stream(workflow: string, prompt?: string): AsyncGenerator<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "run", workflow];
    if (prompt) args.push(prompt);
    yield* streamZig(this._bin, args);
  }

  /**
   * Start a workflow with bidirectional stdio piping.
   *
   * Returns a `StreamingSession` for sending input and reading output lines.
   *
   * @param workflow - Workflow name or path to a .zug file
   * @param prompt - Optional additional context prompt
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
  runStreaming(workflow: string, prompt?: string): StreamingSession {
    const args = [...this.buildGlobalArgs(), "run", workflow];
    if (prompt) args.push(prompt);
    return streamWithInput(this._bin, args, {
      autoCleanup: this._autoCleanup,
    });
  }

  /**
   * Validate a .zug workflow file.
   *
   * Returns the raw stdout from `zig validate`. The CLI exits with code 0
   * on a valid workflow and non-zero on validation errors; in the latter case
   * this method throws `ZigError` with the validation output on stderr.
   *
   * @param workflow - Path to the .zug file to validate
   *
   * @example
   * ```ts
   * try {
   *   const msg = await new ZigBuilder().validate("deploy.zug");
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
   * Generate a .zug workflow file from a natural language description.
   *
   * Runs with inherited stdio since the describe process is interactive.
   *
   * @param prompt - Natural language description of the workflow
   * @param output - Output file path (defaults to workflow.zug)
   */
  async describe(prompt: string, output?: string): Promise<void> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "describe", prompt];
    if (output) args.push("--output", output);
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
   * Show a manual page topic.
   *
   * @param topic - Topic name (e.g., "run", "zug", "patterns"). Omit to list all topics.
   *
   * @example
   * ```ts
   * const content = await new ZigBuilder().man("zug");
   * console.log(content);
   * ```
   */
  async man(topic?: string): Promise<string> {
    await this.preflight();
    const args = [...this.buildGlobalArgs(), "man"];
    if (topic) args.push(topic);
    return execZig(this._bin, args);
  }
}
