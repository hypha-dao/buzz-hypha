import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  isCannedOrgAgentOpening,
  ORG_AGENT_OPENING,
  orgAgentWelcomeText,
} from "./orgAgentOpening.ts";

const AGENT = "aa".repeat(32);

test("the opening says what the org agent is for", () => {
  assert.equal(
    ORG_AGENT_OPENING.lead,
    "I draft for this organization — what it is for, the work, who decides, and your profile. You decide what becomes real. Nothing I write changes the org until the right person agrees.",
  );
  assert.deepEqual(
    ORG_AGENT_OPENING.items.map((item) => item.title),
    ["Direction", "Work", "Shapers", "Profile", "Questions"],
  );
  for (const item of ORG_AGENT_OPENING.items) {
    assert.ok(item.detail.length > 0, item.title);
  }
});

test("the posted welcome is the same opening the DM shows", () => {
  const rust = readFileSync(
    new URL(
      "../../../../crates/buzz-org-agent/src/dm_chat.rs",
      import.meta.url,
    ),
    "utf8",
  );
  const welcome = orgAgentWelcomeText();
  assert.ok(
    rust.includes(welcome),
    "WELCOME_LINES must match orgAgentWelcomeText()",
  );
  assert.match(welcome, /^I draft for this organization/);
  assert.match(
    welcome,
    /Nothing I write changes the org until the right person agrees\./,
  );
  assert.doesNotMatch(welcome, /Are you shaping this alone/);
});

test("a canned opening from the org agent is hidden; a person's line is not", () => {
  assert.equal(
    isCannedOrgAgentOpening(
      { body: orgAgentWelcomeText(), signerPubkey: AGENT },
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
  assert.match(intro, /opening\.items\.map/);
  assert.doesNotMatch(intro, /opening\.prompt/);
});
