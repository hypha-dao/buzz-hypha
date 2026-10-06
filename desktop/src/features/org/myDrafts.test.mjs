import assert from "node:assert/strict";
import test from "node:test";

import {
  closeDraft,
  closedDraftIds,
  MAX_CLOSED_DRAFTS,
  parseDraftLifecycle,
} from "./draftLifecycle.ts";
import { draftPageModel } from "./draftPage.ts";
import {
  channelOfTags,
  draftPublishableBy,
  ownChatDrafts,
} from "./myDrafts.ts";

const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const ADA = "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";
const AGENT =
  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DM = "dm-room";
const SHAPERS = "shapers-room";
const ITEM = "11111111-1111-4111-8111-111111111111";

function project(id, from, channelId, title, createdAt = Number(id)) {
  return {
    id,
    createdAt,
    pubkey: AGENT,
    channelId,
    tags: [
      ["h", channelId],
      ["from", from],
      ["project", title, `${title}, in full.`],
      ["due", "1700000000"],
    ],
  };
}

function direction(id, from, channelId, slug, body) {
  return {
    id,
    createdAt: Number(id),
    pubkey: AGENT,
    channelId,
    tags: [
      ["h", channelId],
      ["direction", slug, body],
      ["from", from],
    ],
  };
}

test("channelOfTags reads the h tag", () => {
  assert.equal(channelOfTags([["h", DM]]), DM);
  assert.equal(channelOfTags([["p", ME]]), null);
});

test("only drafts the viewer asked for are theirs, newest first", () => {
  const drafts = ownChatDrafts(
    [
      project("1", ME, DM, "Fix the hall"),
      direction("2", ME, SHAPERS, "mission", "Feed every family on the block."),
      project("3", ADA, SHAPERS, "Paint the fence"),
    ],
    AGENT,
    ME,
    new Set(),
  );
  assert.deepEqual(
    drafts.map((entry) => [entry.draft.messageId, entry.channelId]),
    [
      ["2", SHAPERS],
      ["1", DM],
    ],
  );
});

test("each room keeps its own newest draft of a subject", () => {
  const drafts = ownChatDrafts(
    [
      project("1", ME, DM, "Fix the hall"),
      project("2", ME, SHAPERS, "Paint the fence"),
      project("3", ME, DM, "Fix the hall roof"),
    ],
    AGENT,
    ME,
    new Set(),
  );
  assert.deepEqual(
    drafts.map((entry) => entry.draft.messageId),
    ["3", "2"],
  );
});

test("a published or deleted draft leaves the list", () => {
  const messages = [
    project("1", ME, DM, "Fix the hall"),
    project("2", ME, SHAPERS, "Paint the fence"),
  ];
  let lifecycle = parseDraftLifecycle(null);
  lifecycle = closeDraft(lifecycle, "1", "published");
  lifecycle = closeDraft(lifecycle, "2", "deleted");
  assert.deepEqual(
    ownChatDrafts(messages, AGENT, ME, closedDraftIds(lifecycle)),
    [],
  );
  const later = project("4", ME, DM, "Fix the hall again");
  assert.deepEqual(
    ownChatDrafts(
      [...messages, later],
      AGENT,
      ME,
      closedDraftIds(lifecycle),
    ).map((entry) => entry.draft.messageId),
    ["4"],
  );
});

test("lines from someone other than the agent are not drafts", () => {
  const forged = { ...project("1", ME, DM, "Fix the hall"), pubkey: ADA };
  assert.deepEqual(ownChatDrafts([forged], AGENT, ME, new Set()), []);
});

test("the lifecycle store parses defensively and stays bounded", () => {
  assert.deepEqual(parseDraftLifecycle("not json"), {
    published: [],
    deleted: [],
  });
  assert.deepEqual(parseDraftLifecycle('{"published":[1,"a"],"deleted":"x"}'), {
    published: ["a"],
    deleted: [],
  });
  let lifecycle = parseDraftLifecycle(null);
  for (let index = 0; index < MAX_CLOSED_DRAFTS + 5; index += 1) {
    lifecycle = closeDraft(lifecycle, `id-${index}`, "deleted");
  }
  assert.equal(lifecycle.deleted.length, MAX_CLOSED_DRAFTS);
  assert.equal(lifecycle.deleted.at(-1), `id-${MAX_CLOSED_DRAFTS + 4}`);
  assert.equal(lifecycle.deleted.includes("id-0"), false);
  const again = closeDraft(lifecycle, "id-10", "deleted");
  assert.equal(again.deleted.filter((id) => id === "id-10").length, 1);
});

test("a DRI draft is published by whoever asked; the rest by a Shaper", () => {
  const [{ draft: projectDraft }] = ownChatDrafts(
    [project("1", ME, DM, "Fix the hall")],
    AGENT,
    ME,
    new Set(),
  );
  assert.equal(draftPublishableBy(projectDraft, ME, []), false);
  assert.equal(draftPublishableBy(projectDraft, ME, [ME.toUpperCase()]), true);
  const dri = {
    kind: "dri",
    messageId: "9",
    createdAt: 9,
    from: ME,
    itemId: ITEM,
    pubkey: ME,
  };
  assert.equal(draftPublishableBy(dri, ME, []), true);
  assert.equal(draftPublishableBy(dri, ADA, [ADA]), false);
  assert.equal(draftPublishableBy(dri, null, [ME]), false);
});

test("a project draft reads like its project page", () => {
  const [{ draft }] = ownChatDrafts(
    [project("1", ME, DM, "Fix the hall")],
    AGENT,
    ME,
    new Set(),
  );
  const page = draftPageModel(draft);
  assert.equal(page.eyebrow, "Project draft");
  assert.equal(page.title, "Fix the hall");
  assert.equal(page.brief, "Fix the hall, in full.");
  assert.deepEqual(
    page.facts.map((fact) => fact.label),
    ["Holds it", "Review"],
  );
});

test("an objectives draft lists one line per outcome", () => {
  const [{ draft }] = ownChatDrafts(
    [
      direction(
        "1",
        ME,
        DM,
        "objectives",
        "Open the pantry by June. Serve two hundred families by August.",
      ),
    ],
    AGENT,
    ME,
    new Set(),
  );
  const page = draftPageModel(draft);
  assert.equal(page.eyebrow, "Objectives draft");
  assert.equal(page.title, "Objectives");
  assert.deepEqual(
    page.lines.map((line) => line.text),
    ["Open the pantry by June.", "Serve two hundred families by August."],
  );
  assert.deepEqual(page.facts, []);
});

test("a DRI draft names its project and holder", () => {
  const page = draftPageModel(
    {
      kind: "dri",
      messageId: "9",
      createdAt: 9,
      from: ME,
      itemId: ITEM,
      pubkey: ADA,
    },
    "Fix the hall",
  );
  assert.equal(page.title, "Fix the hall");
  assert.deepEqual(page.facts, [
    { label: "Holds it", kind: "person", pubkey: ADA, empty: "Not yet" },
  ]);
});
