#!/usr/bin/env node
// Post-build SEO generator.
//
// Runs after `vite build` and:
//   1. Splices a route-specific <head> block into a copy of dist/index.html
//      for every public route declared in siteConfig.mjs (§11.3 "post-build
//      per-route head splicer"). With a single route the SPA shell is just
//      rewritten in-place; multi-route sites get one HTML file per route.
//   2. Emits dist/sitemap.xml with <lastmod> derived from the latest git
//      commit that touched any source file under website/src/ — never
//      build-time now() (§11.3).
//   3. Copies the homepage shell to dist/404.html so SPA-fallback hosting
//      keeps working on unknown URLs.
//
// All copy comes from website/src/seo/siteConfig.mjs — never inline strings.
// Adding a route is a one-file change there.

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "fs";
import { execSync } from "child_process";
import { dirname, join, resolve } from "path";
import { fileURLToPath } from "url";

import {
  SITE_URL,
  SITE_NAME,
  SITE_LANG,
  SITE_LOCALE,
  OG_IMAGE,
  OG_IMAGE_WIDTH,
  OG_IMAGE_HEIGHT,
  OG_IMAGE_ALT,
  ROUTES,
} from "../src/seo/siteConfig.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const WEBSITE_DIR = resolve(__dirname, "..");
const DIST_DIR = join(WEBSITE_DIR, "dist");
const REPO_ROOT = resolve(WEBSITE_DIR, "..");

if (!existsSync(DIST_DIR)) {
  console.error(
    `generate-seo: ${DIST_DIR} not found — run \`vite build\` before this script.`,
  );
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Source-derived <lastmod>: latest commit touching website/src/ (or a
// per-route source path once the route map starts pointing at specific docs).
// ---------------------------------------------------------------------------

function gitLastModified(relativePath) {
  try {
    const iso = execSync(
      `git -C "${REPO_ROOT}" log -1 --format=%cI -- "${relativePath}"`,
      { encoding: "utf-8" },
    ).trim();
    if (iso) return iso.slice(0, 10); // YYYY-MM-DD
  } catch {
    // fall through
  }
  return null;
}

const SITE_LASTMOD = gitLastModified("website/src") ?? gitLastModified(".");

// ---------------------------------------------------------------------------
// Head splicing
// ---------------------------------------------------------------------------

function escapeHtml(s) {
  return String(s)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function buildHead(route) {
  const url = `${SITE_URL}${route.path}`;
  const title = route.title;
  const description = route.description;
  const jsonLd = JSON.stringify(route.jsonLd, null, 2);
  return `    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🔀</text></svg>" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${escapeHtml(title)}</title>
    <meta name="description" content="${escapeHtml(description)}" />
    <link rel="canonical" href="${escapeHtml(url)}" />
    <meta name="robots" content="index,follow,max-image-preview:large" />

    <!-- Open Graph -->
    <meta property="og:site_name" content="${escapeHtml(SITE_NAME)}" />
    <meta property="og:locale" content="${escapeHtml(SITE_LOCALE)}" />
    <meta property="og:type" content="website" />
    <meta property="og:title" content="${escapeHtml(title)}" />
    <meta property="og:description" content="${escapeHtml(description)}" />
    <meta property="og:url" content="${escapeHtml(url)}" />
    <meta property="og:image" content="${escapeHtml(OG_IMAGE)}" />
    <meta property="og:image:width" content="${OG_IMAGE_WIDTH}" />
    <meta property="og:image:height" content="${OG_IMAGE_HEIGHT}" />
    <meta property="og:image:alt" content="${escapeHtml(OG_IMAGE_ALT)}" />

    <!-- Twitter Card -->
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:title" content="${escapeHtml(title)}" />
    <meta name="twitter:description" content="${escapeHtml(description)}" />
    <meta name="twitter:image" content="${escapeHtml(OG_IMAGE)}" />

    <!-- Sitemap (also referenced from /robots.txt) -->
    <link rel="sitemap" type="application/xml" href="/sitemap.xml" />

    <!-- JSON-LD structured data -->
    <script type="application/ld+json">
${jsonLd}
    </script>`;
}

function spliceHead(html, headInner, lang) {
  const langReplaced = html.replace(
    /<html\b[^>]*>/i,
    `<html lang="${lang}">`,
  );
  // Vite emits its own <link rel="stylesheet"> / <script> tags into <head>.
  // Preserve them — only replace the prelude up to the first such asset tag.
  const headOpenIdx = langReplaced.indexOf("<head>");
  const headCloseIdx = langReplaced.indexOf("</head>");
  if (headOpenIdx === -1 || headCloseIdx === -1) {
    throw new Error(
      "generate-seo: dist/index.html is missing <head>...</head>; check your vite config",
    );
  }
  const insideHead = langReplaced.slice(headOpenIdx + 6, headCloseIdx);
  // Pull every Vite-injected <script> / <link rel> tag verbatim — those carry
  // hashed asset URLs that we must not regenerate. Match only tags pointing
  // at the /assets/ output dir so we don't accidentally re-inject our own
  // <link rel="sitemap"> / icon / canonical tags.
  const assetTagRe =
    /<(script|link)\b[^>]*\b(?:src|href)\s*=\s*"[^"]*\/assets\/[^"]+"[^>]*(?:\/?>|>[\s\S]*?<\/\1>)/gi;
  const assetTags = insideHead.match(assetTagRe) ?? [];
  const newHead = ["\n", headInner, "", ...assetTags.map((t) => `    ${t}`), "  "].join(
    "\n",
  );
  return (
    langReplaced.slice(0, headOpenIdx + 6) +
    newHead +
    langReplaced.slice(headCloseIdx)
  );
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const shellPath = join(DIST_DIR, "index.html");
const shell = readFileSync(shellPath, "utf-8");

let written = 0;
for (const route of ROUTES) {
  const html = spliceHead(shell, buildHead(route), SITE_LANG);
  const outPath =
    route.path === "/"
      ? join(DIST_DIR, "index.html")
      : join(DIST_DIR, route.path.replace(/^\/+|\/+$/g, ""), "index.html");
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, html, "utf-8");
  written += 1;
}

// SPA-fallback page for hosts that serve unknown URLs by rewriting to /404.
writeFileSync(
  join(DIST_DIR, "404.html"),
  readFileSync(join(DIST_DIR, "index.html"), "utf-8"),
  "utf-8",
);

// sitemap.xml — every route, with git-derived <lastmod>.
const sitemap = [
  `<?xml version="1.0" encoding="UTF-8"?>`,
  `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">`,
  ...ROUTES.map((r) => {
    const url = `${SITE_URL}${r.path}`;
    const lastmod = SITE_LASTMOD ? `\n    <lastmod>${SITE_LASTMOD}</lastmod>` : "";
    return `  <url>\n    <loc>${url}</loc>${lastmod}\n  </url>`;
  }),
  `</urlset>`,
  "",
].join("\n");
writeFileSync(join(DIST_DIR, "sitemap.xml"), sitemap, "utf-8");

console.log(
  `generate-seo: wrote ${written} route HTML file(s), 404.html, sitemap.xml (lastmod=${SITE_LASTMOD ?? "n/a"})`,
);
