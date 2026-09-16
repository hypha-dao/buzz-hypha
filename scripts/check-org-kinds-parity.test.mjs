import assert from "node:assert/strict";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import {
  compareRegistries,
  extractKinds,
  findRetiredTagNames,
  normaliseName,
  runCheck,
} from "./check-org-kinds-parity.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const RUST = `
pub const KIND_PROJECT: u32 = 30621;
pub const KIND_IO_DIRECTION: u32 = 39100;
pub const KIND_IO_WORK_ITEM: u32 = 39101;
pub const KIND_IO_SHAPERS_PROPOSE: u32 = 50001;
pub const KIND_IO_DRAFT: u32 = 50100;
`;
const TS = `
export const KIND_DM_VISIBILITY = 30622;
export const KIND_IO_DIRECTION = 39100;
export const KIND_IO_WORK_ITEM = 39101;
export const KIND_IO_SHAPERS_PROPOSE = 50001;
export const KIND_IO_DRAFT = 50100;
`;
const DART = `
  static const dmVisibility = 30622;
  static const ioDirection = 39100;
  static const ioWorkItem = 39101;
  static const ioShapersPropose = 50001;
  static const ioDraft = 50100;
`;

test("normaliseName maps Rust, TS, and Dart spellings to one key", () => {
  assert.equal(normaliseName("KIND_IO_WORK_ITEM"), "io_work_item");
  assert.equal(normaliseName("ioWorkItem"), "io_work_item");
  assert.equal(normaliseName("ioShaperStepDown"), "io_shaper_step_down");
});

test("extractKinds keeps only org-range constants", () => {
  const rust = extractKinds(RUST, "rust");
  assert.deepEqual(
    [...rust.entries()],
    [
      ["io_direction", 39100],
      ["io_work_item", 39101],
      ["io_shapers_propose", 50001],
      ["io_draft", 50100],
    ],
  );
  assert.deepEqual([...extractKinds(TS, "ts").entries()], [...rust.entries()]);
  assert.deepEqual([...extractKinds(DART, "dart").entries()], [...rust.entries()]);
  assert.throws(() => extractKinds("", "kotlin"), /unknown registry language/);
});

test("compareRegistries passes when the mirrors match the authority", () => {
  const problems = compareRegistries({
    rust: extractKinds(RUST, "rust"),
    ts: extractKinds(TS, "ts"),
    dart: extractKinds(DART, "dart"),
  });
  assert.deepEqual(problems, []);
});

test("compareRegistries reports missing, renumbered, misnamed, extra, and empty ranges", () => {
  const rust = extractKinds(RUST, "rust");
  const ts = extractKinds(TS.replace("export const KIND_IO_DRAFT = 50100;", ""), "ts");
  const dart = extractKinds(
    DART.replace("ioWorkItem = 39101", "ioWorkItem = 39102").replace("ioDraft", "ioDraught"),
    "dart",
  );
  const problems = compareRegistries({ rust, ts, dart });
  assert.ok(problems.some((p) => p === "kinds.ts: missing io_draft (50100)"), problems.join("\n"));
  assert.ok(problems.some((p) => p === "nostr_models.dart: io_work_item is 39102, kind.rs says 39101"));
  assert.ok(problems.some((p) => p === "nostr_models.dart: missing io_draft (50100)"));
  assert.ok(problems.some((p) => p === "nostr_models.dart: io_draught (50100) is not in kind.rs"));

  const noReads = extractKinds(RUST.replace("pub const KIND_IO_DRAFT: u32 = 50100;", ""), "rust");
  const emptyRange = compareRegistries({ rust: noReads, ts: noReads, dart: noReads });
  assert.ok(emptyRange.some((p) => p.includes("no KIND_IO_* constant in the read range")));

  const duplicate = extractKinds(`${RUST}pub const KIND_IO_AGAIN: u32 = 39100;\n`, "rust");
  assert.ok(compareRegistries({ rust: duplicate, ts: duplicate, dart: duplicate }).some((p) => p.includes("share kind 39100")));
});

test("findRetiredTagNames flags every retired name and honours the allowlist", () => {
  const files = [
    {
      path: "docs/intelligent-org/architecture/x.md",
      content: [
        'My Work is `{kinds:[50100], "#needs":[me]}`', // 1
        "filter on #item and #parent", // 2
        'a `#status` of open, a `#skill` tag', // 3
        '["kind", "ticket"] on 50100', // 4
        '["note" , "tally"]', // 5
        "see [state machines](#status-machines) and #items and #skills", // 6: anchors / plurals are fine
        'let k = payload["kind"].as_u64(); // JSON indexing is fine', // 7
        '["t", "ticket"], ["n", "shaper"], {"#n": [me]}', // 8: the D11 letters are fine
      ].join("\n"),
    },
    { path: "docs/intelligent-org/plans/intelligent-org-readiness.md", content: "the old `#needs` tag" },
  ];
  const hits = findRetiredTagNames(files);
  assert.deepEqual(
    hits.map((h) => h.line),
    [1, 2, 3, 4, 5],
    hits.map((h) => `${h.line}: ${h.text}`).join("\n"),
  );
  assert.ok(hits.every((h) => h.path.endsWith("x.md")));
  // Without the allowlist the record document is a hit too.
  assert.equal(findRetiredTagNames(files, new Set()).length, 6);
});

test("the checked-in registries and org-facing sources pass the real check", () => {
  // Binds the guard to the production files: removing a KIND_IO_* constant
  // from kinds.ts or nostr_models.dart, or reintroducing `#needs` in a
  // Protocol document, fails this test.
  assert.deepEqual(runCheck(repoRoot), []);
});
