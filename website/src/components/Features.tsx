import { commands, patterns, stepFields } from "../data/sourceData";

const features = [
  {
    title: "Natural Language Workflows",
    description:
      "Describe what you want done in plain English. An AI agent designs the orchestration and produces a portable .zug file for you.",
    icon: "\u{1F4AC}",
  },
  {
    title: `${commands.length + 4} Commands`,
    description:
      `Everything you need: ${commands.map((c) => c.name).join(", ")}, plus workflow subcommands for list, show, create, and delete.`,
    icon: "\u{2328}\u{FE0F}",
  },
  {
    title: `${patterns.length} Orchestration Patterns`,
    description:
      `Built-in templates for ${patterns.slice(0, 3).map((p) => p.displayName).join(", ")}, and more. Start from a proven pattern, customize from there.`,
    icon: "\u{1F9E9}",
  },
  {
    title: "DAG Execution Engine",
    description:
      "Steps form a dependency graph. Independent steps run in parallel tiers automatically. Conditions, loops, and failure policies give you full control.",
    icon: "\u{1F310}",
  },
  {
    title: `${stepFields.length} Step Fields`,
    description:
      "Fine-grained control over each step: provider, model, timeouts, retries, isolation, environment, file attachments, race groups, and more.",
    icon: "\u{1F527}",
  },
  {
    title: "Shareable .zug Files",
    description:
      "Workflow definitions are portable TOML files. Commit them to your repo, share with your team, version alongside your code.",
    icon: "\u{1F4E4}",
  },
  {
    title: "Multi-Provider",
    description:
      "Each step can use a different AI provider — Claude, Codex, Gemini, Copilot, or Ollama. Mix and match for the best results.",
    icon: "\u{1F500}",
  },
  {
    title: "Powered by zag",
    description:
      "Built on zag's battle-tested orchestration primitives. Zig translates .zug files into spawn, wait, collect, and pipe commands automatically.",
    icon: "\u{26A1}",
  },
  {
    title: "Isolation & Safety",
    description:
      "Run steps in isolated git worktrees or Docker sandboxes. Auto-approve or require human gates. Full control over the execution environment.",
    icon: "\u{1F512}",
  },
];

export default function Features() {
  return (
    <section id="features" className="border-t border-border py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          Everything you need for AI workflow automation
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          From interactive workflow creation to reproducible execution — zig makes multi-agent automation accessible and shareable.
        </p>

        <div className="mt-14 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {features.map((f) => (
            <div
              key={f.title}
              className="group rounded-xl border border-border bg-surface-alt p-6 transition-all hover:border-accent/40 hover:bg-surface-hover"
            >
              <div className="mb-4 text-2xl">{f.icon}</div>
              <h3 className="mb-2 text-lg font-semibold text-text-primary">{f.title}</h3>
              <p className="text-sm leading-relaxed text-text-secondary">{f.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
