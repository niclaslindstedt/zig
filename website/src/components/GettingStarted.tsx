import { commands, workflowSubcommands } from "../data/sourceData";

const methods = [
  {
    title: "From crates.io",
    command: "cargo install zig-cli",
    note: "Requires Rust 1.85+",
  },
  {
    title: "From source",
    command: "git clone https://github.com/niclaslindstedt/zig\ncd zig && cargo install --path zig-cli",
    note: "Build from latest source",
  },
  {
    title: "GitHub Releases",
    command: "# Download pre-built binary from\n# github.com/niclaslindstedt/zig/releases",
    note: "Pre-built for major platforms",
  },
];

const prereqs = [
  { name: "zag CLI", cmd: "cargo install zag-cli", required: true },
  { name: "Claude", cmd: "curl -fsSL https://claude.ai/install.sh | bash", required: false },
  { name: "Codex", cmd: "npm i -g @openai/codex", required: false },
  { name: "Gemini", cmd: "npm i -g @anthropic-ai/gemini-cli", required: false },
];

export default function GettingStarted() {
  return (
    <section id="get-started" className="border-t border-border bg-surface-alt py-20 md:py-28">
      <div className="mx-auto max-w-5xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          Get started in seconds
        </h2>
        <p className="mx-auto mt-4 max-w-xl text-center text-text-secondary">
          Install zig, install zag, and you're ready to create and run workflows.
        </p>

        {/* Install methods */}
        <div className="mt-12 grid gap-6 md:grid-cols-3">
          {methods.map((m) => (
            <div key={m.title} className="rounded-xl border border-border bg-surface p-5">
              <h3 className="mb-1 text-sm font-semibold text-text-primary">{m.title}</h3>
              <p className="mb-3 text-xs text-text-dim">{m.note}</p>
              <pre className="overflow-x-auto rounded-lg bg-surface-alt p-3 text-xs leading-relaxed text-accent">
                <code>{m.command}</code>
              </pre>
            </div>
          ))}
        </div>

        {/* Prerequisites */}
        <div className="mt-12">
          <h3 className="mb-4 text-center text-lg font-semibold text-text-primary">
            Prerequisites
          </h3>
          <div className="mx-auto max-w-2xl space-y-2">
            {prereqs.map((p) => (
              <div key={p.name} className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between rounded-lg border border-border bg-surface px-4 py-2.5">
                <span className="text-sm font-medium text-text-secondary">
                  {p.name}
                  {p.required && <span className="ml-2 text-xs text-accent">(required)</span>}
                </span>
                <code className="text-xs text-text-dim">{p.cmd}</code>
              </div>
            ))}
          </div>
        </div>

        {/* Command reference */}
        <div className="mt-12">
          <h3 className="mb-4 text-center text-lg font-semibold text-text-primary">
            Command reference
          </h3>
          <div className="mx-auto max-w-2xl space-y-2">
            {commands.map((c) => (
              <div key={c.name} className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between rounded-lg border border-border bg-surface px-4 py-2.5">
                <code className="text-sm font-semibold text-accent">zig {c.name}</code>
                <span className="text-xs text-text-dim">{c.description}</span>
              </div>
            ))}
            {workflowSubcommands.map((c) => (
              <div key={c.name} className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between rounded-lg border border-border bg-surface px-4 py-2.5">
                <code className="text-sm font-semibold text-accent">zig workflow {c.name}</code>
                <span className="text-xs text-text-dim">{c.description}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Quick verify */}
        <div className="mx-auto mt-12 max-w-lg rounded-xl border border-border bg-surface p-5">
          <p className="mb-3 text-center text-sm text-text-secondary">Try it out:</p>
          <pre className="overflow-x-auto text-sm text-text-secondary">
            <code>
              <span className="text-accent">$</span> zig workflow create my-first-workflow
            </code>
          </pre>
        </div>
      </div>
    </section>
  );
}
