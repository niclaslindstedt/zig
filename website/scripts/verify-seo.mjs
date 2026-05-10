#!/usr/bin/env node
// SEO output verifier — OSS_SPEC.md §11.3 ("CI verification").
//
// The website build job must fail if any required SEO output is missing.
// This script runs after generate-seo.mjs and checks that:
//
//   - dist/sitemap.xml exists and lists every route in siteConfig.mjs
//   - dist/robots.txt exists and references the sitemap
//   - dist/og-default.png exists (default Open Graph card)
//   - every route's HTML has <title>, <link rel="canonical">,
//     <meta name="description">, OG/Twitter cards, and at least one
//     <script type="application/ld+json"> block of valid JSON
//
// On any failure we print every issue and exit non-zero so CI surfaces all
// of them at once instead of one-at-a-time.

import { readFileSync, existsSync, statSync } from "fs";
import { dirname, join, resolve } from "path";
import { fileURLToPath } from "url";

import { SITE_URL, ROUTES } from "../src/seo/siteConfig.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const DIST_DIR = resolve(__dirname, "..", "dist");

const failures = [];
function fail(msg) {
  failures.push(msg);
}

function readIfExists(path) {
  return existsSync(path) ? readFileSync(path, "utf-8") : null;
}

// --- sitemap.xml -----------------------------------------------------------
const sitemapPath = join(DIST_DIR, "sitemap.xml");
const sitemap = readIfExists(sitemapPath);
if (!sitemap) {
  fail(`missing dist/sitemap.xml`);
} else {
  for (const route of ROUTES) {
    const url = `${SITE_URL}${route.path}`;
    if (!sitemap.includes(`<loc>${url}</loc>`)) {
      fail(`sitemap.xml is missing <loc>${url}</loc>`);
    }
  }
  if (!/<lastmod>\d{4}-\d{2}-\d{2}<\/lastmod>/.test(sitemap)) {
    fail(`sitemap.xml has no <lastmod> entries`);
  }
}

// --- robots.txt ------------------------------------------------------------
const robotsPath = join(DIST_DIR, "robots.txt");
const robots = readIfExists(robotsPath);
if (!robots) {
  fail(`missing dist/robots.txt`);
} else {
  if (!robots.includes(`Sitemap: ${SITE_URL}/sitemap.xml`)) {
    fail(`robots.txt is missing absolute "Sitemap: ${SITE_URL}/sitemap.xml" line`);
  }
  if (!/User-agent:\s*\*/.test(robots)) {
    fail(`robots.txt has no "User-agent: *" rule`);
  }
}

// --- Open Graph image ------------------------------------------------------
const ogPath = join(DIST_DIR, "og-default.png");
if (!existsSync(ogPath)) {
  fail(`missing dist/og-default.png (referenced by every route's og:image)`);
} else if (statSync(ogPath).size < 1024) {
  fail(`dist/og-default.png is suspiciously small (<1KB)`);
}

// --- per-route HTML --------------------------------------------------------
for (const route of ROUTES) {
  const htmlPath =
    route.path === "/"
      ? join(DIST_DIR, "index.html")
      : join(DIST_DIR, route.path.replace(/^\/+|\/+$/g, ""), "index.html");
  const html = readIfExists(htmlPath);
  if (!html) {
    fail(`missing ${htmlPath} for route ${route.path}`);
    continue;
  }

  const checks = [
    { re: /<title>[^<]+<\/title>/i, msg: "<title>" },
    {
      re: /<link\s+rel=["']canonical["']\s+href=["'][^"']+["']/i,
      msg: '<link rel="canonical">',
    },
    {
      re: /<meta\s+name=["']description["']\s+content=["'][^"']+["']/i,
      msg: '<meta name="description">',
    },
    {
      re: /<meta\s+name=["']robots["']/i,
      msg: '<meta name="robots">',
    },
    {
      re: /<meta\s+property=["']og:title["']/i,
      msg: '<meta property="og:title">',
    },
    {
      re: /<meta\s+property=["']og:image["']/i,
      msg: '<meta property="og:image">',
    },
    {
      re: /<meta\s+name=["']twitter:card["']/i,
      msg: '<meta name="twitter:card">',
    },
    {
      re: /<link\s+rel=["']sitemap["']/i,
      msg: '<link rel="sitemap">',
    },
  ];
  for (const c of checks) {
    if (!c.re.test(html)) fail(`${htmlPath}: missing ${c.msg}`);
  }

  // JSON-LD: must have at least one valid block
  const ldBlocks = [
    ...html.matchAll(
      /<script\s+type=["']application\/ld\+json["']\s*>([\s\S]*?)<\/script>/gi,
    ),
  ];
  if (ldBlocks.length === 0) {
    fail(`${htmlPath}: missing <script type="application/ld+json">`);
  } else {
    for (const [i, m] of ldBlocks.entries()) {
      try {
        JSON.parse(m[1]);
      } catch (e) {
        fail(`${htmlPath}: JSON-LD block #${i + 1} is invalid JSON: ${e.message}`);
      }
    }
  }
}

if (failures.length > 0) {
  console.error("verify-seo: FAILED");
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}

console.log(
  `verify-seo: OK (${ROUTES.length} route(s), sitemap + robots + og-default verified)`,
);
