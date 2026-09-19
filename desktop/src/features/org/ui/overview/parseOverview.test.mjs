import assert from "node:assert/strict";
import test from "node:test";

import {
  collectOverviewPubkeys,
  directionHistory,
  directionSlots,
  parseShapersState,
  parseTallyNote,
  projectHolds,
} from "./parseOverview.ts";

const CONFIRMER =
  "953d3363262e86b770419834c53d2446409db6d918a57f8f339d495d54ab001f";
const HOLDER =
  "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899";
const ME = "deadbeef".repeat(8);

function event(kind, content, tags, createdAt = 10) {
  return {
    id: "1".repeat(64),
    pubkey: "f".repeat(64),
    created_at: createdAt,
    kind,
    tags,
    content: JSON.stringify(content),
    sig: "",
  };
}

test("directionSlots fills four slugs and keeps the newest head", () => {
  const slots = directionSlots([
    event(
      39100,
      {
        slug: "mission",
        version: 1,
        body: "old",
        confirmed_by: ME,
        confirmed_at: 1,
      },
      [
        ["d", "mission"],
        ["version", "1"],
        ["p", ME],
      ],
      1,
    ),
    event(
      39100,
      {
        slug: "mission",
        version: 3,
        body: "Feed the market.",
        confirmed_by: CONFIRMER,
        confirmed_at: 9,
      },
      [
        ["d", "mission"],
        ["version", "3"],
        ["p", CONFIRMER],
      ],
      9,
    ),
    event(
      39100,
      {
        slug: "vision",
        version: 1,
        body: "A hall that stays open.",
        confirmed_by: ME,
      },
      [
        ["d", "vision"],
        ["version", "1"],
        ["p", ME],
      ],
    ),
  ]);
  assert.deepEqual(
    slots.map((slot) => slot.slug),
    ["mission", "vision", "objectives", "strategy"],
  );
  assert.equal(slots[0].head?.version, 3);
  assert.equal(slots[0].head?.confirmedBy, CONFIRMER);
  assert.equal(slots[0].head?.body, "Feed the market.");
  assert.equal(slots[1].head?.version, 1);
  assert.equal(slots[2].head, null);
  assert.equal(slots[3].head, null);
});

test("parseShapersState reads members, rules, and the agent host", () => {
  const state = parseShapersState([
    event(
      39103,
      {
        founder: ME,
        shapers: [ME, CONFIRMER],
        offered: [
          {
            p: HOLDER,
            proposal: "11111111-1111-4111-8111-111111111111",
            at: 4,
          },
        ],
        agent:
          "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92",
        agent_hosted: true,
        rules: { direction: "majority", project: "majority", dri: 1 },
        decision_window_secs: 604800,
      },
      [
        ["d", "shapers"],
        ["p", ME],
        ["p", CONFIRMER],
      ],
    ),
  ]);
  assert.equal(state?.founder, ME);
  assert.deepEqual(state?.shapers, [ME, CONFIRMER]);
  assert.equal(state?.offered[0]?.p, HOLDER);
  assert.equal(state?.agentHosted, true);
  assert.equal(state?.rules.direction, "majority");
  assert.equal(state?.rules.dri, 1);
});

test("projectHolds reads root 39101s and skips tickets", () => {
  const holds = projectHolds([
    event(
      39101,
      {
        id: "11111111-1111-4111-8111-111111111111",
        title: "Weekday hall",
        dri: HOLDER,
        state: "accepted",
        parent: null,
        depth: 0,
      },
      [
        ["d", "11111111-1111-4111-8111-111111111111"],
        ["t", "project"],
        ["p", HOLDER],
        ["s", "accepted"],
      ],
    ),
    event(
      39101,
      {
        id: "22222222-2222-4222-8222-222222222222",
        title: "A ticket",
        dri: ME,
        parent: "11111111-1111-4111-8111-111111111111",
        depth: 1,
      },
      [
        ["d", "22222222-2222-4222-8222-222222222222"],
        ["t", "ticket"],
        ["p", ME],
      ],
    ),
    event(
      39101,
      {
        id: "33333333-3333-4333-8333-333333333333",
        title: "Harvest",
        dri: null,
        state: "open",
        parent: null,
        depth: 0,
      },
      [
        ["d", "33333333-3333-4333-8333-333333333333"],
        ["t", "project"],
        ["s", "open"],
      ],
    ),
  ]);
  assert.deepEqual(
    holds.map((hold) => [hold.title, hold.dri]),
    [
      ["Harvest", null],
      ["Weekday hall", HOLDER],
    ],
  );
});

test("parseTallyNote binds t=tally and ignores other notes", () => {
  const tally = parseTallyNote([
    event(
      50103,
      { note: "draft_dropped", reason: "nag" },
      [["t", "draft_dropped"]],
      2,
    ),
    event(
      50103,
      {
        note: "tally",
        week: "2026-W38",
        window_weeks: 4,
        moves: {
          1: {
            opened: 6,
            accepted: 3,
            amended: 1,
            declined: { already_covered: 1 },
            dropped: { nag: 0 },
            shadow: 0,
          },
        },
        health: { reads: 4, rated: 8, agreed: 7 },
        open_older_than_5d: 0,
        receipt_rejected: 1,
      },
      [
        ["t", "tally"],
        ["week", "2026-W38"],
      ],
      9,
    ),
  ]);
  assert.equal(tally?.week, "2026-W38");
  assert.equal(tally?.moves["1"]?.opened, 6);
  assert.equal(tally?.health?.agreed, 7);
  assert.equal(tally?.receiptRejected, 1);
});

test("directionHistory keeps passed proposals for that slug", () => {
  const versions = directionHistory(
    [
      event(
        39102,
        {
          id: "aaaaaaa1-1111-4111-8111-111111111111",
          kind: "direction",
          status: "passed",
          payload: { slug: "mission", base: 2, body: "v3 body" },
          votes: [{ p: CONFIRMER, vote: "agree", at: 9 }],
          decided_at: 9,
        },
        [
          ["d", "aaaaaaa1-1111-4111-8111-111111111111"],
          ["t", "direction"],
          ["s", "passed"],
        ],
      ),
      event(
        39102,
        {
          id: "bbbbbbb2-2222-4222-8222-222222222222",
          kind: "direction",
          status: "passed",
          payload: { slug: "vision", base: 0, body: "other" },
          executed: { kind: "direction", id: "vision" },
        },
        [
          ["t", "direction"],
          ["s", "passed"],
        ],
      ),
    ],
    "mission",
  );
  assert.equal(versions.length, 1);
  assert.equal(versions[0].version, 3);
  assert.equal(versions[0].confirmedBy, CONFIRMER);
  assert.equal(versions[0].body, "v3 body");
});

test("collectOverviewPubkeys unions confirmer, shapers, holder, agent", () => {
  const slots = directionSlots([
    event(
      39100,
      { slug: "mission", version: 1, body: "x", confirmed_by: CONFIRMER },
      [["d", "mission"]],
    ),
  ]);
  const shapers = parseShapersState([
    event(
      39103,
      { shapers: [ME], agent: HOLDER, agent_hosted: false, founder: ME },
      [["d", "shapers"]],
    ),
  ]);
  const holds = projectHolds([
    event(
      39101,
      {
        id: "11111111-1111-4111-8111-111111111111",
        title: "Hall",
        dri: CONFIRMER,
      },
      [["t", "project"]],
    ),
  ]);
  const pubkeys = collectOverviewPubkeys(slots, shapers, holds);
  assert.ok(pubkeys.includes(CONFIRMER));
  assert.ok(pubkeys.includes(ME));
  assert.ok(pubkeys.includes(HOLDER));
});
