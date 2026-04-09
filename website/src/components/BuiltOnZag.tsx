export default function BuiltOnZag() {
  return (
    <section id="built-on-zag" className="border-t border-border py-20 md:py-28">
      <div className="mx-auto max-w-5xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          Built on{" "}
          <span className="text-zag-light">zag</span>
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          Zig is the workflow layer on top of zag's orchestration engine.
          You get the power of zag's primitives without needing to learn them directly.
        </p>

        {/* Architecture diagram */}
        <div className="mx-auto mt-14 max-w-2xl">
          <div className="rounded-xl border border-border bg-surface-alt p-8">
            {/* Zig layer */}
            <div className="rounded-lg border-2 border-accent bg-surface p-6 text-center">
              <div className="text-lg font-bold text-accent">zig</div>
              <div className="mt-2 text-sm text-text-secondary">
                Workflow CLI &mdash; .zug files, patterns, validation, execution
              </div>
              <div className="mt-3 flex flex-wrap justify-center gap-2">
                {["run", "workflow create", "validate", "describe", "man"].map((cmd) => (
                  <code key={cmd} className="rounded-full bg-surface-alt px-2.5 py-0.5 text-xs text-accent">{cmd}</code>
                ))}
              </div>
            </div>

            {/* Arrow */}
            <div className="flex justify-center py-4">
              <div className="flex flex-col items-center gap-1">
                <div className="text-xs text-text-dim">delegates to</div>
                <svg className="h-6 w-6 text-text-dim" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19.5 13.5 12 21m0 0-7.5-7.5M12 21V3" />
                </svg>
              </div>
            </div>

            {/* Zag layer */}
            <div className="rounded-lg border-2 border-zag bg-surface p-6 text-center">
              <div className="text-lg font-bold text-zag-light">zag</div>
              <div className="mt-2 text-sm text-text-secondary">
                Agent engine &mdash; providers, orchestration, sessions, isolation
              </div>
              <div className="mt-3 flex flex-wrap justify-center gap-2">
                {["spawn", "wait", "collect", "pipe", "cancel", "watch"].map((cmd) => (
                  <code key={cmd} className="rounded-full bg-surface-alt px-2.5 py-0.5 text-xs text-zag-light">{cmd}</code>
                ))}
              </div>
            </div>

            {/* Arrow */}
            <div className="flex justify-center py-4">
              <div className="flex flex-col items-center gap-1">
                <div className="text-xs text-text-dim">drives</div>
                <svg className="h-6 w-6 text-text-dim" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19.5 13.5 12 21m0 0-7.5-7.5M12 21V3" />
                </svg>
              </div>
            </div>

            {/* Provider layer */}
            <div className="rounded-lg border border-border bg-surface p-6 text-center">
              <div className="text-sm font-semibold text-text-primary">AI Agents</div>
              <div className="mt-3 flex flex-wrap justify-center gap-2">
                {["Claude", "Codex", "Gemini", "Copilot", "Ollama"].map((name) => (
                  <span key={name} className="rounded-full border border-border bg-surface-alt px-2.5 py-0.5 text-xs text-text-secondary">{name}</span>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Comparison table */}
        <div className="mx-auto mt-12 max-w-2xl">
          <div className="rounded-xl border border-border overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-surface-alt">
                  <th className="border-b border-border px-5 py-3 text-left font-semibold text-text-primary">You want to...</th>
                  <th className="border-b border-border px-5 py-3 text-left font-semibold text-accent">zig</th>
                  <th className="border-b border-border px-5 py-3 text-left font-semibold text-zag-light">zag</th>
                </tr>
              </thead>
              <tbody className="text-text-secondary">
                <tr>
                  <td className="border-b border-border px-5 py-2.5">Define a reusable multi-step workflow</td>
                  <td className="border-b border-border px-5 py-2.5 text-accent">zig workflow create</td>
                  <td className="border-b border-border px-5 py-2.5 text-zag-light">Manual shell scripts</td>
                </tr>
                <tr className="bg-surface-alt">
                  <td className="border-b border-border px-5 py-2.5">Run a workflow from a file</td>
                  <td className="border-b border-border px-5 py-2.5 text-accent">zig run deploy.zug</td>
                  <td className="border-b border-border px-5 py-2.5 text-zag-light">Compose spawn/wait/pipe</td>
                </tr>
                <tr>
                  <td className="border-b border-border px-5 py-2.5">Share automation with your team</td>
                  <td className="border-b border-border px-5 py-2.5 text-accent">Commit .zug file</td>
                  <td className="border-b border-border px-5 py-2.5 text-zag-light">Share the script</td>
                </tr>
                <tr className="bg-surface-alt">
                  <td className="px-5 py-2.5">Execute a one-off agent task</td>
                  <td className="px-5 py-2.5 text-text-dim">Use zag directly</td>
                  <td className="px-5 py-2.5 text-zag-light">zag exec "task"</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </section>
  );
}
