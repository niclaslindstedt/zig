import { spawn } from "node:child_process";
import { ZigError, ZigVersionError } from "./types.js";

/** Parsed semver tuple. */
type SemVer = [number, number, number];

/** Parse a semver string like "0.4.1" into a numeric tuple. */
export function parseSemver(version: string): SemVer {
  const parts = version.trim().split(".");
  if (parts.length !== 3) {
    throw new ZigError(
      `Could not parse version "${version}": expected format "X.Y.Z"`,
      null,
      "",
    );
  }
  const nums = parts.map(Number);
  if (nums.some(isNaN)) {
    throw new ZigError(
      `Could not parse version "${version}": non-numeric components`,
      null,
      "",
    );
  }
  return nums as unknown as SemVer;
}

/** Compare two semver tuples. Returns -1 if a < b, 0 if equal, 1 if a > b. */
export function compareSemver(a: SemVer, b: SemVer): number {
  for (let i = 0; i < 3; i++) {
    if (a[i] < b[i]) return -1;
    if (a[i] > b[i]) return 1;
  }
  return 0;
}

/** Cached detected versions keyed by binary path. */
const versionCache = new Map<string, string>();

/**
 * Detect the CLI version by running `{bin} --version`.
 * Result is cached per binary path.
 */
export async function detectVersion(bin: string): Promise<string> {
  const cached = versionCache.get(bin);
  if (cached) return cached;

  const version = await new Promise<string>((resolve, reject) => {
    const child = spawn(bin, ["--version"], {
      stdio: ["ignore", "pipe", "pipe"],
    });

    const stdoutChunks: Buffer[] = [];

    child.stdout.on("data", (chunk: Buffer) => stdoutChunks.push(chunk));

    child.on("error", (err) => {
      reject(
        new ZigError(
          `Could not detect zig CLI version: failed to run '${bin} --version'. ` +
            `Ensure zig is installed and on your PATH, or set ZIG_BIN. ` +
            `(${err.message})`,
          null,
          "",
        ),
      );
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(
          new ZigError(
            `Could not detect zig CLI version: '${bin} --version' exited with code ${code}`,
            code,
            "",
          ),
        );
        return;
      }

      const output = Buffer.concat(stdoutChunks).toString().trim();
      // Expected format: "zig-cli 0.4.1" or just "0.4.1"
      const parts = output.split(/\s+/);
      const versionStr = parts[parts.length - 1];

      try {
        parseSemver(versionStr);
      } catch {
        reject(
          new ZigError(
            `Could not parse zig CLI version from output: "${output}". ` +
              `Expected format: "zig-cli X.Y.Z"`,
            null,
            "",
          ),
        );
        return;
      }

      resolve(versionStr);
    });
  });

  versionCache.set(bin, version);
  return version;
}

/** Feature requirement passed to checkVersion. */
export interface VersionRequirement {
  method: string;
  version: string;
  isSet: boolean;
}

/**
 * Check that the installed CLI version satisfies all configured feature requirements.
 * Throws ZigVersionError if any requirement is not met.
 */
export async function checkVersion(
  bin: string,
  requirements: VersionRequirement[],
): Promise<void> {
  const active = requirements.filter((r) => r.isSet);
  if (active.length === 0) return;

  const detected = await detectVersion(bin);
  const detectedSemver = parseSemver(detected);

  const failures = active.filter(
    (r) => compareSemver(detectedSemver, parseSemver(r.version)) < 0,
  );

  if (failures.length === 0) return;

  if (failures.length === 1) {
    throw new ZigVersionError(
      `${failures[0].method} requires zig CLI >= ${failures[0].version}, ` +
        `but the installed version is ${detected}. Please update the zig binary.`,
      failures[0].version,
      detected,
    );
  }

  const lines = failures.map(
    (f) => `  - ${f.method} requires >= ${f.version}`,
  );
  throw new ZigVersionError(
    `The following methods require a newer zig CLI version:\n` +
      `${lines.join("\n")}\n` +
      `Installed version: ${detected}. Please update the zig binary.`,
    failures[0].version,
    detected,
  );
}

/** @internal Inject a version into the cache for testing. */
export function _setVersionForTesting(bin: string, version: string): void {
  versionCache.set(bin, version);
}

/** @internal Clear the version cache for testing. */
export function _clearVersionCache(): void {
  versionCache.clear();
}
