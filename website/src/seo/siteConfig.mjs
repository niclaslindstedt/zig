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
export const SITE_TAGLINE = "Describe, share, and run AI agent workflows";
export const SITE_DESCRIPTION =
  "An orchestration CLI for AI coding agents. Describe workflows in natural language, capture them as shareable .zwf files, and replay them anywhere — powered by zag.";
export const SITE_LANG = "en";
export const SITE_LOCALE = "en_US";

export const OG_IMAGE = `${SITE_URL}/og-default.png`;
export const OG_IMAGE_WIDTH = 1200;
export const OG_IMAGE_HEIGHT = 630;
export const OG_IMAGE_ALT = `${SITE_NAME} — orchestration CLI for AI coding agents`;

export const REPO_URL = "https://github.com/niclaslindstedt/zig";
export const LICENSE_URL = `${REPO_URL}/blob/main/LICENSE`;

// Per-route metadata. Add a new entry here when you add a new public route;
// the generator will emit a sitemap entry, splice a route-specific <head>,
// and the runtime can read it via routeConfig(path).
export const ROUTES = [
  {
    path: "/",
    title: `${SITE_NAME} — ${SITE_TAGLINE}`,
    description: SITE_DESCRIPTION,
    jsonLd: {
      "@context": "https://schema.org",
      "@type": "SoftwareApplication",
      "@id": `${SITE_URL}/#software`,
      name: SITE_NAME,
      description: SITE_DESCRIPTION,
      url: `${SITE_URL}/`,
      applicationCategory: "DeveloperApplication",
      operatingSystem: "Linux, macOS, Windows",
      license: LICENSE_URL,
      codeRepository: REPO_URL,
      offers: {
        "@type": "Offer",
        price: "0",
        priceCurrency: "USD",
      },
    },
  },
];

export function routeConfig(path) {
  return ROUTES.find((r) => r.path === path) ?? ROUTES[0];
}
