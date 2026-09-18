#!/usr/bin/env node
// Regenerate the intelligent-org evaluation fixtures from
// `prototypes/org-preview/src/lib/data.ts` (Development plan E-1; the
// mapping table in Prototype map §3 is the spec).
//
//   node crates/buzz-org-agent/tests/eval/fixtures/generate.mjs          # write
//   node crates/buzz-org-agent/tests/eval/fixtures/generate.mjs --check  # diff
//
// The output is a pure function of `data.ts`, `lib/constants.mjs`, and the
// locale tables: keys derive from names, ids from stable keys, signatures
// use a fixed nonce. `--check` regenerates in memory and fails when any
// checked-in file differs, which is what `just org-fixtures-check` runs.
//
// Node ≥ 23.6 (type stripping imports `data.ts` directly); no dependencies.

import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { COLD, ORGS } from "./lib/constants.mjs";
import { NOW } from "./lib/dates.mjs";
import { AGENT, READER, RELAY, personKey } from "./lib/emit.mjs";
import { localeFor } from "./lib/locales/index.mjs";
import { keyFor } from "./lib/nostr.mjs";
import { buildSequences } from "./lib/sequences.mjs";
import { whoIsNeeded } from "./lib/who-is-needed.mjs";
import { buildCold, buildOrg } from "./lib/world.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const check = process.argv.includes("--check");

// One event per line: diffs stay per event and the files stay greppable.
function eventList(events) {
  return `[\n${events.map((e) => `  ${JSON.stringify(e)}`).join(",\n")}\n]\n`;
}

function pretty(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function countKinds(events) {
  const counts = {};
  for (const e of events) counts[e.kind] = (counts[e.kind] ?? 0) + 1;
  return Object.fromEntries(Object.entries(counts).sort(([a], [b]) => Number(a) - Number(b)));
}

function manifestFor(spec, { org, world }, locales, events) {
  // Only items the seed has a `39101` for: an open project proposal has its
  // item uuid reserved but no item yet.
  const items = {};
  for (const [key, id] of world.byKey ?? []) if (org.items.has(id)) items[key] = id;
  const proposals = {};
  for (const p of org.proposals.values()) proposals[p._meta.key] = { id: p.id, kind: p.kind, status: p.status };
  const rooms = {};
  for (const [key, r] of org.rooms) rooms[key] = { id: r.id, name: r.name, dm: Boolean(r.dm) };
  const direction = {};
  for (const [slug, { artifact, eventId }] of org.direction) {
    direction[slug] = {
      version: artifact.version,
      event: eventId,
      lines: Object.fromEntries((artifact.lines ?? []).map((l) => [l.id, l.text])),
    };
  }
  const people = {};
  for (const name of spec.members) people[name] = personKey(name).pubkey;
  return {
    org: spec.id,
    locales,
    now: NOW,
    relay: keyFor(RELAY).pubkey,
    agent: keyFor(AGENT).pubkey,
    reader: keyFor(READER).pubkey,
    people,
    shapers: org.shaperPubkeys(),
    rooms,
    direction,
    items,
    proposals,
    counts: countKinds(events),
  };
}

// Every string `T` sees during an `en` build; the locale table must cover it.
function collectStrings(spec) {
  const seen = new Set();
  buildOrg(spec, (s) => {
    seen.add(s);
    return s;
  });
  return seen;
}

function translator(spec, locale) {
  const table = localeFor(spec.id, locale);
  const strings = collectStrings(spec);
  const missing = [...strings].filter((s) => !(s in table));
  if (missing.length) {
    throw new Error(`locale ${spec.id}/${locale} is missing ${missing.length} string(s):\n${missing.map((s) => JSON.stringify(s)).join("\n")}`);
  }
  const unused = Object.keys(table).filter((s) => !strings.has(s));
  if (unused.length) {
    throw new Error(`locale ${spec.id}/${locale} has ${unused.length} unused string(s):\n${unused.map((s) => JSON.stringify(s)).join("\n")}`);
  }
  return (s) => table[s];
}

function generate() {
  const files = new Map();
  const put = (rel, text) => files.set(rel, text);

  for (const spec of ORGS) {
    const en = buildOrg(spec);
    const events = en.org.sorted();
    put(`orgs/${spec.id}/seed.json`, eventList(events));
    put(`orgs/${spec.id}/health-gold.json`, eventList(en.health));
    const locales = ["en", spec.locale];
    put(`orgs/${spec.id}/manifest.json`, pretty(manifestFor(spec, en, locales, events)));

    const T = translator(spec, spec.locale);
    const localized = buildOrg(spec, T);
    put(`orgs/${spec.id}/seed.${spec.locale}.json`, eventList(localized.org.sorted()));
    put(`orgs/${spec.id}/health-gold.${spec.locale}.json`, eventList(localized.health));

    put(`who-is-needed/${spec.id}.json`, pretty(whoIsNeeded(spec, en)));
  }

  const cold = buildCold(COLD);
  const coldEvents = cold.org.sorted();
  put("orgs/cold/seed.json", eventList(coldEvents));
  put("orgs/cold/manifest.json", pretty(manifestFor(COLD, cold, ["en"], coldEvents)));

  for (const seq of buildSequences()) {
    put(`sequences/${seq.name}/sequence.json`, pretty(seq.sequence));
    put(`sequences/${seq.name}/before.json`, eventList(seq.before));
    put(`sequences/${seq.name}/gate.json`, eventList(seq.gate));
    put(`sequences/${seq.name}/outcome-a.json`, eventList(seq.outcomeA));
    put(`sequences/${seq.name}/outcome-b.json`, eventList(seq.outcomeB));
  }
  return files;
}

function existingOutputs() {
  const out = [];
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      const full = path.join(dir, name);
      if (statSync(full).isDirectory()) walk(full);
      else if (name.endsWith(".json")) out.push(path.relative(here, full));
    }
  };
  for (const top of ["orgs", "sequences", "who-is-needed"]) {
    try {
      walk(path.join(here, top));
    } catch {
      // Not generated yet.
    }
  }
  return out;
}

const files = generate();
if (check) {
  const problems = [];
  for (const [rel, text] of files) {
    let current = null;
    try {
      current = readFileSync(path.join(here, rel), "utf8");
    } catch {
      problems.push(`missing: ${rel}`);
      continue;
    }
    if (current !== text) problems.push(`differs: ${rel}`);
  }
  for (const rel of existingOutputs()) if (!files.has(rel)) problems.push(`stale: ${rel}`);
  if (problems.length) {
    console.error("intelligent-org fixtures are out of date:\n  " + problems.join("\n  "));
    console.error("\nRun `just org-fixtures` to regenerate.");
    process.exit(1);
  }
  console.log(`org fixtures: ${files.size} files match the generator.`);
} else {
  for (const [rel, text] of files) {
    const full = path.join(here, rel);
    mkdirSync(path.dirname(full), { recursive: true });
    writeFileSync(full, text);
  }
  const stale = existingOutputs().filter((rel) => !files.has(rel));
  if (stale.length) console.warn(`stale outputs not regenerated (delete by hand):\n  ${stale.join("\n  ")}`);
  console.log(`org fixtures: wrote ${files.size} files.`);
}
