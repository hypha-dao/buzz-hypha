import assert from "node:assert/strict";
import test from "node:test";

import {
  announcementMentions,
  chatDraftFromMessage,
  directionBodyLines,
  directionDraftText,
  chatHolderLabel,
  commandForDraft,
  driDraftSentence,
  driRequest,
  nameHoldersInChat,
  omitSuggestedHolderWhenOffered,
  openChatDrafts,
  proposalAnnouncement,
  shortProposalAnnouncement,
  stripProposalOpenLink,
} from "./chatDraft.ts";

const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const ADA = "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";
const AGENT =
  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ITEM = "11111111-1111-4111-8111-111111111111";

function message(id, tags, createdAt = Number(id)) {
  return { id, createdAt, pubkey: AGENT, tags };
}

test("a project message is a draft, not a signed agree", () => {
  const draft = chatDraftFromMessage(
    message("1", [
      ["from", ME],
      ["project", "Fix the hall", "The roof leaks when it rains."],
      ["due", "1700000000"],
      ["p", ADA, "", "suggested"],
    ]),
    AGENT,
  );
  assert.equal(draft.kind, "project");
  assert.equal(draft.title, "Fix the hall");
  assert.equal(draft.suggestedDri, ADA);
  const command = commandForDraft({
    draft,
    title: "Fix the hall",
    brief: "The roof leaks when it rains.",
    body: "",
    dueAt: 1700000000,
    why: "",
    directionBase: 0,
  });
  assert.equal(
    command.tags.some((tag) => tag[0] === "vote" && tag[1] === "agree"),
    false,
  );
  assert.equal(JSON.parse(command.content).suggested_dri, ADA);
  const cleared = commandForDraft({
    draft,
    title: "Fix the hall",
    brief: "The roof leaks when it rains.",
    body: "",
    dueAt: 1700000000,
    why: "",
    directionBase: 0,
    suggestedDri: null,
  });
  assert.equal("suggested_dri" in JSON.parse(cleared.content), false);
  const chosen = commandForDraft({
    draft,
    title: "Fix the hall",
    brief: "The roof leaks when it rains.",
    body: "",
    dueAt: 1700000000,
    why: "",
    directionBase: 0,
    suggestedDri: ME,
  });
  assert.equal(JSON.parse(chosen.content).suggested_dri, ME);
});

test("direction, dri, removal, and shapers drafts share the publish path", () => {
  const direction = chatDraftFromMessage(
    message("2", [
      ["from", ME],
      ["direction", "mission", "We keep the hall open for the town."],
    ]),
    AGENT,
  );
  assert.equal(direction.kind, "direction");
  const dri = chatDraftFromMessage(
    message("3", [
      ["from", ME],
      ["dri", ITEM, ADA],
    ]),
    AGENT,
  );
  assert.equal(dri.kind, "dri");
  const renamed = commandForDraft({
    draft: dri,
    title: "",
    brief: "",
    body: "",
    dueAt: 0,
    why: "From the draft.",
    directionBase: 0,
    holder: ME,
  });
  assert.equal(renamed.tags.find((tag) => tag[0] === "p")?.[1], ME);
  const removal = chatDraftFromMessage(
    message("4", [
      ["from", ME],
      ["remove", ITEM, "proposal"],
    ]),
    AGENT,
  );
  assert.equal(removal.kind, "remove-project");
  const add = chatDraftFromMessage(
    message("5", [
      ["from", ME],
      ["shapers", "add", ADA, "She already holds the hall."],
    ]),
    AGENT,
  );
  assert.equal(add.kind, "shapers-add");
  const rules = chatDraftFromMessage(
    message("6", [
      ["from", ME],
      ["shapers", "rules", JSON.stringify({ shapers: "all" }), "604800", ""],
    ]),
    AGENT,
  );
  assert.equal(rules.kind, "shapers-rules");
  const agentChoice = chatDraftFromMessage(
    message("7", [
      ["from", ME],
      ["shapers", "agent", "", "Back to the hosted agent."],
    ]),
    AGENT,
  );
  assert.equal(agentChoice.kind, "shapers-agent");
  assert.equal(agentChoice.pubkey, null);
  for (const draft of [direction, dri, removal, add, rules, agentChoice]) {
    const command = commandForDraft({
      draft,
      title: "",
      brief: "",
      body: draft.kind === "direction" ? draft.body : "",
      dueAt: 0,
      why: "From the draft.",
      directionBase: 1,
    });
    assert.equal(
      command.tags.some((tag) => tag[0] === "vote"),
      false,
    );
  }
});

test("a newer draft of the same subject replaces the card", () => {
  const first = message(
    "1",
    [
      ["from", ME],
      ["project", "Fix the hall", "The roof leaks."],
      ["due", "1700000000"],
    ],
    1,
  );
  const second = message(
    "2",
    [
      ["from", ME],
      ["project", "Fix the hall roof", "The roof leaks when it rains."],
      ["due", "1700000000"],
    ],
    2,
  );
  const open = openChatDrafts([first, second], AGENT, new Set());
  assert.equal(open.length, 1);
  assert.equal(open[0].messageId, "2");
  assert.equal(open[0].title, "Fix the hall roof");
  const hidden = openChatDrafts([first, second], AGENT, new Set(["2"]));
  assert.equal(hidden.length, 0);
  const later = message(
    "3",
    [
      ["from", ME],
      ["project", "Paint the door", "The door is bare."],
      ["due", "1700000000"],
    ],
    3,
  );
  const again = openChatDrafts([first, second, later], AGENT, new Set(["2"]));
  assert.equal(again.length, 1);
  assert.equal(again[0].messageId, "3");
});

test("a dri request without tags still opens a draft", () => {
  assert.deepEqual(driRequest("which projects dont have a DRI yet?"), null);
  assert.deepEqual(
    driRequest("make a proposal to set me as DRI for build best site"),
    { who: "me", item: "build best site" },
  );
  const human = message("8", [], 8);
  human.pubkey = ME;
  human.body = "make a proposal to set me as DRI for build best site";
  const agent = message("9", [], 9);
  agent.body =
    "Drafting a DRI proposal to set you as holder of Build Best Site. Publish it when you are ready to open the vote.";
  const open = openChatDrafts([human, agent], AGENT, new Set(), [
    {
      id: ITEM,
      title: "Build Best Site",
      state: "open",
      dri: null,
    },
  ]);
  assert.equal(open.length, 1);
  assert.equal(open[0].kind, "dri");
  assert.equal(open[0].itemId, ITEM);
  assert.equal(open[0].pubkey, ME);
  assert.equal(open[0].from, ME);
  const rewritten = { ...agent };
  rewritten.body = driDraftSentence("Build Best Site", "Travolta");
  const again = openChatDrafts([human, rewritten], AGENT, new Set(), [
    {
      id: ITEM,
      title: "Build Best Site",
      state: "open",
      dri: null,
    },
  ]);
  assert.equal(again.length, 1);
  assert.equal(again[0].messageId, "9");
});

test("a suggested holder in chat is the person's name", () => {
  const prefix = ADA.slice(0, 8);
  const profiles = {
    [ADA]: { displayName: "Ada Lovelace", name: "ada" },
  };
  assert.equal(chatHolderLabel(ADA, profiles), "Ada Lovelace");
  assert.equal(chatHolderLabel(ME, profiles), null);
  const ticket = nameHoldersInChat(
    `Want me to offer this ticket? Suggested holder: ${prefix}.`,
    [["h", "room"]],
    profiles,
  );
  assert.equal(
    ticket,
    "Want me to offer this ticket? Suggested holder: Ada Lovelace.",
  );
  assert.equal(ticket.includes(prefix), false);
  const project = nameHoldersInChat(
    `Opening that project. Suggested holder: ${ADA}.`,
    [
      ["from", ME],
      ["project", "Hall roof", "Fix it."],
      ["p", ADA, "", "suggested"],
    ],
    profiles,
  );
  assert.equal(
    project,
    "Opening that project. Suggested holder: Ada Lovelace.",
  );
  const dri = nameHoldersInChat(
    `A draft to name ${prefix} as the holder. Open it, then publish.`,
    [
      ["from", ME],
      ["dri", ITEM, ADA],
    ],
    profiles,
  );
  assert.match(dri, /Ada Lovelace/);
  assert.equal(dri.includes(prefix), false);
  const offered = omitSuggestedHolderWhenOffered(
    "Opening the ticket Order tiles under Hall roof, offered to you, due in 7 days. Suggested holder: TRAVOLTA.",
  );
  assert.equal(
    offered,
    "Opening the ticket Order tiles under Hall roof, offered to you, due in 7 days.",
  );
  const stillAsking = omitSuggestedHolderWhenOffered(
    "Order tiles. Buy the tiles. Want me to offer this ticket? Suggested holder: TRAVOLTA.",
  );
  assert.match(stillAsking, /Suggested holder: TRAVOLTA/);
});

test("objectives written as one paragraph show one line each", () => {
  const body =
    "By the end of November 2026, the four other Hypha members have used the app to set Hypha's direction; today only Vlad uses it. By the end of January 2027, each Hypha member starts most weeks' work from the AI's suggestion of what to do and how. By the end of March 2027, one small team outside Hypha runs its daily work on the app without Vlad's help; none is chosen yet.";
  assert.deepEqual(directionBodyLines("objectives", body), [
    "By the end of November 2026, the four other Hypha members have used the app to set Hypha's direction; today only Vlad uses it.",
    "By the end of January 2027, each Hypha member starts most weeks' work from the AI's suggestion of what to do and how.",
    "By the end of March 2027, one small team outside Hypha runs its daily work on the app without Vlad's help; none is chosen yet.",
  ]);
  assert.deepEqual(
    directionBodyLines(
      "objectives",
      "1. Book the hall by March.\n2. Pay growers the week they sell.",
    ),
    ["Book the hall by March.", "Pay growers the week they sell."],
  );
  assert.deepEqual(
    directionBodyLines(
      "strategy",
      "We refuse new orgs. Hypha and Hypha Buzz come first.\nHypha Buzz wins when the two compete.",
    ),
    [
      "We refuse new orgs. Hypha and Hypha Buzz come first.",
      "Hypha Buzz wins when the two compete.",
    ],
  );
  const situation =
    "Running one season. The Saturday stall has never missed a week. Demand on a weekday night is only assumed.";
  assert.deepEqual(directionBodyLines("situation", situation), [situation]);
  assert.equal(
    directionDraftText("objectives", body, ["Book the hall by March."]),
    `Book the hall by March.\n${directionBodyLines("objectives", body).join("\n")}`,
  );
});

test("a dri draft names the project and the person", () => {
  assert.equal(
    driDraftSentence("Internal Dogfooding at Hypha", "Ada"),
    "A draft to name Ada as the holder of “Internal Dogfooding at Hypha”. Open it, then publish.",
  );
  assert.equal(
    driDraftSentence("Internal Dogfooding at Hypha", null),
    "A draft to name a holder for “Internal Dogfooding at Hypha”. Open it, then publish.",
  );
  assert.match(driDraftSentence("Hall [roof]", "Ada"), /Hall \\\[roof\\\]/);
});

test("the shapers announcement names the proposal and the card opens it", () => {
  const text = proposalAnnouncement("project", "Fix the hall");
  assert.equal(text, "Opened a project proposal: Fix the hall.");
  assert.equal(text.includes("["), false);
  assert.equal(
    announcementMentions(text, {
      kind: "project",
      title: "Fix the hall",
      slug: null,
    }),
    true,
  );
  const older =
    "Opened a project proposal: Fix the hall.\n\n[Open it](/org/proposal/33333333-3333-4333-8333-333333333333)";
  assert.equal(
    stripProposalOpenLink(older),
    "Opened a project proposal: Fix the hall.",
  );
  assert.equal(
    shortProposalAnnouncement(
      "Opened a strategy proposal: We refuse new orgs.\nHypha Buzz wins..",
    ),
    "Opened a strategy proposal.",
  );
  assert.equal(
    shortProposalAnnouncement(
      proposalAnnouncement("objectives", "Book the hall"),
    ),
    "Opened an objectives proposal.",
  );
  assert.equal(shortProposalAnnouncement("hello"), "hello");
});
