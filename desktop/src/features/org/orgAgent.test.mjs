import assert from "node:assert/strict";
import test from "node:test";

import {
  excludeOrgAgent,
  isOrgAgentDm,
  NO_ORG_AGENT,
  parseOrgAgentFromShapers,
  pinOrgAgentDmFirst,
} from "./orgAgent.ts";

const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const AGENT =
  "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";
const ALICE =
  "953d3363262e86b770419834c53d2446409db6d918a57f8f339d495d54ab001f";
const BOB = "bb22a5299220cad76ffd46190ccbeede8ab5dc260faa28b6e5a2cb31b9aff260";

function shapers(
  content,
  { createdAt = 100, d = "shapers", kind = 39103 } = {},
) {
  return {
    id: "1".repeat(64),
    pubkey: "f".repeat(64),
    created_at: createdAt,
    kind,
    tags: [["d", d]],
    content: typeof content === "string" ? content : JSON.stringify(content),
    sig: "",
  };
}

function dm(id, participantPubkeys, channelType = "dm") {
  return { id, channelType, participantPubkeys };
}

test("parseOrgAgentFromShapers reads agent and agent_hosted from the newest 39103", () => {
  const older = shapers(
    { agent: ALICE, agent_hosted: false },
    { createdAt: 1 },
  );
  const newer = shapers(
    { agent: AGENT.toUpperCase(), agent_hosted: true },
    { createdAt: 2 },
  );

  assert.deepEqual(parseOrgAgentFromShapers([older, newer]), {
    pubkey: AGENT,
    hosted: true,
  });
});

test("parseOrgAgentFromShapers yields no agent for an empty list or a null agent", () => {
  assert.deepEqual(parseOrgAgentFromShapers([]), NO_ORG_AGENT);
  assert.deepEqual(
    parseOrgAgentFromShapers([shapers({ agent: null, agent_hosted: false })]),
    NO_ORG_AGENT,
  );
});

test("parseOrgAgentFromShapers ignores other kinds, other d tags, and malformed content", () => {
  const good = shapers({ agent: AGENT, agent_hosted: true }, { createdAt: 1 });
  const otherKind = shapers({ agent: ALICE }, { createdAt: 9, kind: 39100 });
  const otherD = shapers({ agent: ALICE }, { createdAt: 9, d: "other" });

  assert.equal(
    parseOrgAgentFromShapers([good, otherKind, otherD]).pubkey,
    AGENT,
  );
  assert.deepEqual(
    parseOrgAgentFromShapers([shapers("not json")]),
    NO_ORG_AGENT,
  );
  assert.deepEqual(
    parseOrgAgentFromShapers([shapers({ agent: "not-a-pubkey" })]),
    NO_ORG_AGENT,
  );
  assert.equal(
    parseOrgAgentFromShapers([shapers({ agent: AGENT, agent_hosted: "yes" })])
      .hosted,
    false,
  );
});

test("isOrgAgentDm is true only for the member's 1:1 with the agent", () => {
  assert.equal(isOrgAgentDm(dm("a", [AGENT, ME]), AGENT, ME), true);
  assert.equal(
    isOrgAgentDm(dm("a", [ME, AGENT.toUpperCase()]), AGENT, ME),
    true,
  );
  // Another member's DM with the agent, a group DM the agent sits in, and a
  // DM without the agent are not it.
  assert.equal(isOrgAgentDm(dm("b", [AGENT, ALICE]), AGENT, ME), false);
  assert.equal(isOrgAgentDm(dm("c", [AGENT, ME, ALICE]), AGENT, ME), false);
  assert.equal(isOrgAgentDm(dm("d", [ALICE, ME]), AGENT, ME), false);
  // Not a DM, or no agent known yet.
  assert.equal(isOrgAgentDm(dm("e", [AGENT, ME], "stream"), AGENT, ME), false);
  assert.equal(isOrgAgentDm(dm("a", [AGENT, ME]), null, ME), false);
});

test("isOrgAgentDm recognises the {member} identity the relay writes once it excludes the agent (Protocol §6.8)", () => {
  // The agent DM's `p` tags carry only the member; a self-only DM exists for
  // no other reason.
  assert.equal(isOrgAgentDm(dm("a", [ME]), AGENT, ME), true);
  assert.equal(isOrgAgentDm(dm("a", [ME, ME.toUpperCase()]), AGENT, ME), true);
  // Someone else's self-identity DM, or one without an org agent, is not it.
  assert.equal(isOrgAgentDm(dm("b", [ALICE]), AGENT, ME), false);
  assert.equal(isOrgAgentDm(dm("a", [ME]), null, ME), false);
  // No participant list means unknown, never the agent's.
  assert.equal(isOrgAgentDm(dm("z", []), AGENT, ME), false);
  assert.equal(isOrgAgentDm(dm("z", []), AGENT, null), false);
});

test("isOrgAgentDm while identity is still loading only accepts a 1:1 that lists the agent", () => {
  assert.equal(isOrgAgentDm(dm("a", [AGENT, ME]), AGENT, null), true);
  assert.equal(isOrgAgentDm(dm("c", [AGENT, ME, ALICE]), AGENT, null), false);
  // A self-only DM cannot be told apart from anyone else's until identity
  // resolves; the pin re-evaluates once it does.
  assert.equal(isOrgAgentDm(dm("a", [ME]), AGENT, null), false);
});

test("pinOrgAgentDmFirst moves the agent DM to the front and keeps the rest in order", () => {
  const alice = dm("alice", [ALICE, ME]);
  const bob = dm("bob", [BOB, ME]);
  const agent = dm("agent", [AGENT, ME]);

  assert.deepEqual(
    pinOrgAgentDmFirst([alice, bob, agent], AGENT, ME).map((c) => c.id),
    ["agent", "alice", "bob"],
  );
});

test("pinOrgAgentDmFirst returns the same array when nothing moves", () => {
  const alice = dm("alice", [ALICE, ME]);
  const agent = dm("agent", [AGENT, ME]);
  const alreadyFirst = [agent, alice];
  const noAgentDm = [alice];

  assert.equal(pinOrgAgentDmFirst(alreadyFirst, AGENT, ME), alreadyFirst);
  assert.equal(pinOrgAgentDmFirst(noAgentDm, AGENT, ME), noAgentDm);
  assert.equal(pinOrgAgentDmFirst([alice, agent], null, ME).length, 2);
  assert.equal(pinOrgAgentDmFirst([alice, agent], null, ME)[0], alice);
});

test("excludeOrgAgent drops the org agent from a member's agents", () => {
  const mine = { pubkey: ALICE, name: "mine" };
  const org = { pubkey: AGENT.toUpperCase(), name: "org" };

  assert.deepEqual(excludeOrgAgent([mine, org], AGENT), [mine]);
  const untouched = [mine];
  assert.equal(excludeOrgAgent(untouched, AGENT), untouched);
  assert.equal(excludeOrgAgent([mine, org], null).length, 2);
});
