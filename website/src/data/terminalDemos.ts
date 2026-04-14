import type { TerminalTab } from "./logStyles";

export const terminalDemos: TerminalTab[] = [
  {
    label: "Run Workflow",
    sequence: [
      { type: "comment", text: "# Run a workflow from a .zwf file" },
      {
        type: "command",
        text: "zig run code-review",
      },
      { type: "pause", duration: 500 },
      {
        type: "output",
        delay: 200,
        lines: [
          { text: "\u25b6 Running workflow: code-review (3 steps)", style: "zag" },
          { text: "\u2713 Parsed code-review.zwf", style: "success" },
          "",
        ],
      },
      { type: "pause", duration: 400 },
      {
        type: "output",
        delay: 100,
        lines: [
          { text: "[1/3] analyze \u2192 zag spawn -p claude", style: "assistant" },
          { text: "      \u2713 completed in 6.2s", style: "success" },
          { text: "[2/3] test \u2192 zag spawn -p claude", style: "assistant" },
          { text: "      \u2713 completed in 4.8s", style: "success" },
          { text: "[3/3] report \u2192 zag spawn -p claude --depends-on analyze,test", style: "assistant" },
          { text: "      \u2713 completed in 3.1s", style: "success" },
        ],
      },
      { type: "pause", duration: 300 },
      {
        type: "output",
        lines: [
          "",
          { text: "\u2713 Workflow completed (3/3 steps passed)", style: "success" },
          { text: "  Total: 14.1s \u00b7 Tokens: 4,231 in / 2,107 out", style: "diffStat" },
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
  {
    label: "Create Workflow",
    sequence: [
      { type: "comment", text: "# Create a workflow with an AI agent" },
      {
        type: "command",
        text: "zig workflow create deploy --pattern sequential",
      },
      { type: "pause", duration: 500 },
      {
        type: "output",
        delay: 200,
        lines: [
          { text: "\u25b6 Starting interactive session with Claude...", style: "zag" },
          { text: "> Describe what your \"deploy\" workflow should do:", style: "assistant" },
        ],
      },
      { type: "pause", duration: 800 },
      {
        type: "output",
        delay: 100,
        lines: [
          "",
          { text: "  Agent is designing your workflow...", style: "assistant" },
          { text: "  \u2192 Step 1: lint \u2014 Run linter on changed files", style: "diffStat" },
          { text: "  \u2192 Step 2: test \u2014 Run test suite", style: "diffStat" },
          { text: "  \u2192 Step 3: build \u2014 Build release artifact", style: "diffStat" },
          { text: "  \u2192 Step 4: deploy \u2014 Deploy to staging", style: "diffStat" },
        ],
      },
      { type: "pause", duration: 500 },
      {
        type: "output",
        lines: [
          "",
          { text: "\u2713 Wrote deploy.zwf (4 steps, sequential pattern)", style: "success" },
          { text: "  Run it with: zig run deploy", style: "dim" },
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
  {
    label: "Validate",
    sequence: [
      { type: "comment", text: "# Validate a workflow before sharing" },
      {
        type: "command",
        text: "zig validate workflows/ci-pipeline.zwf",
      },
      { type: "pause", duration: 300 },
      {
        type: "output",
        delay: 150,
        lines: [
          { text: "\u2713 workflow 'ci-pipeline' is valid (5 steps)", style: "success" },
        ],
      },
      { type: "pause", duration: 1000 },
      { type: "comment", text: "# List all available workflows" },
      {
        type: "command",
        text: "zig workflow list",
      },
      { type: "pause", duration: 200 },
      {
        type: "output",
        delay: 80,
        lines: [
          { text: "  code-review     3 steps   sequential", style: "diffStat" },
          { text: "  ci-pipeline     5 steps   fan-out", style: "diffStat" },
          { text: "  deploy          4 steps   sequential", style: "diffStat" },
          { text: "  refactor        6 steps   generator-critic", style: "diffStat" },
          "",
          "4 workflows found",
        ],
      },
      { type: "pause", duration: 1500 },
      { type: "comment", text: "# Show details of a specific workflow" },
      {
        type: "command",
        text: "zig workflow show code-review",
      },
      { type: "pause", duration: 200 },
      {
        type: "output",
        delay: 80,
        lines: [
          { text: "  name:        code-review", style: "diffStat" },
          { text: "  description: Review code changes and generate report", style: "diffStat" },
          { text: "  steps:       analyze \u2192 test \u2192 report", style: "diffStat" },
          { text: "  tags:        review, ci", style: "diffStat" },
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
  {
    label: "Parallel Steps",
    sequence: [
      { type: "comment", text: "# Run a fan-out workflow with parallel agents" },
      {
        type: "command",
        text: 'zig run security-audit "focus on auth module"',
      },
      { type: "pause", duration: 500 },
      {
        type: "output",
        delay: 200,
        lines: [
          { text: "\u25b6 Running workflow: security-audit (4 steps)", style: "zag" },
          { text: "\u2713 Parsed security-audit.zwf", style: "success" },
          "",
          { text: "[tier 1] Running 3 steps in parallel...", style: "assistant" },
        ],
      },
      { type: "pause", duration: 600 },
      {
        type: "output",
        delay: 150,
        lines: [
          { text: "  \u2713 sast-scan completed (8.4s)", style: "success" },
          { text: "  \u2713 dep-audit completed (3.2s)", style: "success" },
          { text: "  \u2713 secrets-check completed (2.1s)", style: "success" },
          "",
          { text: "[tier 2] synthesize \u2192 zag spawn -p claude --inject-context", style: "assistant" },
        ],
      },
      { type: "pause", duration: 500 },
      {
        type: "output",
        lines: [
          { text: "  \u2713 synthesize completed (5.7s)", style: "success" },
          "",
          { text: "\u2713 Workflow completed (4/4 steps passed)", style: "success" },
          { text: "  Findings: 1 critical, 3 warnings, 0 info", style: "warn" },
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
];
