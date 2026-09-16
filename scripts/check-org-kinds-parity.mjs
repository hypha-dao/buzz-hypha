// Intelligent-org kind parity + retired-tag-name guard (Development plan R-1,
// risk 1; Codebase verification V1).
//
// 1. The org kind ranges (39100–39149 state, 50000–50049 commands,
//    50100–50149 drafts/reads) must hold the same named constants, with the
//    same numbers, in the three registries:
//      crates/buzz-core/src/kind.rs            (authority)
//      desktop/src/shared/constants/kinds.ts    (mirror)
//      mobile/lib/shared/relay/nostr_models.dart (mirror)
//    Names are normalised (`KIND_IO_WORK_ITEM` / `ioWorkItem` → `io_work_item`)
//    so a number mirrored under the wrong name is a failure too.
//
// 2. No org-facing source may use the retired multi-letter filter tags
//    (`#needs`, `#item`, `#parent`, `#status`, `#skill`) or the retired
//    `["kind", …]` / `["note", …]` tag literals — Readiness D5 and D11 made
//    every `io` filter tag a single letter (`n`, `i`, `u`, `s`, `k`, `t`).
//
// Usage: node scripts/check-org-kinds-parity.mjs   (exit 1 on any finding)

import { existsSync, readdirSync, readFileSync, realpathSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ORG_RANGES = [
  { family: "state", min: 39100, max: 39149 },
  { family: "command", min: 50000, max: 50049 },
  { family: "read", min: 50100, max: 50149 },
];

export const REGISTRIES = {
  rust: "crates/buzz-core/src/kind.rs",
  ts: "desktop/src/shared/constants/kinds.ts",
  dart: "mobile/lib/shared/relay/nostr_models.dart",
};

// Where org-facing code and fixtures live or will live. Missing roots are
// skipped: a wave that has not started has nothing to check yet.
export const RETIRED_TAG_SCAN_ROOTS = [
  "docs/intelligent-org",
  "desktop/src/features/org",
  "crates/buzz-cli",
  "crates/buzz-org-agent/tests/eval", // E-1 fixtures and their generator
  "prototypes/org-preview/scripts", // E-1 generator, if it lands beside the prototype
];

// Documents that record the retirement itself (the F/D/V rows naming the old
// tags). They describe the decision; they are not usage. Historical drafts
// under archive/ are excluded for the same reason.
export const RETIRED_TAG_ALLOWLIST = new Set([
  "docs/intelligent-org/plans/intelligent-org-readiness.md",
  "docs/intelligent-org/plans/intelligent-org-development-plan.md",
  "docs/intelligent-org/architecture/intelligent-org-codebase-verification.md",
  "docs/intelligent-org/architecture/intelligent-org-current-state.md",
]);
const RETIRED_TAG_SKIP_DIRS = new Set(["archive", "node_modules", "target", ".git"]);
const RETIRED_TAG_EXTENSIONS = new Set([".md", ".rs", ".ts", ".tsx", ".mjs", ".js", ".json", ".yaml", ".yml", ".toml", ".sh"]);

export const RETIRED_TAG_PATTERNS = [
  // A `#name` filter key not continued by a word char, `-`, or `/` (so a
  // markdown heading anchor like `#status-machines` does not count).
  { label: "retired multi-letter filter tag", regex: /#(needs|item|parent|status|skill)(?![\w\-/])/g },
  // A tag literal `["kind", …]` / `["note", …]`; JSON indexing like
  // `payload["kind"]` has no comma and does not count.
  { label: "retired tag literal", regex: /\["(kind|note)"\s*,/g },
];

const CONST_PATTERNS = {
  rust: /^\s*pub const (KIND_IO_[A-Z0-9_]+): u32 = (\d+);/gm,
  ts: /^\s*export const (KIND_IO_[A-Z0-9_]+) = (\d+);/gm,
  dart: /^\s*static const (io[A-Za-z0-9]+) = (\d+);/gm,
};

export function isOrgKind(value) {
  return ORG_RANGES.some((r) => value >= r.min && value <= r.max);
}

export function normaliseName(name) {
  if (name.startsWith("KIND_")) {
    return name.slice("KIND_".length).toLowerCase();
  }
  return name.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
}

/** Extract `{ normalisedName → number }` for org-range constants in one registry. */
export function extractKinds(source, lang) {
  const pattern = CONST_PATTERNS[lang];
  if (!pattern) {
    throw new Error(`unknown registry language: ${lang}`);
  }
  const kinds = new Map();
  for (const match of source.matchAll(pattern)) {
    const value = Number(match[2]);
    if (isOrgKind(value)) {
      kinds.set(normaliseName(match[1]), value);
    }
  }
  return kinds;
}

/** Compare the two mirrors against the Rust authority. Returns problem strings. */
export function compareRegistries({ rust, ts, dart }) {
  const problems = [];
  for (const range of ORG_RANGES) {
    const inRange = [...rust.values()].filter((v) => v >= range.min && v <= range.max);
    if (inRange.length === 0) {
      problems.push(`kind.rs: no KIND_IO_* constant in the ${range.family} range ${range.min}–${range.max} (extractor drift?)`);
    }
  }
  const byNumber = new Map();
  for (const [name, value] of rust) {
    const other = byNumber.get(value);
    if (other) {
      problems.push(`kind.rs: ${name} and ${other} share kind ${value}`);
    }
    byNumber.set(value, name);
  }
  for (const [label, mirror] of [
    ["kinds.ts", ts],
    ["nostr_models.dart", dart],
  ]) {
    for (const [name, value] of rust) {
      if (!mirror.has(name)) {
        problems.push(`${label}: missing ${name} (${value})`);
      } else if (mirror.get(name) !== value) {
        problems.push(`${label}: ${name} is ${mirror.get(name)}, kind.rs says ${value}`);
      }
    }
    for (const [name, value] of mirror) {
      if (!rust.has(name)) {
        problems.push(`${label}: ${name} (${value}) is not in kind.rs`);
      }
    }
  }
  return problems;
}

/** Scan `{ path, content }` files for retired tag names. Returns `{ path, line, label, text }` hits. */
export function findRetiredTagNames(files, allowlist = RETIRED_TAG_ALLOWLIST) {
  const hits = [];
  for (const { path: filePath, content } of files) {
    if (allowlist.has(filePath)) {
      continue;
    }
    const lines = content.split(/\r?\n/);
    lines.forEach((text, index) => {
      for (const { label, regex } of RETIRED_TAG_PATTERNS) {
        regex.lastIndex = 0;
        if (regex.test(text)) {
          hits.push({ path: filePath, line: index + 1, label, text: text.trim() });
        }
      }
    });
  }
  return hits;
}

function walk(repoRoot, relativeDir, out) {
  const absolute = path.join(repoRoot, relativeDir);
  if (!existsSync(absolute)) {
    return;
  }
  for (const entry of readdirSync(absolute)) {
    const relative = `${relativeDir}/${entry}`;
    const stat = statSync(path.join(repoRoot, relative));
    if (stat.isDirectory()) {
      if (!RETIRED_TAG_SKIP_DIRS.has(entry)) {
        walk(repoRoot, relative, out);
      }
    } else if (RETIRED_TAG_EXTENSIONS.has(path.extname(entry))) {
      out.push({ path: relative, content: readFileSync(path.join(repoRoot, relative), "utf8") });
    }
  }
}

/** Run both checks against a checkout. Returns the list of problems (empty = pass). */
export function runCheck(repoRoot) {
  const registries = {};
  for (const [lang, relative] of Object.entries(REGISTRIES)) {
    registries[lang] = extractKinds(readFileSync(path.join(repoRoot, relative), "utf8"), lang);
  }
  const problems = compareRegistries(registries);
  const files = [];
  for (const root of RETIRED_TAG_SCAN_ROOTS) {
    walk(repoRoot, root, files);
  }
  for (const hit of findRetiredTagNames(files)) {
    problems.push(`${hit.path}:${hit.line}: ${hit.label}: ${hit.text}`);
  }
  return problems;
}

const scriptPath = realpathSync(fileURLToPath(import.meta.url));
if (process.argv[1] && realpathSync(path.resolve(process.argv[1])) === scriptPath) {
  const repoRoot = path.resolve(path.dirname(scriptPath), "..");
  const problems = runCheck(repoRoot);
  if (problems.length > 0) {
    console.error("Intelligent-org kind parity check failed:");
    for (const problem of problems) {
      console.error(`  - ${problem}`);
    }
    console.error(
      "\nKinds must match across kind.rs, kinds.ts, and nostr_models.dart; org-facing code must use the single-letter filter tags (Protocol §6.5, Readiness D11).",
    );
    process.exit(1);
  }
  console.log("Intelligent-org kind parity: OK");
}
