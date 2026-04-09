const steps = [
  {
    number: "1",
    title: "Describe",
    subtitle: "Tell zig what to automate",
    description:
      "Use zig workflow create or zig describe to launch an interactive session. An AI agent helps you design the workflow and produces a .zug file.",
    code: `zig workflow create deploy --pattern sequential`,
    color: "text-accent",
  },
  {
    number: "2",
    title: "Share",
    subtitle: "Commit the .zug file",
    description:
      "The .zug file is a self-contained workflow definition in TOML. Commit it to your repo, send it to a colleague, or publish it for your team.",
    code: `git add workflows/deploy.zug\ngit commit -m "add deploy workflow"`,
    color: "text-zag-light",
  },
  {
    number: "3",
    title: "Run",
    subtitle: "Execute anywhere",
    description:
      "Run the workflow with zig run. Zig parses the .zug file, resolves the dependency graph, and delegates each step to zag's orchestration engine.",
    code: `zig run deploy`,
    color: "text-success",
  },
];

export default function HowItWorks() {
  return (
    <section id="how-it-works" className="border-t border-border bg-surface-alt py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          Three steps to automation
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          Describe what you want, share the workflow definition, run it anywhere.
        </p>

        <div className="mt-14 grid gap-8 lg:grid-cols-3">
          {steps.map((s) => (
            <div key={s.number} className="relative rounded-xl border border-border bg-surface p-6">
              <div className={`mb-4 inline-flex h-10 w-10 items-center justify-center rounded-full border border-border text-lg font-bold ${s.color}`}>
                {s.number}
              </div>
              <h3 className={`mb-1 text-xl font-bold ${s.color}`}>{s.title}</h3>
              <p className="mb-4 text-xs font-medium uppercase tracking-wider text-text-dim">{s.subtitle}</p>
              <p className="mb-4 text-sm leading-relaxed text-text-secondary">{s.description}</p>
              <pre className="overflow-x-auto rounded-lg bg-surface-alt p-3 text-xs leading-relaxed text-accent">
                <code>{s.code}</code>
              </pre>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
