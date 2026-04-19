#!/usr/bin/env node
// Extract structured data from Rust source files and generate sourceData.ts.
//
// Usage: node scripts/extract-source-data.mjs
// Run from the website/ directory (or repo root — it auto-detects).
//
// This replaces hardcoded website data with values parsed from the actual
// Rust source, so the website stays in sync with the codebase.

import { readFileSync, writeFileSync, existsSync, readdirSync } from "fs";
import { resolve, dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

// Resolve repo root (works from website/ or repo root)
const REPO_ROOT = existsSync(resolve(__dirname, "../../zig-cli"))
  ? resolve(__dirname, "../..")
  : resolve(__dirname, "..");

function read(relPath) {
  return readFileSync(join(REPO_ROOT, relPath), "utf-8");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function camelToKebab(s) {
  return s.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
}

// Parse a Rust block (enum or struct body) into a list of top-level items.
// Each item has the form { doc: string[], decl: string } where `doc` is the
// collected `///` doc comment lines and `decl` is the first non-doc line of
// the declaration (trimmed). Only items at brace depth 0 within the block are
// returned, so fields inside a variant's inline struct are skipped.
function parseItems(blockText) {
  const lines = blockText.split("\n");
  const items = [];
  let doc = [];
  let attrs = [];
  let depth = 0;
  for (const line of lines) {
    const opens = (line.match(/\{/g) || []).length;
    const closes = (line.match(/\}/g) || []).length;
    const preDepth = depth;
    depth += opens - closes;

    // Only process lines at the top level of the block.
    if (preDepth !== 0) continue;

    const docMatch = line.match(/^\s*\/\/\/\s?(.*)$/);
    if (docMatch) {
      doc.push(docMatch[1]);
      continue;
    }

    // Collect attributes without clearing the doc buffer.
    const attrMatch = line.match(/^\s*(#\[.*\])\s*$/);
    if (attrMatch) {
      attrs.push(attrMatch[1]);
      continue;
    }

    // Skip regular `//` comments (but not `///` doc comments, handled above).
    if (/^\s*\/\/($|[^/])/.test(line)) continue;

    const trimmed = line.trim();
    if (trimmed === "" || trimmed === "}") {
      // Blank line or closing brace breaks association with any collected doc.
      doc = [];
      attrs = [];
      continue;
    }

    if (doc.length > 0) {
      items.push({ doc: [...doc], attrs: [...attrs], decl: trimmed });
      doc = [];
      attrs = [];
    }
  }
  return items;
}

// Extract the clap-renamed command name from a `#[command(name = "...")]`
// attribute, if present. Otherwise falls back to kebab-casing the variant
// identifier.
function commandName(item, fallbackIdent) {
  for (const attr of item.attrs || []) {
    const m = attr.match(/^#\[command\([^)]*\bname\s*=\s*"([^"]+)"/);
    if (m) return m[1];
  }
  return camelToKebab(fallbackIdent);
}

// First non-empty doc line (used for one-line descriptions like CLI commands).
function firstDocLine(doc) {
  for (const l of doc) {
    if (l.trim() !== "") return l.trim();
  }
  return "";
}

// Join all non-empty doc lines with a single space (used for full field docs).
function joinDoc(doc) {
  return doc
    .filter((l) => l.trim() !== "")
    .join(" ")
    .trim();
}

// ---------------------------------------------------------------------------
// 1. Version (from zig-cli/Cargo.toml)
// ---------------------------------------------------------------------------

function extractVersion() {
  const cargo = read("zig-cli/Cargo.toml");
  const m = cargo.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("Could not extract version from zig-cli/Cargo.toml");
  return m[1];
}

// ---------------------------------------------------------------------------
// 2. Commands (from zig-cli/src/cli.rs)
// ---------------------------------------------------------------------------

function extractCommands() {
  const src = read("zig-cli/src/cli.rs");

  const block = src.match(/pub enum Command\s*\{([\s\S]*?)^}/m);
  if (!block) throw new Error("Could not find Command enum in cli.rs");

  const commands = [];
  for (const item of parseItems(block[1])) {
    const m = item.decl.match(/^(\w+)/);
    if (!m) continue;
    commands.push({
      name: commandName(item, m[1]),
      description: firstDocLine(item.doc),
    });
  }
  return commands;
}

// Extract WorkflowCommand subcommands
function extractWorkflowSubcommands() {
  const src = read("zig-cli/src/cli.rs");

  const block = src.match(/pub enum WorkflowCommand\s*\{([\s\S]*?)^}/m);
  if (!block) return [];

  const subcommands = [];
  for (const item of parseItems(block[1])) {
    const m = item.decl.match(/^(\w+)/);
    if (!m) continue;
    subcommands.push({
      name: commandName(item, m[1]),
      description: firstDocLine(item.doc),
    });
  }
  return subcommands;
}

// ---------------------------------------------------------------------------
// 3. Patterns (from zig-cli/src/cli.rs)
// ---------------------------------------------------------------------------

function extractPatterns() {
  const src = read("zig-cli/src/cli.rs");

  const block = src.match(/pub enum Pattern\s*\{([\s\S]*?)^}/m);
  if (!block) return [];

  const patterns = [];
  for (const item of parseItems(block[1])) {
    const m = item.decl.match(/^(\w+)/);
    if (!m) continue;
    const name = m[1];
    // Find the kebab-case name from as_core_name
    const kebabMatch = src.match(new RegExp(`Pattern::${name}\\s*=>\\s*"([^"]+)"`));
    patterns.push({
      name: kebabMatch ? kebabMatch[1] : camelToKebab(name),
      displayName: name.replace(/([a-z])([A-Z])/g, "$1 $2"),
      description: firstDocLine(item.doc),
    });
  }
  return patterns;
}

// ---------------------------------------------------------------------------
// 4. Step fields (from zig-core/src/workflow/model.rs)
// ---------------------------------------------------------------------------

function extractStepFields() {
  const src = read("zig-core/src/workflow/model.rs");

  const block = src.match(/pub struct Step\s*\{([\s\S]*?)^}/m);
  if (!block) return [];

  const fields = [];
  for (const item of parseItems(block[1])) {
    // Match `pub <name>: <type>` where the type may contain commas (e.g.,
    // `HashMap<String, String>`) and may or may not end with a trailing comma.
    const m = item.decl.match(/^pub (\w+):\s*(.+?),?$/);
    if (!m) continue;
    const name = m[1];
    let fieldType = m[2].trim();

    // Simplify types for display.
    if (fieldType.startsWith("Option<")) {
      fieldType = fieldType.replace(/^Option<(.+)>$/, "$1") + "?";
    }
    if (fieldType === "HashMap<String, String>") fieldType = "map";
    if (fieldType === "Vec<String>") fieldType = "list";

    fields.push({
      name,
      type: fieldType,
      description: joinDoc(item.doc),
    });
  }
  return fields;
}

// ---------------------------------------------------------------------------
// 5. Variable types (from zig-core/src/workflow/model.rs)
// ---------------------------------------------------------------------------

function extractVarTypes() {
  const src = read("zig-core/src/workflow/model.rs");

  const block = src.match(/pub enum VarType\s*\{([\s\S]*?)^}/m);
  if (!block) return [];

  return [...block[1].matchAll(/^\s+(\w+),?$/gm)]
    .map((m) => m[1].toLowerCase())
    .filter((v) => !v.startsWith("#"));
}

// ---------------------------------------------------------------------------
// 6. Manpage topics (from manpages/ directory)
// ---------------------------------------------------------------------------

function extractManpageTopics() {
  const manDir = join(REPO_ROOT, "manpages");
  if (!existsSync(manDir)) return [];

  return readdirSync(manDir)
    .filter((f) => f.endsWith(".md"))
    .map((f) => f.replace(/\.md$/, ""))
    .sort();
}

// ---------------------------------------------------------------------------
// Generate output
// ---------------------------------------------------------------------------

function generate() {
  const version = extractVersion();
  const commands = extractCommands();
  const workflowSubcommands = extractWorkflowSubcommands();
  const patterns = extractPatterns();
  const stepFields = extractStepFields();
  const varTypes = extractVarTypes();

  let manpageTopics = [];
  try {
    manpageTopics = extractManpageTopics();
  } catch {
    // manpages dir may not exist
  }

  const output = `// AUTO-GENERATED from Rust source — do not edit manually.
// To regenerate: npm run extract (from website/) or make extract-website-data
// Source files:
//   - zig-cli/Cargo.toml (version)
//   - zig-cli/src/cli.rs (commands, patterns)
//   - zig-core/src/workflow/model.rs (step fields, variable types)
//   - manpages/*.md (manpage topics)

// --- Types ---

export interface CommandData {
  name: string;
  description: string;
}

export interface PatternData {
  name: string;
  displayName: string;
  description: string;
}

export interface StepField {
  name: string;
  type: string;
  description: string;
}

// --- Data ---

export const version = ${JSON.stringify(version)};

export const commands: CommandData[] = ${JSON.stringify(commands, null, 2)};

export const workflowSubcommands: CommandData[] = ${JSON.stringify(workflowSubcommands, null, 2)};

export const patterns: PatternData[] = ${JSON.stringify(patterns, null, 2)};

export const stepFields: StepField[] = ${JSON.stringify(stepFields, null, 2)};

export const varTypes: string[] = ${JSON.stringify(varTypes)};

export const manpageTopics: string[] = ${JSON.stringify(manpageTopics)};
`;

  const outPath = join(__dirname, "../src/data/sourceData.ts");
  writeFileSync(outPath, output, "utf-8");
  console.log(`Generated ${outPath}`);
  console.log(`  Version: ${version}`);
  console.log(`  Commands: ${commands.length}`);
  console.log(`  Workflow subcommands: ${workflowSubcommands.length}`);
  console.log(`  Patterns: ${patterns.length}`);
  console.log(`  Step fields: ${stepFields.length}`);
  console.log(`  Variable types: ${varTypes.length}`);
  console.log(`  Manpage topics: ${manpageTopics.length}`);
}

generate();
