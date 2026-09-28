import assert from "node:assert/strict";
import test from "node:test";

import {
  isPersonalAssistantAgent,
  isPersonalAssistantDm,
  PERSONAL_ASSISTANT_ENV_MARKER,
  PERSONAL_ASSISTANT_NAME,
  PERSONAL_ASSISTANT_SYSTEM_PROMPT_PREFIX,
  pickPersonalAssistantForRelay,
  pinPersonalAssistantDmFirst,
} from "./personalAssistant.ts";

const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const PA = "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";
const OTHER =
  "953d3363262e86b770419834c53d2446409db6d918a57f8f339d495d54ab001f";

function agent(overrides = {}) {
  return {
    name: PERSONAL_ASSISTANT_NAME,
    envVars: { [PERSONAL_ASSISTANT_ENV_MARKER]: "1" },
    systemPrompt: PERSONAL_ASSISTANT_SYSTEM_PROMPT_PREFIX,
    pubkey: PA,
    status: "running",
    relayUrl: "ws://localhost:3000",
    ...overrides,
  };
}

test("isPersonalAssistantAgent matches env marker", () => {
  assert.equal(isPersonalAssistantAgent(agent()), true);
  assert.equal(
    isPersonalAssistantAgent(
      agent({
        envVars: {},
        name: PERSONAL_ASSISTANT_NAME,
        systemPrompt: `${PERSONAL_ASSISTANT_SYSTEM_PROMPT_PREFIX} more`,
      }),
    ),
    true,
  );
  assert.equal(
    isPersonalAssistantAgent(
      agent({ envVars: {}, name: "Fizz", systemPrompt: "You are Fizz." }),
    ),
    false,
  );
});

test("pickPersonalAssistantForRelay prefers running on this relay", () => {
  const idle = agent({ status: "stopped", pubkey: OTHER });
  const running = agent({ status: "running" });
  const picked = pickPersonalAssistantForRelay(
    [idle, running],
    "ws://localhost:3000",
  );
  assert.equal(picked?.pubkey, PA);
});

test("isPersonalAssistantDm recognises the 1:1 with the PA", () => {
  const channel = {
    channelType: "dm",
    participantPubkeys: [ME, PA],
  };
  assert.equal(isPersonalAssistantDm(channel, PA, ME), true);
  assert.equal(
    isPersonalAssistantDm(
      { channelType: "dm", participantPubkeys: [ME, PA, OTHER] },
      PA,
      ME,
    ),
    false,
  );
  assert.equal(
    isPersonalAssistantDm(
      { channelType: "stream", participantPubkeys: [ME, PA] },
      PA,
      ME,
    ),
    false,
  );
});

test("pinPersonalAssistantDmFirst moves the PA DM to the front", () => {
  const channels = [
    { id: "a", channelType: "dm", participantPubkeys: [ME, OTHER] },
    { id: "pa", channelType: "dm", participantPubkeys: [ME, PA] },
  ];
  const pinned = pinPersonalAssistantDmFirst(channels, PA, ME);
  assert.equal(pinned[0].id, "pa");
  assert.equal(pinned.length, 2);
});
