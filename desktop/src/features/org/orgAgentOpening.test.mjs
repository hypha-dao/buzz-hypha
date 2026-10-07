import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  isCannedOrgAgentOpening,
  ORG_AGENT_FIRST_MESSAGE,
  ORG_AGENT_OPENING,
  orgAgentWelcomeText,
} from "./orgAgentOpening.ts";

const AGENT = "aa".repeat(32);

test("the opening says what the org agent is", () => {
  assert.equal(
    ORG_AGENT_OPENING.lead,
    "I'm super intelligence of your organization.",
  );
  assert.deepEqual(ORG_AGENT_OPENING.items, []);
  assert.equal(
    orgAgentWelcomeText(),
    "I'm super intelligence of your organization.",
  );
});

test("the posted first message matches the org agent welcome", () => {
  const rust = readFileSync(
    new URL(
      "../../../../crates/buzz-org-agent/src/dm_chat.rs",
      import.meta.url,
    ),
    "utf8",
  );
  assert.ok(
    rust.includes(ORG_AGENT_FIRST_MESSAGE),
    "WELCOME_LINES must match ORG_AGENT_FIRST_MESSAGE",
  );
  assert.match(ORG_AGENT_FIRST_MESSAGE, /^Hey, I'm glad to connect!/);
  assert.match(
    ORG_AGENT_FIRST_MESSAGE,
    /Do you have time to set up your organization now\?$/,
  );
  assert.doesNotMatch(ORG_AGENT_FIRST_MESSAGE, /Are you shaping this alone/);
});

test("a canned opening from the org agent is hidden; a person's line is not", () => {
  assert.equal(
    isCannedOrgAgentOpening(
      { body: ORG_AGENT_FIRST_MESSAGE, signerPubkey: AGENT },
      AGENT,
    ),
    false,
  );
  assert.equal(
    isCannedOrgAgentOpening(
      {
        body: [
          "I draft for this organization — what it is for, the work, who decides, and your profile. You decide what becomes real. Nothing I write changes the org until the right person agrees.",
          "",
          "Direction — Mission, vision, where you stand, objectives, and strategy. I draft each one and say what's weak.",
          "Work — Projects, tickets, and who should hold them. Only the named person accepts.",
          "Shapers — Who decides, and how many of them must agree before something passes.",
          "Profile — What you do, the work you want, and your links, so offers go to the right person.",
          "Questions — Ask about anything the organization has already written down.",
        ].join("\n"),
        signerPubkey: AGENT,
      },
      AGENT,
    ),
    true,
  );
  assert.equal(
    isCannedOrgAgentOpening(
      { body: "Hey. I'm Org. Agent.", pubkey: AGENT },
      AGENT,
    ),
    true,
  );
  assert.equal(
    isCannedOrgAgentOpening(
      {
        body: "I draft, you decide. Are you shaping this alone, or with other people?",
        signerPubkey: AGENT,
      },
      AGENT,
    ),
    true,
  );
  assert.equal(
    isCannedOrgAgentOpening(
      {
        body: [
          "I draft. You decide. Nothing is real until the right person agrees.",
          "",
          "Direction — Mission, vision, where you stand, objectives, and strategy, with an honest read on each.",
          "Work — Projects, tickets, and who holds them.",
          "Shapers — Who decides, and how many must agree.",
          "Profile — About you, the work you want, and your links.",
          "Questions — Answers from what's already written down.",
          "",
          "Are you shaping this alone, or with other people?",
        ].join("\n"),
        signerPubkey: AGENT,
      },
      AGENT,
    ),
    true,
  );
  assert.equal(
    isCannedOrgAgentOpening(
      { body: "Hey. I'm Org. Agent.", signerPubkey: "bb".repeat(32) },
      AGENT,
    ),
    false,
  );
  assert.equal(
    isCannedOrgAgentOpening(
      { body: "Let's set a mission.", signerPubkey: AGENT },
      AGENT,
    ),
    false,
  );
  assert.equal(
    isCannedOrgAgentOpening(
      { body: "Hey. I'm Org. Agent.", pubkey: AGENT },
      null,
    ),
    false,
  );

  const hook = readFileSync(
    new URL("../channels/ui/useChannelPaneMessages.ts", import.meta.url),
    "utf8",
  );
  assert.match(hook, /isCannedOrgAgentOpening\(/);

  const timeline = readFileSync(
    new URL("../messages/ui/MessageTimeline.tsx", import.meta.url),
    "utf8",
  );
  assert.match(timeline, /DirectMessageIntroBlock/);
  const intro = readFileSync(
    new URL("../messages/ui/DirectMessageIntroBlock.tsx", import.meta.url),
    "utf8",
  );
  assert.match(intro, /data-testid="org-agent-opening"/);
  assert.match(intro, /opening\.lead/);
  assert.match(intro, /opening\.items\.length/);
  assert.doesNotMatch(intro, /opening\.prompt/);
});
