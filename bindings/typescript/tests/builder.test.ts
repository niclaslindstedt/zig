import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ZigBuilder } from "../src/builder.js";
import { ZigError, ZigVersionError } from "../src/types.js";
import type { Workflow } from "../src/types.js";
import { parseWorkflow, zagSessionName, zagSessionNames } from "../src/workflow.js";
import {
  parseSemver,
  compareSemver,
  checkVersion,
  _setVersionForTesting,
  _clearVersionCache,
} from "../src/version.js";

describe("ZigBuilder", () => {
  it("should construct with defaults", () => {
    const builder = new ZigBuilder();
    assert.ok(builder);
  });

  it("should support method chaining", () => {
    const builder = new ZigBuilder()
      .bin("/usr/local/bin/zig")
      .debug()
      .quiet()
      .autoCleanup();

    assert.ok(builder);
  });

  it("should support debug toggle", () => {
    const builder = new ZigBuilder().debug();
    // @ts-expect-error -- accessing private for test
    assert.equal(builder._debug, true);
    const off = new ZigBuilder().debug(false);
    // @ts-expect-error -- accessing private for test
    assert.equal(off._debug, false);
  });

  it("should support quiet toggle", () => {
    const builder = new ZigBuilder().quiet();
    // @ts-expect-error -- accessing private for test
    assert.equal(builder._quiet, true);
  });

  it("should support autoCleanup toggle", () => {
    const builder = new ZigBuilder().autoCleanup();
    // @ts-expect-error -- accessing private for test
    assert.equal(builder._autoCleanup, true);
    const disabled = new ZigBuilder().autoCleanup(false);
    // @ts-expect-error -- accessing private for test
    assert.equal(disabled._autoCleanup, false);
  });

  it("should support bin override", () => {
    const builder = new ZigBuilder().bin("/custom/zig");
    // @ts-expect-error -- accessing private for test
    assert.equal(builder._bin, "/custom/zig");
  });

  it("should build global args with debug", () => {
    const builder = new ZigBuilder().debug();
    // @ts-expect-error -- accessing private for test
    const args = builder.buildGlobalArgs();
    assert.ok(args.includes("--debug"));
  });

  it("should build global args with quiet", () => {
    const builder = new ZigBuilder().quiet();
    // @ts-expect-error -- accessing private for test
    const args = builder.buildGlobalArgs();
    assert.ok(args.includes("--quiet"));
  });

  it("should build empty global args by default", () => {
    const builder = new ZigBuilder();
    // @ts-expect-error -- accessing private for test
    const args = builder.buildGlobalArgs();
    assert.equal(args.length, 0);
  });
});

describe("ZigError", () => {
  it("should contain exit code and stderr", () => {
    const err = new ZigError("test error", 1, "stderr output");
    assert.equal(err.message, "test error");
    assert.equal(err.exitCode, 1);
    assert.equal(err.stderr, "stderr output");
    assert.equal(err.name, "ZigError");
    assert.ok(err instanceof Error);
  });

  it("should handle null exit code", () => {
    const err = new ZigError("spawn failed", null, "");
    assert.equal(err.exitCode, null);
    assert.equal(err.stderr, "");
  });
});

describe("ZigVersionError", () => {
  it("should extend ZigError", () => {
    const err = new ZigVersionError(
      "method requires zig CLI >= 1.0.0",
      "1.0.0",
      "0.4.1",
    );
    assert.ok(err instanceof ZigError);
    assert.ok(err instanceof Error);
    assert.equal(err.name, "ZigVersionError");
    assert.equal(err.requiredVersion, "1.0.0");
    assert.equal(err.installedVersion, "0.4.1");
  });
});

describe("Version checking", () => {
  it("should parse valid semver", () => {
    assert.deepStrictEqual(parseSemver("0.4.1"), [0, 4, 1]);
    assert.deepStrictEqual(parseSemver("1.2.3"), [1, 2, 3]);
  });

  it("should reject invalid semver", () => {
    assert.throws(() => parseSemver("invalid"), ZigError);
    assert.throws(() => parseSemver("1.2"), ZigError);
    assert.throws(() => parseSemver("a.b.c"), ZigError);
  });

  it("should compare semver correctly", () => {
    assert.equal(compareSemver([0, 3, 0], [0, 4, 0]), -1);
    assert.equal(compareSemver([0, 4, 1], [0, 4, 1]), 0);
    assert.equal(compareSemver([0, 5, 0], [0, 4, 1]), 1);
    assert.equal(compareSemver([1, 0, 0], [0, 9, 9]), 1);
  });

  it("should pass when no requirements are set", async () => {
    _setVersionForTesting("zig", "0.4.0");
    try {
      await checkVersion("zig", [
        { method: "someMethod()", version: "0.5.0", isSet: false },
      ]);
    } finally {
      _clearVersionCache();
    }
  });

  it("should pass when version is sufficient", async () => {
    _setVersionForTesting("zig", "0.5.0");
    try {
      await checkVersion("zig", [
        { method: "someMethod()", version: "0.5.0", isSet: true },
      ]);
    } finally {
      _clearVersionCache();
    }
  });

  it("should throw ZigVersionError when version is insufficient", async () => {
    _setVersionForTesting("zig", "0.3.0");
    try {
      await assert.rejects(
        () =>
          checkVersion("zig", [
            { method: "someMethod()", version: "0.5.0", isSet: true },
          ]),
        (err: unknown) => {
          if (!(err instanceof ZigVersionError)) return false;
          assert.ok(err.message.includes("someMethod()"));
          assert.ok(err.message.includes("0.5.0"));
          assert.ok(err.message.includes("0.3.0"));
          assert.equal(err.requiredVersion, "0.5.0");
          assert.equal(err.installedVersion, "0.3.0");
          return true;
        },
      );
    } finally {
      _clearVersionCache();
    }
  });

  it("should report multiple failures", async () => {
    _setVersionForTesting("zig", "0.3.0");
    try {
      await assert.rejects(
        () =>
          checkVersion("zig", [
            { method: "methodA()", version: "0.5.0", isSet: true },
            { method: "methodB()", version: "0.5.0", isSet: true },
          ]),
        (err: unknown) => {
          if (!(err instanceof ZigVersionError)) return false;
          assert.ok(err.message.includes("methodA()"));
          assert.ok(err.message.includes("methodB()"));
          return true;
        },
      );
    } finally {
      _clearVersionCache();
    }
  });
});

describe("Workflow parsing", () => {
  it("should parse a minimal workflow", () => {
    const toml = `
[workflow]
name = "hello"
description = "A hello workflow"

[[step]]
name = "greet"
prompt = "Say hello"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.workflow.name, "hello");
    assert.equal(wf.workflow.description, "A hello workflow");
    assert.equal(wf.steps.length, 1);
    assert.equal(wf.steps[0].name, "greet");
    assert.equal(wf.steps[0].prompt, "Say hello");
  });

  it("should parse workflow tags", () => {
    const toml = `
[workflow]
name = "tagged"
tags = ["ci", "deploy"]

[[step]]
name = "s1"
prompt = "Do stuff"
`;
    const wf = parseWorkflow(toml);
    assert.deepStrictEqual(wf.workflow.tags, ["ci", "deploy"]);
  });

  it("should parse variables", () => {
    const toml = `
[workflow]
name = "with-vars"

[vars.target]
type = "string"
default = "production"
description = "Deploy target"
required = true

[[step]]
name = "deploy"
prompt = "Deploy to \${target}"
`;
    const wf = parseWorkflow(toml);
    assert.ok(wf.vars.target);
    assert.equal(wf.vars.target.type, "string");
    assert.equal(wf.vars.target.default, "production");
    assert.equal(wf.vars.target.description, "Deploy target");
    assert.equal(wf.vars.target.required, true);
  });

  it("should parse multiple steps with dependencies", () => {
    const toml = `
[workflow]
name = "multi"

[[step]]
name = "plan"
prompt = "Create a plan"
provider = "claude"
model = "sonnet"

[[step]]
name = "execute"
prompt = "Execute the plan"
depends_on = ["plan"]
inject_context = true
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.steps.length, 2);
    assert.equal(wf.steps[0].name, "plan");
    assert.equal(wf.steps[0].provider, "claude");
    assert.equal(wf.steps[0].model, "sonnet");
    assert.equal(wf.steps[1].name, "execute");
    assert.deepStrictEqual(wf.steps[1].depends_on, ["plan"]);
    assert.equal(wf.steps[1].inject_context, true);
  });

  it("should parse step with failure policy", () => {
    const toml = `
[workflow]
name = "retry-wf"

[[step]]
name = "flaky"
prompt = "Do something flaky"
on_failure = "retry"
max_retries = 3
retry_model = "opus"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.steps[0].on_failure, "retry");
    assert.equal(wf.steps[0].max_retries, 3);
    assert.equal(wf.steps[0].retry_model, "opus");
  });

  it("should parse step with isolation settings", () => {
    const toml = `
[workflow]
name = "isolated"

[[step]]
name = "safe"
prompt = "Work safely"
worktree = true
auto_approve = true
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.steps[0].worktree, true);
    assert.equal(wf.steps[0].auto_approve, true);
  });

  it("should parse step with command type", () => {
    const toml = `
[workflow]
name = "review-wf"

[[step]]
name = "review"
prompt = "Review changes"
command = "review"
uncommitted = true
base = "main"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.steps[0].command, "review");
    assert.equal(wf.steps[0].uncommitted, true);
    assert.equal(wf.steps[0].base, "main");
  });

  it("should handle empty workflow", () => {
    const toml = `
[workflow]
name = "empty"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.workflow.name, "empty");
    assert.equal(wf.steps.length, 0);
    assert.deepStrictEqual(wf.vars, {});
  });

  it("should skip comments and blank lines", () => {
    const toml = `
# This is a comment
[workflow]
name = "commented"
# Another comment

[[step]]
name = "s1"
prompt = "Hello"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.workflow.name, "commented");
    assert.equal(wf.steps.length, 1);
  });

  it("should parse numeric variable constraints", () => {
    const toml = `
[workflow]
name = "constrained"

[vars.score]
type = "number"
min = 0
max = 100
description = "Quality score"

[[step]]
name = "s1"
prompt = "Rate it"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.vars.score.type, "number");
    assert.equal(wf.vars.score.min, 0);
    assert.equal(wf.vars.score.max, 100);
  });

  it("should parse step environment settings", () => {
    const toml = `
[workflow]
name = "env-wf"

[[step]]
name = "build"
prompt = "Build the project"
root = "/tmp/project"
max_turns = 10
timeout = "5m"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.steps[0].root, "/tmp/project");
    assert.equal(wf.steps[0].max_turns, 10);
    assert.equal(wf.steps[0].timeout, "5m");
  });
});

describe("Zag session names", () => {
  it("should compute a single session name", () => {
    assert.equal(zagSessionName("deploy", "lint"), "zig-deploy-lint");
    assert.equal(zagSessionName("ci-pipeline", "test"), "zig-ci-pipeline-test");
  });

  it("should extract all session names from a workflow", () => {
    const toml = `
[workflow]
name = "deploy"

[[step]]
name = "lint"
prompt = "Run linters"

[[step]]
name = "test"
prompt = "Run tests"

[[step]]
name = "deploy"
prompt = "Deploy to prod"
depends_on = ["lint", "test"]
`;
    const wf = parseWorkflow(toml);
    const sessions = zagSessionNames(wf);
    assert.deepStrictEqual(sessions, {
      lint: "zig-deploy-lint",
      test: "zig-deploy-test",
      deploy: "zig-deploy-deploy",
    });
  });

  it("should return empty record for workflow with no steps", () => {
    const toml = `
[workflow]
name = "empty"
`;
    const wf = parseWorkflow(toml);
    const sessions = zagSessionNames(wf);
    assert.deepStrictEqual(sessions, {});
  });
});

describe("parseWorkflow: roles", () => {
  it("should parse roles section", () => {
    const toml = `
[workflow]
name = "with-roles"

[roles.reviewer]
system_prompt = "You are a code reviewer"

[roles.planner]
system_prompt_file = "prompts/planner.md"

[[step]]
name = "review"
prompt = "Review the code"
role = "reviewer"
`;
    const wf = parseWorkflow(toml);
    assert.ok(wf.roles.reviewer);
    assert.equal(wf.roles.reviewer.system_prompt, "You are a code reviewer");
    assert.equal(wf.roles.reviewer.system_prompt_file, undefined);
    assert.ok(wf.roles.planner);
    assert.equal(wf.roles.planner.system_prompt_file, "prompts/planner.md");
    assert.equal(wf.roles.planner.system_prompt, undefined);
    assert.equal(wf.steps[0].role, "reviewer");
  });

  it("should default to empty roles", () => {
    const toml = `
[workflow]
name = "no-roles"

[[step]]
name = "s1"
prompt = "Hello"
`;
    const wf = parseWorkflow(toml);
    assert.deepStrictEqual(wf.roles, {});
  });
});

describe("parseWorkflow: variable default_file and allowed_values", () => {
  it("should parse default_file on a variable", () => {
    const toml = `
[workflow]
name = "file-default"

[vars.template]
type = "string"
default_file = "prompts/template.txt"
description = "Template content"

[[step]]
name = "s1"
prompt = "Use template"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.vars.template.default_file, "prompts/template.txt");
  });

  it("should parse allowed_values on a variable", () => {
    const toml = `
[workflow]
name = "constrained"

[vars.env]
type = "string"
allowed_values = ["dev", "staging", "prod"]
description = "Target environment"

[[step]]
name = "s1"
prompt = "Deploy"
`;
    const wf = parseWorkflow(toml);
    assert.deepStrictEqual(wf.vars.env.allowed_values, ["dev", "staging", "prod"]);
  });
});

describe("parseWorkflow: workflow-level version, provider, model", () => {
  it("should parse version from [workflow]", () => {
    const toml = `
[workflow]
name = "versioned"
version = "1.2.3"

[[step]]
name = "s1"
prompt = "Hello"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.workflow.version, "1.2.3");
  });

  it("should parse provider and model from [workflow]", () => {
    const toml = `
[workflow]
name = "defaults"
provider = "claude"
model = "sonnet"

[[step]]
name = "s1"
prompt = "Hello"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.workflow.provider, "claude");
    assert.equal(wf.workflow.model, "sonnet");
  });

  it("should leave version/provider/model undefined when not set", () => {
    const toml = `
[workflow]
name = "minimal"

[[step]]
name = "s1"
prompt = "Hello"
`;
    const wf = parseWorkflow(toml);
    assert.equal(wf.workflow.version, undefined);
    assert.equal(wf.workflow.provider, undefined);
    assert.equal(wf.workflow.model, undefined);
  });
});
