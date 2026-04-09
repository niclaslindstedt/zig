import { patterns as sourcePatterns } from "../data/sourceData";

const patternExamples: Record<string, string> = {
  sequential: `[[step]]
name = "lint"
prompt = "Run linter on changed files"

[[step]]
name = "test"
prompt = "Run test suite"
depends_on = ["lint"]

[[step]]
name = "deploy"
prompt = "Deploy to staging"
depends_on = ["test"]`,

  "fan-out": `[[step]]
name = "sast"
prompt = "Run static analysis"

[[step]]
name = "deps"
prompt = "Audit dependencies"

[[step]]
name = "secrets"
prompt = "Scan for leaked secrets"

[[step]]
name = "report"
prompt = "Synthesize all findings"
depends_on = ["sast", "deps", "secrets"]
inject_context = true`,

  "generator-critic": `[[step]]
name = "generate"
prompt = "Write the API endpoint"
json = true
saves = { score = "$.quality" }

[[step]]
name = "review"
prompt = "Score this code 1-10"
depends_on = ["generate"]
inject_context = true
json = true
saves = { score = "$.score" }

[[step]]
name = "refine"
prompt = "Improve based on feedback"
depends_on = ["review"]
inject_context = true
condition = "score < 8"
next = "review"`,
};

// Only show the first 3 patterns with code examples
const featured = sourcePatterns.slice(0, 3);
const remaining = sourcePatterns.slice(3);

export default function Patterns() {
  return (
    <section id="patterns" className="border-t border-border bg-surface-alt py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          {sourcePatterns.length} built-in orchestration patterns
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          Start from a proven pattern with <code className="rounded bg-surface px-1.5 py-0.5 text-xs text-accent">zig workflow create --pattern &lt;name&gt;</code>.
          Each pattern generates a .zug template you can customize.
        </p>

        {/* Featured patterns with code */}
        <div className="mt-14 grid gap-6 lg:grid-cols-3">
          {featured.map((p) => (
            <div key={p.name} className="rounded-xl border border-border bg-surface overflow-hidden">
              <div className="border-b border-border p-4">
                <h3 className="font-semibold text-text-primary">{p.displayName}</h3>
                <p className="mt-1 text-xs text-text-dim">{p.description}</p>
              </div>
              <pre className="overflow-x-auto p-4 text-xs leading-relaxed text-text-secondary">
                <code>{patternExamples[p.name] ?? `# zig workflow create --pattern ${p.name}`}</code>
              </pre>
            </div>
          ))}
        </div>

        {/* Remaining patterns as cards */}
        <div className="mx-auto mt-10 max-w-3xl">
          <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
            {remaining.map((p) => (
              <div key={p.name} className="rounded-lg border border-border bg-surface p-4 text-center">
                <code className="text-sm font-semibold text-accent">{p.name}</code>
                <p className="mt-1 text-xs text-text-dim">{p.description}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
