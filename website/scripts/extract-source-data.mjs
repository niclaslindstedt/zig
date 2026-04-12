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

  // Extract Command enum
  const commandsBlock = src.match(/pub enum Command\s*\{([\s\S]*?)^}/m);
  if (!commandsBlock) throw new Error("Could not find Command enum in cli.rs");

  const commands = [];
  const re = /\/\/\/\s*(.+)\n\s+(\w+)\s*[\{,]/g;
  let m;
  while ((m = re.exec(commandsBlock[1])) !== null) {
    const desc = m[1].trim();
    const name = camelToKebab(m[2]);
    if (desc.includes("Internal:") || desc.includes("#[command(hide")) continue;
    commands.push({ name, description: desc });
  }
  return commands;
}

// Extract WorkflowCommand subcommands
function extractWorkflowSubcommands() {
  const src = read("zig-cli/src/cli.rs");

  const block = src.match(/pub enum WorkflowCommand\s*\{([\s\S]*?)^}/m);
  if (!block) return [];

  const subcommands = [];
  const re = /\/\/\/\s*(.+)\n\s+(\w+)\s*[\{,]/g;
  let m;
  while ((m = re.exec(block[1])) !== null) {
    subcommands.push({
      name: camelToKebab(m[2]),
      description: m[1].trim(),
    });
  }
  return subcommands;
}

function camelToKebab(s) {
  return s.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
}

// ---------------------------------------------------------------------------
// 3. Patterns (from zig-cli/src/cli.rs)
// ---------------------------------------------------------------------------

function extractPatterns() {
  const src = read("zig-cli/src/cli.rs");

  const block = src.match(/pub enum Pattern\s*\{([\s\S]*?)^}/m);
  if (!block) return [];

  const patterns = [];
  const re = /\/\/\/\s*(.+)\n\s+(\w+),?/g;
  let m;
  while ((m = re.exec(block[1])) !== null) {
    const name = m[2];
    // Find the kebab-case name from as_core_name
    const kebabMatch = src.match(new RegExp(`Pattern::${name}\\s*=>\\s*"([^"]+)"`));
    patterns.push({
      name: kebabMatch ? kebabMatch[1] : camelToKebab(name),
      displayName: name.replace(/([a-z])([A-Z])/g, "$1 $2"),
      description: m[1].trim(),
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
  // Match doc comments followed by pub field declarations
  const re = /\/\/\/\s*(.+(?:\n\s*\/\/\/\s*.+)*)\n\s*(?:#\[serde[^\]]*\]\s*\n\s*)*pub (\w+):\s*([^,\n]+)/g;
  let m;
  while ((m = re.exec(block[1])) !== null) {
    const description = m[1].replace(/\n\s*\/\/\/\s*/g, " ").trim();
    const name = m[2];
    let fieldType = m[3].trim();

    // Simplify types for display
    if (fieldType.startsWith("Option<")) fieldType = fieldType.replace(/Option<(.+)>/, "$1") + "?";
    if (fieldType === "HashMap<String, String>") fieldType = "map";
    if (fieldType === "Vec<String>") fieldType = "list";

    fields.push({ name, type: fieldType, description });
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
