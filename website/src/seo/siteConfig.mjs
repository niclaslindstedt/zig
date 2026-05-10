// Single source of truth for SEO copy and per-route head metadata.
//
// Imported by:
//   - runtime React code (Vite resolves .mjs as an ES module)
//   - the build-time generator at website/scripts/generate-seo.mjs, which
//     splices per-route <head> into dist/*.html and emits dist/sitemap.xml
//
// Tweaking the site's pitch must be a one-file change. If you find yourself
// editing website/index.html or scripts/generate-seo.mjs to change copy,
// you're editing the wrong file — change it here and let the generator and
// the runtime pick it up.

export const SITE_URL = "https://zig.niclaslindstedt.se";
export const SITE_NAME = "zig";
// "zig" collides with the Zig programming language in search results, so
// the tagline must immediately disambiguate this project for programmers.
export const SITE_TAGLINE = "AI agent orchestration CLI for developers";
export const SITE_DESCRIPTION =
  "Open-source CLI that orchestrates AI coding agents (Claude, Codex, Gemini, Copilot, Ollama). Describe multi-agent workflows in natural language, capture them as portable .zwf files, and replay them anywhere — powered by zag.";
export const SITE_LANG = "en";
export const SITE_LOCALE = "en_US";

// Keyword set is rendered into <meta name="keywords"> and the
// SoftwareApplication.keywords schema field. Order is deliberate: lead with
// the highest-intent programmer queries, then the project-specific terms.
export const SITE_KEYWORDS = [
  "AI agent orchestration",
  "AI agent CLI",
  "AI coding agents",
  "AI coding agent orchestration",
  "multi-agent workflow",
  "agent workflow automation",
  "agent DAG runner",
  "Claude orchestration",
  "Codex orchestration",
  "Gemini orchestration",
  "Copilot orchestration",
  "Ollama orchestration",
  "MCP workflow",
  "LLM workflow CLI",
  ".zwf workflow file",
  "workflow as code",
  "Rust CLI",
  "developer tools",
  "zag",
  "zig CLI",
];

export const OG_IMAGE = `${SITE_URL}/og-default.png`;
export const OG_IMAGE_WIDTH = 1200;
export const OG_IMAGE_HEIGHT = 630;
export const OG_IMAGE_ALT = `${SITE_NAME} — ${SITE_TAGLINE}`;

export const REPO_URL = "https://github.com/niclaslindstedt/zig";
export const LICENSE_URL = `${REPO_URL}/blob/main/LICENSE`;
export const RELEASES_URL = `${REPO_URL}/releases`;
export const CRATES_URL = "https://crates.io/crates/zig-cli";

// Author / publisher entity (re-used across schema graphs).
export const AUTHOR_NAME = "Niclas Lindstedt";
export const AUTHOR_URL = "https://niclaslindstedt.se";
export const AUTHOR_GITHUB = "https://github.com/niclaslindstedt";

// Canonical schema.org @id values. Stable URLs let crawlers compose the
// graph cleanly across deploys and across sibling sites.
const PERSON_ID = `${AUTHOR_URL}/#person`;
const WEBSITE_ID = `${SITE_URL}/#website`;
const SOFTWARE_ID = `${SITE_URL}/#software`;

// SoftwareApplication entity. softwareVersion is intentionally hand-bumped
// when crates publish — the website is also rebuilt and redeployed at the
// same time, so duplicating it here is the simplest sync point.
function softwareApplication(extra = {}) {
  return {
    "@type": "SoftwareApplication",
    "@id": SOFTWARE_ID,
    name: SITE_NAME,
    alternateName: "zig-cli",
    description: SITE_DESCRIPTION,
    url: `${SITE_URL}/`,
    applicationCategory: "DeveloperApplication",
    applicationSubCategory: "AI Agent Orchestration",
    operatingSystem: "Linux, macOS, Windows",
    license: LICENSE_URL,
    codeRepository: REPO_URL,
    programmingLanguage: "Rust",
    softwareVersion: "0.11.0",
    downloadUrl: RELEASES_URL,
    installUrl: CRATES_URL,
    keywords: SITE_KEYWORDS.join(", "),
    image: OG_IMAGE,
    author: { "@id": PERSON_ID },
    publisher: { "@id": PERSON_ID },
    maintainer: { "@id": PERSON_ID },
    isAccessibleForFree: true,
    offers: {
      "@type": "Offer",
      price: "0",
      priceCurrency: "USD",
    },
    ...extra,
  };
}

function person() {
  return {
    "@type": "Person",
    "@id": PERSON_ID,
    name: AUTHOR_NAME,
    url: AUTHOR_URL,
    sameAs: [AUTHOR_GITHUB],
  };
}

function website() {
  return {
    "@type": "WebSite",
    "@id": WEBSITE_ID,
    url: `${SITE_URL}/`,
    name: SITE_NAME,
    description: SITE_DESCRIPTION,
    inLanguage: SITE_LANG,
    publisher: { "@id": PERSON_ID },
  };
}

// The 3-step "Describe / Share / Run" pitch on the homepage, restated as
// a HowTo so search engines can surface it as a step-by-step rich result.
function howTo() {
  return {
    "@type": "HowTo",
    "@id": `${SITE_URL}/#howto-automate-ai-agents`,
    name: "How to orchestrate AI coding agents with zig",
    description:
      "Describe a multi-agent workflow in natural language, capture it as a portable .zwf file, and run it anywhere with zig.",
    totalTime: "PT5M",
    tool: [{ "@type": "HowToTool", name: "zig CLI" }],
    step: [
      {
        "@type": "HowToStep",
        position: 1,
        name: "Describe the workflow",
        text: "Run `zig workflow create <name>` to launch an interactive session. An AI agent helps you design the orchestration and emits a portable .zwf file.",
        url: `${SITE_URL}/#how-it-works`,
      },
      {
        "@type": "HowToStep",
        position: 2,
        name: "Share the .zwf file",
        text: "The .zwf file is a self-contained TOML workflow definition. Commit it to your repo, send it to a colleague, or publish it for your team.",
        url: `${SITE_URL}/#how-it-works`,
      },
      {
        "@type": "HowToStep",
        position: 3,
        name: "Run it anywhere",
        text: "Execute the workflow with `zig run <name>`. Zig parses the .zwf file, resolves the dependency graph, and delegates each step to zag's orchestration engine.",
        url: `${SITE_URL}/#how-it-works`,
      },
    ],
  };
}

// FAQ entries pair high-intent programmer search queries with concise
// answers. Each Q is phrased the way a developer would actually search,
// so we are eligible for "People also ask" / FAQ rich results.
function faqPage() {
  return {
    "@type": "FAQPage",
    "@id": `${SITE_URL}/#faq`,
    mainEntity: [
      {
        "@type": "Question",
        name: "What is zig?",
        acceptedAnswer: {
          "@type": "Answer",
          text: "zig is an open-source CLI for orchestrating AI coding agents. You describe a workflow in natural language, capture it as a portable .zwf file, and replay it anywhere by running `zig run <workflow>`. Note: this project is unrelated to the Zig programming language.",
        },
      },
      {
        "@type": "Question",
        name: "What is a .zwf file?",
        acceptedAnswer: {
          "@type": "Answer",
          text: "A .zwf file is a portable, TOML-based definition of an AI agent workflow. It declares the steps, their dependencies, and which AI provider runs each step. Commit it alongside your code so anyone with the zig CLI can reproduce the same automation.",
        },
      },
      {
        "@type": "Question",
        name: "Which AI providers does zig support?",
        acceptedAnswer: {
          "@type": "Answer",
          text: "Each step can target any provider supported by zag — Anthropic Claude, OpenAI Codex, Google Gemini, GitHub Copilot, or local models via Ollama. You can mix providers across steps in a single workflow.",
        },
      },
      {
        "@type": "Question",
        name: "How does zig differ from zag?",
        acceptedAnswer: {
          "@type": "Answer",
          text: "zag is the lower-level orchestration engine — it spawns and coordinates agent processes via primitives like spawn, wait, collect, and pipe. zig is the higher-level CLI: it compiles human-readable .zwf files into zag commands so you don't have to wire up the orchestration yourself.",
        },
      },
      {
        "@type": "Question",
        name: "How do I orchestrate multiple AI coding agents in parallel?",
        acceptedAnswer: {
          "@type": "Answer",
          text: "Run `zig workflow create <name>` to design a multi-step workflow interactively, then `zig run <name>` to execute it. Independent steps automatically run in parallel tiers based on the dependency graph declared in the .zwf file.",
        },
      },
      {
        "@type": "Question",
        name: "Is zig open source?",
        acceptedAnswer: {
          "@type": "Answer",
          text: "Yes. zig is MIT-licensed. The source lives at github.com/niclaslindstedt/zig and is published to crates.io as `zig-cli`. Install it with `cargo install zig-cli` or grab a prebuilt binary from the GitHub releases page.",
        },
      },
      {
        "@type": "Question",
        name: "What orchestration patterns does zig support?",
        acceptedAnswer: {
          "@type": "Answer",
          text: "zig ships built-in templates for common multi-agent patterns including sequential, fan-out, generator-critic, and more. Pick one with `zig workflow create --pattern <name>` to scaffold a .zwf workflow you can customize.",
        },
      },
      {
        "@type": "Question",
        name: "Does zig run AI agents in isolation?",
        acceptedAnswer: {
          "@type": "Answer",
          text: "Yes. Steps can run in isolated git worktrees or Docker sandboxes, with auto-approve or human-gate policies. The execution environment is configurable per-step in the .zwf file.",
        },
      },
    ],
  };
}

// Per-route metadata. Add a new entry here when you add a new public route;
// the generator will emit a sitemap entry, splice a route-specific <head>,
// and the runtime can read it via routeConfig(path).
//
// Each route exposes:
//   - title / description: rendered into <title> and meta description
//   - keywords: optional override of SITE_KEYWORDS for this route
//   - jsonLd: legacy single entity — kept for the runtime preview
//   - jsonLdGraph: array of schema.org entities, emitted as one
//     {"@graph": [...]} block by the build-time generator. Use this to
//     stack rich-result-eligible types (FAQ, HowTo, Breadcrumbs, ...).
//   - sitemap: optional { changefreq, priority } overrides
export const ROUTES = [
  {
    path: "/",
    title: `${SITE_NAME} — ${SITE_TAGLINE}`,
    description: SITE_DESCRIPTION,
    keywords: SITE_KEYWORDS,
    sitemap: { changefreq: "weekly", priority: "1.0" },
    jsonLd: softwareApplication(),
    jsonLdGraph: [
      softwareApplication(),
      website(),
      person(),
      howTo(),
      faqPage(),
    ],
  },
];

export function routeConfig(path) {
  return ROUTES.find((r) => r.path === path) ?? ROUTES[0];
}
