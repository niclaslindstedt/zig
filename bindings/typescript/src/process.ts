import { spawn, type ChildProcess } from "node:child_process";
import { createInterface } from "node:readline";
import { ZigError } from "./types.js";

/**
 * Parse a timeout value into milliseconds.
 *
 * Accepts a number (already in ms) or a humantime string with a unit
 * suffix: `ms`, `s`, `m`, or `h` (e.g. `"500ms"`, `"5s"`, `"1m"`, `"1h"`).
 * Throws `ZigError` on unparseable input.
 */
export function parseTimeoutMs(input: number | string): number {
  if (typeof input === "number") {
    if (!Number.isFinite(input) || input < 0) {
      throw new ZigError(`Invalid timeout: ${input}`, null, "");
    }
    return Math.floor(input);
  }
  const match = /^\s*(\d+(?:\.\d+)?)\s*(ms|s|m|h)\s*$/i.exec(input);
  if (!match) {
    throw new ZigError(
      `Invalid timeout string: "${input}" (expected e.g. "500ms", "5s", "1m", "1h")`,
      null,
      "",
    );
  }
  const value = Number.parseFloat(match[1]);
  const unit = match[2].toLowerCase();
  const multipliers: Record<string, number> = {
    ms: 1,
    s: 1_000,
    m: 60_000,
    h: 3_600_000,
  };
  return Math.floor(value * multipliers[unit]);
}

// ---------------------------------------------------------------------------
// Module-level orphan-cleanup registry.
// ---------------------------------------------------------------------------

const liveSessions = new Set<ChildProcess>();
let handlersInstalled = false;

function killAllLiveSessions(): void {
  for (const child of liveSessions) {
    try {
      child.kill("SIGTERM");
    } catch {
      // Child may already be dead or unreachable; ignore.
    }
  }
}

function ensureCleanupHandlersInstalled(): void {
  if (handlersInstalled) return;
  handlersInstalled = true;

  process.on("exit", killAllLiveSessions);

  const signals: Array<{ name: NodeJS.Signals; code: number }> = [
    { name: "SIGINT", code: 2 },
    { name: "SIGTERM", code: 15 },
    { name: "SIGHUP", code: 1 },
  ];
  for (const { name, code } of signals) {
    process.on(name, () => {
      killAllLiveSessions();
      process.exit(128 + code);
    });
  }

  process.on("uncaughtException", (err) => {
    killAllLiveSessions();
    throw err;
  });
}

/** Test-only: current number of sessions tracked for auto-cleanup. */
export function _getLiveSessionCount(): number {
  return liveSessions.size;
}

/** Default binary name — override with `ZIG_BIN` env var or builder option. */
export function defaultBin(): string {
  return process.env.ZIG_BIN ?? "zig";
}

/**
 * Run `zig` and collect stdout as a string.
 * Throws `ZigError` on non-zero exit.
 */
export async function execZig(
  bin: string,
  args: string[],
): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn(bin, args, { stdio: ["ignore", "pipe", "pipe"] });

    const stdoutChunks: Buffer[] = [];
    const stderrChunks: Buffer[] = [];

    child.stdout.on("data", (chunk: Buffer) => stdoutChunks.push(chunk));
    child.stderr.on("data", (chunk: Buffer) => stderrChunks.push(chunk));

    child.on("error", (err) => {
      reject(
        new ZigError(
          `Failed to spawn '${bin}': ${err.message}`,
          null,
          Buffer.concat(stderrChunks).toString(),
        ),
      );
    });

    child.on("close", (code) => {
      const stdout = Buffer.concat(stdoutChunks).toString();
      const stderr = Buffer.concat(stderrChunks).toString();

      if (code !== 0) {
        reject(
          new ZigError(
            `zig exited with code ${code}: ${stderr || stdout}`,
            code,
            stderr,
          ),
        );
        return;
      }

      resolve(stdout);
    });
  });
}

/**
 * Run `zig` in streaming mode and yield lines from stdout.
 */
export async function* streamZig(
  bin: string,
  args: string[],
): AsyncGenerator<string> {
  const child = spawn(bin, args, { stdio: ["ignore", "pipe", "pipe"] });

  const stderrChunks: Buffer[] = [];
  child.stderr.on("data", (chunk: Buffer) => stderrChunks.push(chunk));

  const rl = createInterface({ input: child.stdout });

  for await (const line of rl) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    yield trimmed;
  }

  const exitCode = await new Promise<number | null>((resolve) => {
    child.on("close", resolve);
  });

  if (exitCode !== 0) {
    const stderr = Buffer.concat(stderrChunks).toString();
    throw new ZigError(
      `zig exited with code ${exitCode}${stderr ? `: ${stderr}` : ""}`,
      exitCode,
      stderr,
    );
  }
}

/**
 * A live session with piped stdin and stdout for interactive workflows.
 */
export interface StreamingSession {
  /** Send a raw line to stdin. */
  send(message: string): void;

  /** Close stdin to signal no more input. */
  closeInput(): void;

  /** Async iterator over lines from stdout. */
  lines(): AsyncGenerator<string>;

  /** Whether the child process is still running. */
  readonly isRunning: boolean;

  /** Send SIGTERM to the child process. No-op if already exited. */
  terminate(): void;

  /** Wait for the process to exit. Throws ZigError on non-zero exit. */
  wait(): Promise<void>;

  /**
   * Gracefully stop the session.
   *
   * 1. Closes stdin.
   * 2. Waits up to half of `timeout` for the child to exit.
   * 3. Sends SIGTERM and waits the remaining half.
   * 4. Sends SIGKILL as a last resort.
   *
   * @param options.timeout Total budget. Number (ms) or humantime string.
   *   Defaults to 5000ms.
   */
  close(options?: { timeout?: number | string }): Promise<void>;
}

/** Options for {@link streamWithInput}. */
export interface StreamWithInputOptions {
  /** When true, the session is tracked for automatic orphan cleanup on parent exit. */
  autoCleanup?: boolean;
}

/**
 * Spawn `zig` with piped stdin and stdout for bidirectional communication.
 */
export function streamWithInput(
  bin: string,
  args: string[],
  options: StreamWithInputOptions = {},
): StreamingSession {
  const child = spawn(bin, args, { stdio: ["pipe", "pipe", "pipe"] });

  const stderrChunks: Buffer[] = [];
  child.stderr.on("data", (chunk: Buffer) => stderrChunks.push(chunk));

  let running = true;

  if (options.autoCleanup) {
    ensureCleanupHandlersInstalled();
    liveSessions.add(child);
  }

  child.on("exit", () => {
    running = false;
    liveSessions.delete(child);
  });

  const exited: Promise<void> = running
    ? new Promise((resolve) => {
        child.once("exit", () => resolve());
      })
    : Promise.resolve();

  let closingPromise: Promise<void> | null = null;

  return {
    get isRunning() {
      return running;
    },

    terminate() {
      if (running) {
        child.kill("SIGTERM");
      }
    },

    send(message: string) {
      child.stdin.write(message + "\n");
    },

    closeInput() {
      child.stdin.end();
    },

    async *lines(): AsyncGenerator<string> {
      const rl = createInterface({ input: child.stdout });
      for await (const line of rl) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        yield trimmed;
      }
    },

    wait(): Promise<void> {
      return new Promise((resolve, reject) => {
        child.on("close", (code) => {
          if (code !== 0) {
            const stderr = Buffer.concat(stderrChunks).toString();
            reject(
              new ZigError(
                `zig exited with code ${code}${stderr ? `: ${stderr}` : ""}`,
                code,
                stderr,
              ),
            );
          } else {
            resolve();
          }
        });
      });
    },

    close(opts: { timeout?: number | string } = {}): Promise<void> {
      if (closingPromise) return closingPromise;
      if (!running) return Promise.resolve();

      const totalMs = parseTimeoutMs(opts.timeout ?? 5000);
      const half = Math.max(50, Math.floor(totalMs / 2));

      closingPromise = (async () => {
        try {
          child.stdin.end();
        } catch {
          // stdin may already be closed.
        }

        if (!running) return;
        await raceWithTimeout(exited, half);

        if (!running) return;
        try {
          child.kill("SIGTERM");
        } catch {
          // ignore
        }
        await raceWithTimeout(exited, half);

        if (!running) return;
        try {
          child.kill("SIGKILL");
        } catch {
          // ignore
        }
        await exited;
      })();

      return closingPromise;
    },
  };
}

/** Resolve when `promise` settles or `ms` elapses, whichever comes first. */
function raceWithTimeout(promise: Promise<void>, ms: number): Promise<void> {
  return new Promise((resolve) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      resolve();
    }, ms);
    if (typeof timer.unref === "function") timer.unref();
    promise.then(() => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve();
    });
  });
}

/**
 * Run `zig` interactively with inherited stdio.
 * Returns when the process exits.
 */
export async function runZig(bin: string, args: string[]): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(bin, args, { stdio: "inherit" });

    child.on("error", (err) => {
      reject(new ZigError(`Failed to spawn '${bin}': ${err.message}`, null, ""));
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(new ZigError(`zig exited with code ${code}`, code, ""));
      } else {
        resolve();
      }
    });
  });
}
