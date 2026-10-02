import assert from "node:assert/strict";
import test from "node:test";

import {
  ORG_AGENT_TYPING_ACK_MS,
  agentReplyArrived,
  latestOrgAgentMessage,
  mergeTypingPubkey,
  orgAgentPromptFromFresh,
  orgAgentTypingActive,
  orgAgentWillAnswer,
  previousMessageIsOrgAgentQuestion,
  withOrgAgentTypingProfile,
} from "./orgAgentTyping.ts";
import { ORG_AGENT_LABEL } from "./orgAgent.ts";

const AGENT =
  "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";
const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";

function ack(overrides = {}) {
  return {
    generation: 1,
    startedAt: 1_000,
    sentAtSec: 1,
    baselineAgentMessageId: "old",
    ...overrides,
  };
}

test("latestOrgAgentMessage is the newest row from the agent", () => {
  const messages = [
    { id: "a1", pubkey: AGENT, createdAt: 1 },
    { id: "me", pubkey: ME, createdAt: 2 },
    { id: "a2", signerPubkey: AGENT.toUpperCase(), createdAt: 3 },
  ];
  assert.deepEqual(latestOrgAgentMessage(messages, AGENT), {
    id: "a2",
    createdAt: 3,
  });
  assert.equal(latestOrgAgentMessage(messages.slice(1, 2), AGENT), null);
});

test("org agent typing stays up until a newer agent message or the timeout", () => {
  const pending = ack();
  assert.equal(
    orgAgentTypingActive({
      ack: pending,
      enabled: true,
      now: pending.startedAt + 1_000,
      latestAgentId: "old",
      latestAgentCreatedAt: 1,
    }),
    true,
  );
  assert.equal(
    orgAgentTypingActive({
      ack: pending,
      enabled: false,
      now: pending.startedAt + 1_000,
      latestAgentId: "old",
      latestAgentCreatedAt: 1,
    }),
    false,
  );
  assert.equal(
    orgAgentTypingActive({
      ack: pending,
      enabled: true,
      now: pending.startedAt + ORG_AGENT_TYPING_ACK_MS,
      latestAgentId: "old",
      latestAgentCreatedAt: 1,
    }),
    false,
  );
  assert.equal(
    orgAgentTypingActive({
      ack: pending,
      enabled: true,
      now: pending.startedAt + 1_000,
      latestAgentId: "reply",
      latestAgentCreatedAt: 20,
    }),
    false,
  );
});

test("agentReplyArrived ignores history that loads after a send with no baseline", () => {
  assert.equal(
    agentReplyArrived({
      baselineId: null,
      latestId: "welcome",
      latestCreatedAt: 10,
      sentAtSec: 100,
    }),
    false,
  );
  assert.equal(
    agentReplyArrived({
      baselineId: null,
      latestId: "reply",
      latestCreatedAt: 100,
      sentAtSec: 100,
    }),
    true,
  );
  assert.equal(
    agentReplyArrived({
      baselineId: "old",
      latestId: "old",
      latestCreatedAt: 200,
      sentAtSec: 100,
    }),
    false,
  );
});

test("a channel shows typing only when the org agent will answer", () => {
  assert.equal(
    orgAgentWillAnswer({
      answersEveryLine: true,
      body: "hello",
      followsAgentQuestion: false,
      mentionsAgent: false,
    }),
    true,
  );
  assert.equal(
    orgAgentWillAnswer({
      answersEveryLine: false,
      body: "hello",
      followsAgentQuestion: false,
      mentionsAgent: false,
    }),
    false,
  );
  assert.equal(
    orgAgentWillAnswer({
      answersEveryLine: false,
      body: "change the brief, make it more descriptive",
      followsAgentQuestion: false,
      mentionsAgent: true,
    }),
    true,
  );
  assert.equal(
    orgAgentWillAnswer({
      answersEveryLine: false,
      body: "create a new project, test out the shapers chat",
      followsAgentQuestion: false,
      mentionsAgent: false,
    }),
    true,
  );
  const asked = [
    { id: "q", pubkey: AGENT, createdAt: 1, body: "Which brief?" },
  ];
  assert.equal(previousMessageIsOrgAgentQuestion(asked, AGENT), true);
  assert.equal(previousMessageIsOrgAgentQuestion(asked, ME), false);
});

test("a live prompt from someone else starts typing until the agent replies", () => {
  const human = {
    id: "h",
    pubkey: ME,
    createdAt: 10,
    body: "please update the project brief",
    tags: [["p", AGENT]],
  };
  const reply = {
    id: "a",
    pubkey: AGENT,
    createdAt: 11,
    body: "Updated.",
    tags: [],
  };
  assert.equal(
    orgAgentPromptFromFresh([human], [human], AGENT, false)?.id,
    "h",
  );
  assert.equal(
    orgAgentPromptFromFresh([human, reply], [human, reply], AGENT, false),
    null,
  );
  assert.equal(
    orgAgentPromptFromFresh(
      [{ id: "chat", pubkey: ME, createdAt: 9, body: "lunch?", tags: [] }],
      [{ id: "chat", pubkey: ME, createdAt: 9, body: "lunch?", tags: [] }],
      AGENT,
      false,
    ),
    null,
  );
});

test("mergeTypingPubkey keeps the list identity until the agent is added", () => {
  const existing = ["abc"];
  assert.equal(mergeTypingPubkey(existing, null), existing);
  assert.equal(mergeTypingPubkey(existing, "ABC"), existing);
  assert.deepEqual(mergeTypingPubkey(existing, AGENT), ["abc", AGENT]);
});

test("withOrgAgentTypingProfile fills Org. Agent until a display name exists", () => {
  const named = {
    [AGENT]: {
      displayName: "Kept",
      avatarUrl: null,
      nip05Handle: null,
      ownerPubkey: null,
    },
  };
  assert.equal(withOrgAgentTypingProfile(named, AGENT, true), named);
  assert.equal(withOrgAgentTypingProfile(undefined, AGENT, false), undefined);

  const filled = withOrgAgentTypingProfile(undefined, AGENT, true);
  assert.equal(filled?.[AGENT]?.displayName, ORG_AGENT_LABEL);
  assert.equal(filled?.[AGENT]?.isAgent, true);
});
