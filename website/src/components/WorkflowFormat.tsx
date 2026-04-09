import { useState } from "react";
import { varTypes, stepFields } from "../data/sourceData";

const exampleWorkflow = `[workflow]
name = "code-review"
description = "Review code, run tests, generate report"
tags = ["review", "ci"]

[vars.target]
type = "string"
default = "."
description = "Path to analyze"

[[step]]
name = "analyze"
prompt = "Analyze code quality in \${target}"
provider = "claude"
model = "sonnet"
json = true
saves = { score = "$.quality_score" }

[[step]]
name = "test"
prompt = "Run test suite and report coverage"
provider = "codex"
depends_on = []

[[step]]
name = "report"
prompt = "Create a summary from the analysis and test results"
depends_on = ["analyze", "test"]
inject_context = true
condition = "score > 3"`;

// Show most useful fields for the reference table
const highlightFields = [
  "name", "prompt", "provider", "model", "depends_on",
  "inject_context", "condition", "json", "saves",
  "timeout", "on_failure", "max_retries", "worktree",
  "sandbox", "race_group", "interactive",
];

const displayFields = stepFields.filter((f) => highlightFields.includes(f.name));

export default function WorkflowFormat() {
  const [showAll, setShowAll] = useState(false);
  const fields = showAll ? stepFields : displayFields;

  return (
    <section id="zug-format" className="border-t border-border py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          The .zug workflow format
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          A TOML-based format for defining multi-agent workflows. Variables ({varTypes.join(", ")}),
          dependency graphs, conditions, and data flow — all in a single file.
        </p>

        <div className="mt-14 grid gap-8 lg:grid-cols-2">
          {/* Example file */}
          <div className="overflow-hidden rounded-xl border border-border bg-surface-alt shadow-2xl">
            <div className="flex items-center border-b border-border px-4 py-3">
              <div className="flex items-center gap-2 mr-4">
                <div className="h-3 w-3 rounded-full bg-[#ff5f57]" />
                <div className="h-3 w-3 rounded-full bg-[#febc2e]" />
                <div className="h-3 w-3 rounded-full bg-[#28c840]" />
              </div>
              <span className="text-xs text-text-dim font-medium">code-review.zug</span>
            </div>
            <pre className="overflow-x-auto p-5 text-xs leading-relaxed text-text-secondary">
              <code>{exampleWorkflow}</code>
            </pre>
          </div>

          {/* Step field reference */}
          <div>
            <h3 className="mb-4 text-lg font-semibold text-text-primary">Step fields</h3>
            <div className="space-y-2">
              {fields.map((f) => (
                <div key={f.name} className="flex items-start gap-3 rounded-lg border border-border bg-surface-alt px-4 py-2.5">
                  <code className="shrink-0 text-sm font-semibold text-accent">{f.name}</code>
                  <span className="text-xs leading-relaxed text-text-dim">{f.description}</span>
                </div>
              ))}
            </div>
            {stepFields.length > displayFields.length && (
              <button
                onClick={() => setShowAll(!showAll)}
                className="mt-4 text-sm text-accent hover:text-accent-light transition-colors cursor-pointer"
              >
                {showAll ? `Show fewer fields` : `Show all ${stepFields.length} fields`}
              </button>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
