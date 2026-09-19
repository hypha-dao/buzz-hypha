import assert from "node:assert/strict";
import test from "node:test";

import {
  KIND_IO_ACCEPT,
  KIND_IO_DECLINE,
  KIND_IO_DIRECTION_PROPOSE,
  KIND_IO_DONE,
  KIND_IO_DRAFT_DECIDE,
  KIND_IO_DRI_PROPOSE,
  KIND_IO_HEALTH_RATE,
  KIND_IO_JOIN_PROPOSE,
  KIND_IO_MONEY_PROPOSE,
  KIND_IO_MONEY_RELEASED,
  KIND_IO_OFFER,
  KIND_IO_PROFILE_SET,
  KIND_IO_PROJECT_PROPOSE,
  KIND_IO_RELEASE,
  KIND_IO_REOPEN,
  KIND_IO_SET_DUE,
  KIND_IO_SHAPER_ACCEPT,
  KIND_IO_SHAPER_STEP_DOWN,
  KIND_IO_SHAPERS_PROPOSE,
  KIND_IO_TICKET_CREATE,
  KIND_IO_VOTE,
} from "../../shared/constants/kinds.ts";
import {
  buildIoAccept,
  buildIoDecline,
  buildIoDirectionPropose,
  buildIoDone,
  buildIoDraftDecide,
  buildIoDriPropose,
  buildIoHealthRate,
  buildIoJoinPropose,
  buildIoMoneyPropose,
  buildIoMoneyReleased,
  buildIoOffer,
  buildIoProfileSet,
  buildIoProjectPropose,
  buildIoRelease,
  buildIoReopen,
  buildIoSetDue,
  buildIoShaperAccept,
  buildIoShaperStepDown,
  buildIoShapersPropose,
  buildIoTicketCreate,
  buildIoVote,
} from "./commands.ts";

const DRAFT = "ab".repeat(32);
const ITEM = "11111111-1111-4111-8111-111111111111";
const PARENT = "22222222-2222-4222-8222-222222222222";
const PROPOSAL = "33333333-3333-4333-8333-333333333333";
const PUBKEY = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const RECEIPT = "cd".repeat(32);

test("shapers propose add keeps op, p, optional vote — Protocol §4.8 50001", () => {
  const event = buildIoShapersPropose(
    { op: "add", pubkey: PUBKEY, why: "founding" },
    true,
  );
  assert.equal(event.kind, KIND_IO_SHAPERS_PROPOSE);
  assert.deepEqual(event.tags, [
    ["op", "add"],
    ["p", PUBKEY],
    ["vote", "agree"],
  ]);
  assert.equal(event.content, JSON.stringify({ why: "founding" }));
});

test("shapers propose agent without p is the hosted default", () => {
  const event = buildIoShapersPropose({ op: "agent" });
  assert.deepEqual(event.tags, [["op", "agent"]]);
  assert.equal(event.content, "{}");
});

test("direction propose carries d, base, draft e, vote — §4.8 50002", () => {
  const event = buildIoDirectionPropose({
    slug: "mission",
    base: 3,
    body: "Build the hall.",
    draftId: DRAFT,
    voteAgree: true,
  });
  assert.equal(event.kind, KIND_IO_DIRECTION_PROPOSE);
  assert.deepEqual(event.tags, [
    ["d", "mission"],
    ["base", "3"],
    ["e", DRAFT, "", "draft"],
    ["vote", "agree"],
  ]);
});

test("vote tags e as the proposal uuid, not an event id", () => {
  const event = buildIoVote({ proposal: PROPOSAL, vote: "agree" });
  assert.equal(event.kind, KIND_IO_VOTE);
  assert.deepEqual(event.tags, [
    ["e", PROPOSAL],
    ["vote", "agree"],
  ]);
});

test("shaper accept and step down tag sets", () => {
  assert.deepEqual(buildIoShaperAccept(PROPOSAL).tags, [["e", PROPOSAL]]);
  assert.equal(buildIoShaperAccept(PROPOSAL).kind, KIND_IO_SHAPER_ACCEPT);
  assert.deepEqual(buildIoShaperStepDown("tired").tags, []);
  assert.equal(buildIoShaperStepDown("tired").kind, KIND_IO_SHAPER_STEP_DOWN);
  assert.equal(buildIoShaperStepDown("tired").content, JSON.stringify({ why: "tired" }));
});

test("project propose draft tag is the settle marker", () => {
  const event = buildIoProjectPropose({
    title: "Hall",
    brief: "Build it",
    dueAt: 1_800_000_000,
    draftId: DRAFT,
    voteAgree: true,
  });
  assert.equal(event.kind, KIND_IO_PROJECT_PROPOSE);
  assert.deepEqual(event.tags, [
    ["e", DRAFT, "", "draft"],
    ["vote", "agree"],
  ]);
});

test("ticket create is u, optional p, optional draft", () => {
  const event = buildIoTicketCreate({
    parent: PARENT,
    title: "Electrics",
    brief: "Wire the hall",
    dueAt: 1_800_000_000,
    offerTo: PUBKEY,
    draftId: DRAFT,
  });
  assert.equal(event.kind, KIND_IO_TICKET_CREATE);
  assert.deepEqual(event.tags, [
    ["u", PARENT],
    ["p", PUBKEY],
    ["e", DRAFT, "", "draft"],
  ]);
});

test("offer, accept, decline, done, release, set-due, reopen", () => {
  assert.equal(buildIoOffer({ item: ITEM, pubkey: PUBKEY, draftId: DRAFT }).kind, KIND_IO_OFFER);
  assert.deepEqual(buildIoOffer({ item: ITEM, pubkey: PUBKEY, draftId: DRAFT }).tags, [
    ["i", ITEM],
    ["p", PUBKEY],
    ["e", DRAFT, "", "draft"],
  ]);
  assert.deepEqual(buildIoAccept(ITEM).tags, [["i", ITEM]]);
  assert.equal(buildIoAccept(ITEM).kind, KIND_IO_ACCEPT);
  assert.deepEqual(buildIoDecline(ITEM).tags, [["i", ITEM]]);
  assert.equal(buildIoDecline(ITEM).kind, KIND_IO_DECLINE);
  assert.deepEqual(buildIoDone({ item: ITEM, receiptId: RECEIPT, draftId: DRAFT }).tags, [
    ["i", ITEM],
    ["e", RECEIPT, "", "receipt"],
    ["e", DRAFT, "", "draft"],
  ]);
  assert.equal(buildIoDone({ item: ITEM }).kind, KIND_IO_DONE);
  assert.deepEqual(buildIoRelease(ITEM, "handing back").tags, [["i", ITEM]]);
  assert.equal(buildIoRelease(ITEM).kind, KIND_IO_RELEASE);
  assert.deepEqual(buildIoSetDue(ITEM, 1_800_000_000).tags, [
    ["i", ITEM],
    ["due", "1800000000"],
  ]);
  assert.equal(buildIoSetDue(ITEM, 1).kind, KIND_IO_SET_DUE);
  assert.deepEqual(buildIoReopen(ITEM).tags, [["i", ITEM]]);
  assert.equal(buildIoReopen(ITEM).kind, KIND_IO_REOPEN);
});

test("draft decide e has no draft marker — the command is the decision", () => {
  const event = buildIoDraftDecide({
    draftId: DRAFT,
    outcome: "decline",
    reason: "already_covered",
  });
  assert.equal(event.kind, KIND_IO_DRAFT_DECIDE);
  assert.deepEqual(event.tags, [
    ["e", DRAFT],
    ["outcome", "decline"],
    ["reason", "already_covered"],
  ]);
});

test("dri propose and reserved money/join keep the C-1 tag order", () => {
  assert.deepEqual(
    buildIoDriPropose({ item: ITEM, pubkey: PUBKEY, draftId: DRAFT, voteAgree: true }).tags,
    [
      ["i", ITEM],
      ["p", PUBKEY],
      ["e", DRAFT, "", "draft"],
      ["vote", "agree"],
    ],
  );
  assert.equal(
    buildIoDriPropose({ item: ITEM, pubkey: PUBKEY }).kind,
    KIND_IO_DRI_PROPOSE,
  );
  assert.deepEqual(
    buildIoMoneyPropose({
      item: ITEM,
      payee: PUBKEY,
      amount: "10",
      currency: "USD",
      draftId: DRAFT,
    }).tags,
    [
      ["i", ITEM],
      ["p", PUBKEY],
      ["e", DRAFT, "", "draft"],
    ],
  );
  assert.equal(
    buildIoMoneyPropose({
      item: ITEM,
      payee: PUBKEY,
      amount: "10",
      currency: "USD",
    }).kind,
    KIND_IO_MONEY_PROPOSE,
  );
  assert.deepEqual(
    buildIoMoneyReleased({
      proposal: PROPOSAL,
      tx: "0xabc",
      chain: "eth",
      contract: "0x1",
      amount: "10",
      currency: "USD",
    }).tags,
    [
      ["e", PROPOSAL],
      ["tx", "0xabc"],
    ],
  );
  assert.equal(
    buildIoMoneyReleased({
      proposal: PROPOSAL,
      tx: "0xabc",
      chain: "eth",
      contract: "0x1",
      amount: "10",
      currency: "USD",
    }).kind,
    KIND_IO_MONEY_RELEASED,
  );
  assert.deepEqual(buildIoJoinPropose(PUBKEY).tags, [["p", PUBKEY]]);
  assert.equal(buildIoJoinPropose(PUBKEY).kind, KIND_IO_JOIN_PROPOSE);
});

test("health rate and profile set", () => {
  const rate = buildIoHealthRate({
    item: ITEM,
    week: "2026-W38",
    band: "healthy",
  });
  assert.equal(rate.kind, KIND_IO_HEALTH_RATE);
  assert.deepEqual(rate.tags, [
    ["i", ITEM],
    ["week", "2026-W38"],
    ["band", "healthy"],
  ]);
  const profile = buildIoProfileSet({
    about: "I wire halls.",
    skills: ["electrics"],
    draftId: DRAFT,
  });
  assert.equal(profile.kind, KIND_IO_PROFILE_SET);
  assert.deepEqual(profile.tags, [["e", DRAFT, "", "draft"]]);
  assert.ok(!profile.tags.some((tag) => tag[0] === "p"));
});

test("builders refuse a self-invented tag by construction — no extra names", () => {
  const names = new Set(
    [
      ...buildIoDone({ item: ITEM, draftId: DRAFT }).tags,
      ...buildIoDirectionPropose({
        slug: "vision",
        base: 0,
        body: "x",
        draftId: DRAFT,
      }).tags,
    ].map((tag) => tag[0]),
  );
  for (const name of names) {
    assert.ok(
      ["i", "e", "d", "base", "vote"].includes(name),
      `unexpected tag ${name}`,
    );
  }
});
