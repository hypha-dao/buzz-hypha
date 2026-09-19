import assert from "node:assert/strict";
import test from "node:test";

import { classifyEvent, classifyMyWork, kickerFor } from "./classify.ts";
import { claimOf, receiptsOf } from "./parse.ts";

const ORG_AGENT_PUBKEY =
  "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";

const ME = "deadbeef".repeat(8);
const ALICE =
  "953d3363262e86b770419834c53d2446409db6d918a57f8f339d495d54ab001f";
const RECEIPT = "c1".repeat(32);
const ITEM = "11111111-1111-4111-8111-111111111111";
const PROPOSAL = "66666666-6666-4666-8666-666666666666";

const names = {
  [ME]: "You",
  [ALICE]: "alice",
  [ORG_AGENT_PUBKEY]: "Org agent",
};

const ctx = {
  viewer: ME,
  agentPubkey: ORG_AGENT_PUBKEY,
  isShaper: true,
  nameOf: (pubkey) => names[pubkey] ?? pubkey.slice(0, 8),
};

function event(
  kind,
  content,
  tags,
  id = "aa".repeat(32),
  pubkey = ORG_AGENT_PUBKEY,
) {
  return {
    id,
    pubkey,
    kind,
    content: JSON.stringify(content),
    created_at: 10,
    tags,
  };
}

test("kickers: asking / suggesting / drafted", () => {
  const asking = event(50100, { title: "Ask" }, [
    ["n", ME],
    ["t", "project"],
  ]);
  assert.equal(kickerFor(asking, ctx, null).text, "AI is asking you");
  assert.equal(kickerFor(asking, ctx, null).kind, "asking");

  const suggesting = event(50100, { title: "Suggest", suggested_dri: ALICE }, [
    ["n", "shaper"],
    ["t", "project"],
    ["p", ALICE, "", "suggested"],
  ]);
  assert.equal(
    kickerFor(suggesting, ctx, ALICE).text,
    "AI is suggesting for alice",
  );
  assert.equal(kickerFor(suggesting, ctx, ALICE).kind, "suggesting");

  const drafted = event(50100, { title: "Draft" }, [
    ["n", "shaper"],
    ["t", "project"],
  ]);
  assert.equal(kickerFor(drafted, ctx, null).text, "Drafted by the agent");
  assert.equal(kickerFor(drafted, ctx, null).kind, "drafted");
});

test("each card type from 50100 / 39101 / 39102", () => {
  const draft = classifyEvent(
    event(50100, { title: "Hall" }, [
      ["n", ME],
      ["t", "project"],
      ["e", RECEIPT, "", "receipt"],
    ]),
    ctx,
  );
  assert.equal(draft?.cardType, "draft");
  assert.equal(draft?.column, "needs_answer");
  assert.equal(draft?.receipts.length, 1);

  const offer = classifyEvent(
    event(
      39101,
      { title: "Rota", state: "offered", offered_to: ME },
      [
        ["d", ITEM],
        ["s", "offered"],
        ["p", ME, "", "offered"],
      ],
      "bb".repeat(32),
      ALICE,
    ),
    ctx,
  );
  assert.equal(offer?.cardType, "offer");
  assert.equal(offer?.column, "needs_answer");

  const decision = classifyEvent(
    event(
      39102,
      {
        kind: "project",
        status: "open",
        needed: 2,
        votes: [{ p: ALICE, vote: "agree" }],
        payload: { title: "Confirm hall" },
      },
      [
        ["d", PROPOSAL],
        ["s", "open"],
        ["p", ME, "", "eligible"],
      ],
      "cc".repeat(32),
      "f".repeat(64),
    ),
    ctx,
  );
  assert.equal(decision?.cardType, "decision");
  assert.deepEqual(decision?.needed, { agrees: 1, needed: 2 });

  const done = classifyEvent(
    event(50100, { item: ITEM, why: "last child closed" }, [
      ["n", ME],
      ["t", "done"],
      ["u", ITEM],
    ]),
    ctx,
  );
  assert.equal(done?.cardType, "done");

  const review = classifyEvent(
    event(50100, { item: ITEM, recommendation: { type: "follow_up" } }, [
      ["n", "shaper"],
      ["t", "review"],
      ["i", ITEM],
    ]),
    ctx,
  );
  assert.equal(review?.cardType, "review");
});

test("three columns: hold and offered 39101s", () => {
  const held = event(
    39101,
    { title: "Held", state: "accepted", dri: ME },
    [
      ["d", "22222222-2222-4222-8222-222222222222"],
      ["s", "accepted"],
      ["p", ME],
    ],
    "dd".repeat(32),
    "f".repeat(64),
  );
  const offered = event(
    39101,
    {
      title: "Offered out",
      state: "offered",
      offered_by: ME,
      offered_to: ALICE,
    },
    [
      ["d", "33333333-3333-4333-8333-333333333333"],
      ["s", "offered"],
      ["p", ALICE, "", "offered"],
    ],
    "ee".repeat(32),
    ME,
  );
  const columns = classifyMyWork([held, offered], ctx);
  assert.equal(columns.you_hold[0]?.claim, "Held");
  assert.equal(columns.you_offered[0]?.claim, "Offered out");
});

test("receipts and claim bind the tagged e/ref rows", () => {
  const draft = event(50100, { title: "Hall", why: "gap" }, [
    ["e", RECEIPT, "", "receipt"],
    ["ref", "objectives@3#l_7f3a"],
  ]);
  assert.deepEqual(
    receiptsOf(draft).map((row) => row.kind),
    ["e", "ref"],
  );
  assert.equal(claimOf(draft), "Hall");
});

test("shadow drafts and done items are hidden", () => {
  const shadow = event(50100, { title: "Hidden" }, [
    ["n", ME],
    ["t", "project"],
    ["shadow", "true"],
  ]);
  assert.equal(classifyEvent(shadow, ctx), null);
  const doneItem = event(
    39101,
    { title: "Finished", state: "done", dri: ME },
    [
      ["d", ITEM],
      ["s", "done"],
      ["p", ME],
    ],
    "ff".repeat(32),
    ME,
  );
  assert.equal(classifyEvent(doneItem, ctx), null);
});
