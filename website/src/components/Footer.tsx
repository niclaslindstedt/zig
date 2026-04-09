export default function Footer() {
  return (
    <footer className="border-t border-border py-12">
      <div className="mx-auto max-w-6xl px-6">
        <div className="flex flex-col items-center justify-between gap-6 md:flex-row">
          <div>
            <span className="text-lg font-bold text-text-primary">
              <span className="text-accent">&#x1F500;</span> zig
            </span>
            <p className="mt-1 text-sm text-text-dim">Describe, share, and run AI agent workflows</p>
          </div>

          <div className="flex flex-wrap justify-center gap-x-6 gap-y-2 text-sm text-text-secondary">
            <a
              href="https://github.com/niclaslindstedt/zig"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              GitHub
            </a>
            <a
              href="https://github.com/niclaslindstedt/zag"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              zag
            </a>
            <a
              href="https://crates.io/crates/zig-cli"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              crates.io
            </a>
            <a
              href="https://github.com/niclaslindstedt/zig/blob/main/LICENSE"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              MIT License
            </a>
          </div>
        </div>
      </div>
    </footer>
  );
}
