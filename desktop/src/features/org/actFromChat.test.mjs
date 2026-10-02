import assert from "node:assert/strict";
import test from "node:test";

import { nextChatAct, voteAgreeFromChat } from "./actFromChat.ts";

const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const ADA = "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";
const AGENT =
  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

function turn(id, pubkey, tags, createdAt = Number(id)) {
  return { id, createdAt, pubkey, tags };
}

const HELD = [{ id: "item-hall", state: "accepted", dri: ME }];
const OPEN = [{ id: "item-open", state: "open", dri: null }];

test("a project tag is signed only by the person the agent answered", () => {
  const tags = [
    ["from", ME],
    ["project", "Fix the hall", "The roof leaks when it rains."],
    ["due", "1700000000"],
    ["p", ADA, "", "suggested"],
  ];
  const mine = nextChatAct({
    messages: [turn("1", AGENT, tags)],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: [],
    handledIds: new Set(),
  });
  assert.equal(mine.kind, "project");
  assert.equal(mine.title, "Fix the hall");
  assert.equal(mine.dueAt, 1700000000);
  assert.equal(mine.suggestedDri, ADA);

  const other = nextChatAct({
    messages: [turn("1", AGENT, tags)],
    currentPubkey: ADA,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME, ADA],
    items: [],
    handledIds: new Set(),
    room: "shapers",
  });
  assert.deepEqual(other, { kind: "skip", agentEventId: "1" });
});

test("done is signed only by the holder", () => {
  const tags = [
    ["from", ME],
    ["done", "item-hall"],
  ];
  const held = nextChatAct({
    messages: [turn("2", AGENT, tags)],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: HELD,
    handledIds: new Set(),
  });
  assert.deepEqual(held, {
    kind: "done",
    agentEventId: "2",
    itemId: "item-hall",
  });

  const notHolder = nextChatAct({
    messages: [
      turn("2", AGENT, [
        ["from", ADA],
        ["done", "item-hall"],
      ]),
    ],
    currentPubkey: ADA,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME, ADA],
    items: HELD,
    handledIds: new Set(),
    room: "shapers",
  });
  assert.equal(notHolder.kind, "skip");
});

test("a ticket is created only when the speaker holds the parent", () => {
  const tags = [
    ["from", ME],
    ["ticket", "item-hall", "Order tiles", "Buy the replacement tiles."],
    ["due", "1700001000"],
    ["p", ME],
  ];
  const created = nextChatAct({
    messages: [turn("3", AGENT, tags)],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: HELD,
    handledIds: new Set(),
  });
  assert.equal(created.kind, "ticket");
  assert.equal(created.parentId, "item-hall");
  assert.equal(created.offerTo, ME);

  const stranger = nextChatAct({
    messages: [turn("3", AGENT, tags)],
    currentPubkey: ADA,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME, ADA],
    items: HELD,
    handledIds: new Set(),
    room: "shapers",
  });
  assert.equal(stranger.kind, "skip");
});

test("a ticket with no holder is still created, open for a match", () => {
  const tags = [
    ["from", ME],
    ["ticket", "item-hall", "Order tiles", "Buy the replacement tiles."],
    ["due", "1700001000"],
  ];
  const created = nextChatAct({
    messages: [turn("3b", AGENT, tags)],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: HELD,
    handledIds: new Set(),
  });
  assert.equal(created.kind, "ticket");
  assert.equal(created.parentId, "item-hall");
  assert.equal(created.offerTo, undefined);
});

test("a DRI is named only for open work", () => {
  const named = nextChatAct({
    messages: [
      turn("4", AGENT, [
        ["from", ME],
        ["dri", "item-open", ADA],
      ]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME, ADA],
    items: OPEN,
    handledIds: new Set(),
    room: "shapers",
  });
  assert.deepEqual(named, {
    kind: "dri",
    agentEventId: "4",
    itemId: "item-open",
    pubkey: ADA,
  });

  const held = nextChatAct({
    messages: [
      turn("5", AGENT, [
        ["from", ME],
        ["dri", "item-hall", ADA],
      ]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: HELD,
    handledIds: new Set(),
  });
  assert.equal(held.kind, "skip");
});

test("one Shaper publishes a request made in the shapers room", () => {
  const act = nextChatAct({
    messages: [
      turn("6", AGENT, [
        ["from", ME],
        ["project", "Fix the hall", "The roof leaks when it rains."],
        ["due", "1700000000"],
      ]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: [],
    handledIds: new Set(),
    room: "shapers",
  });
  assert.equal(act.kind, "project");
  assert.equal(act.title, "Fix the hall");
});

test("one Shaper removes a project directly; several open a proposal", () => {
  const tags = [
    ["from", ME],
    ["remove", "item-hall"],
  ];
  const direct = nextChatAct({
    messages: [turn("7", AGENT, tags)],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: [{ id: "item-hall", state: "accepted", dri: ME, parent: null }],
    handledIds: new Set(),
  });
  assert.equal(direct.kind, "remove");
  assert.equal(direct.itemId, "item-hall");

  const proposal = nextChatAct({
    messages: [
      turn("8", AGENT, [
        ["from", ME],
        ["remove", "item-hall", "proposal"],
      ]),
    ],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME, ADA],
    items: [{ id: "item-hall", state: "accepted", dri: ME, parent: null }],
    handledIds: new Set(),
  });
  assert.equal(proposal.kind, "removeProposal");

  const outsider = nextChatAct({
    messages: [turn("7", AGENT, tags)],
    currentPubkey: ADA,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: [{ id: "item-hall", state: "accepted", dri: ME, parent: null }],
    handledIds: new Set(),
  });
  assert.equal(outsider.kind, "skip");
});

test("a ticket is removed only by its creator or the person who offered it", () => {
  const tags = [
    ["from", ME],
    ["remove", "item-tiles"],
  ];
  const ticket = {
    id: "item-tiles",
    state: "offered",
    dri: null,
    parent: "item-hall",
    createdBy: ME,
    offeredBy: ADA,
  };
  const creator = nextChatAct({
    messages: [turn("9", AGENT, tags)],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: [ticket],
    handledIds: new Set(),
  });
  assert.equal(creator.kind, "remove");

  const offerer = nextChatAct({
    messages: [
      turn("10", AGENT, [
        ["from", ADA],
        ["remove", "item-tiles"],
      ]),
    ],
    currentPubkey: ADA,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME, ADA],
    items: [ticket],
    handledIds: new Set(),
  });
  assert.equal(offerer.kind, "remove");

  const stranger = nextChatAct({
    messages: [
      turn("11", AGENT, [
        [
          "from",
          "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        ],
        ["remove", "item-tiles"],
      ]),
    ],
    currentPubkey:
      "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: [ticket],
    handledIds: new Set(),
  });
  assert.equal(stranger.kind, "skip");
});

test("a revise tag replaces that proposal for the Shaper it answers", () => {
  const id = "33333333-3333-4333-8333-333333333333";
  const tags = [
    ["from", ME],
    [
      "revise",
      id,
      "direction",
      "mission",
      "We host the hall and keep a garden.",
    ],
    ["base", "1"],
  ];
  const mine = nextChatAct({
    messages: [turn("2", AGENT, tags)],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: [ME],
    items: [],
    handledIds: new Set(),
    room: "shapers",
  });
  assert.equal(mine.kind, "revise");
  assert.equal(mine.reviseKind, "direction");
  assert.equal(mine.proposalId, id);
  assert.equal(mine.base, 1);
  assert.equal(voteAgreeFromChat("shapers"), false);
  assert.equal(voteAgreeFromChat("channel"), false);
  assert.equal(voteAgreeFromChat("dm"), false);
  assert.equal(voteAgreeFromChat(undefined), false);
});

test("a profile tag is the member's own about, interests, and socials", () => {
  const tags = [
    ["from", ME],
    ["profile", "I wire halls."],
    ["skills", "electrics", "grant writing"],
    [
      "socials",
      JSON.stringify([
        { network: "github", url: "https://github.com/travolta" },
        { network: "github", url: "https://evil.example/phish" },
      ]),
    ],
    ["limit", "2"],
  ];
  const act = nextChatAct({
    messages: [turn("9", AGENT, tags)],
    currentPubkey: ME,
    orgAgentPubkey: AGENT,
    shaperPubkeys: null,
    items: null,
    handledIds: new Set(),
    room: "dm",
  });
  assert.equal(act.kind, "profile");
  assert.equal(act.about, "I wire halls.");
  assert.deepEqual(act.skills, ["electrics", "grant writing"]);
  assert.deepEqual(act.socials, [
    { network: "github", url: "https://github.com/travolta" },
  ]);
  assert.equal(act.openLimit, 2);
  const someoneElse = nextChatAct({
    messages: [turn("9", AGENT, tags)],
    currentPubkey: ADA,
    orgAgentPubkey: AGENT,
    shaperPubkeys: null,
    items: null,
    handledIds: new Set(),
    room: "dm",
  });
  assert.equal(someoneElse.kind, "skip");
});
