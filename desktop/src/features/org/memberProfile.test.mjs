import assert from "node:assert/strict";
import test from "node:test";

import {
  describeMemberActivity,
  formatProfileWhen,
  profileHeldWork,
  PROFILE_ACTIVITY_PREVIEW,
  recentMemberActivity,
  visibleProfileActivity,
  voteSubjects,
} from "./memberProfile.ts";
import { parseWorkItem } from "./work/model.ts";

const ME = "aa".repeat(32);
const OTHER = "bb".repeat(32);

function workEvent(id, dri, state, title, createdAt = 100, type = "ticket") {
  return {
    id,
    pubkey: OTHER,
    kind: 39101,
    created_at: createdAt,
    content: JSON.stringify({
      title,
      state,
      dri,
      type,
      parent: type === "ticket" ? "parent-project" : null,
      approved_at: state === "done" ? createdAt : null,
    }),
    tags: [
      ["d", id],
      ["t", type],
      ["p", dri],
    ],
    sig: "sig",
  };
}

test("profile lists responsible projects apart from current tickets", () => {
  const held = parseWorkItem(
    workEvent("held", ME, "accepted", "Saturday stall"),
  );
  const review = parseWorkItem(
    workEvent("review", ME.toUpperCase(), "in_review", "Labels"),
  );
  const project = parseWorkItem(
    workEvent("hall", ME, "accepted", "Weekday hall", 80, "project"),
  );
  const reviewProject = parseWorkItem(
    workEvent("signs", ME, "in_review", "Signage", 70, "project"),
  );
  const finished = parseWorkItem(
    workEvent("done", ME, "done", "Last market", 50),
  );
  const finishedProject = parseWorkItem(
    workEvent("old", ME, "done", "Last season", 40, "project"),
  );
  const offered = parseWorkItem(
    workEvent("offered", ME, "offered", "Not yet", 60, "project"),
  );
  const someoneElse = parseWorkItem(
    workEvent("else", OTHER, "accepted", "Their ticket"),
  );
  assert.ok(
    held &&
      review &&
      project &&
      reviewProject &&
      finished &&
      finishedProject &&
      offered &&
      someoneElse,
  );

  const split = profileHeldWork(
    [
      held,
      review,
      project,
      reviewProject,
      finished,
      finishedProject,
      offered,
      someoneElse,
    ],
    ME,
  );
  assert.deepEqual(
    split.projects.map((item) => item.id),
    ["signs", "hall"],
  );
  assert.deepEqual(
    split.tickets.map((item) => item.id),
    ["review", "held"],
  );
  assert.deepEqual(
    split.earlier.map((item) => item.id),
    ["done", "old"],
  );
});

test("recent activity names the action and keeps the newest first", () => {
  const voted = describeMemberActivity({
    id: "vote",
    kind: 50003,
    content: "",
    created_at: 10,
    tags: [],
  });
  const note = describeMemberActivity({
    id: "note",
    kind: 50102,
    content: JSON.stringify({ text: "Stalls are up." }),
    created_at: 20,
    tags: [["i", "held"]],
  });
  assert.equal(voted?.label, "Voted");
  const named = describeMemberActivity(
    {
      id: "vote",
      kind: 50003,
      content: "",
      created_at: 10,
      tags: [
        ["e", "hall-proposal"],
        ["vote", "agree"],
      ],
    },
    voteSubjects([
      {
        id: "p1",
        kind: 39102,
        content: JSON.stringify({
          kind: "project",
          payload: { title: "Weekday hall" },
        }),
        created_at: 1,
        tags: [
          ["d", "hall-proposal"],
          ["i", "hall"],
          ["t", "project"],
        ],
      },
    ]),
  );
  assert.equal(named?.label, "Voted on Weekday hall");
  assert.equal(named?.detail, "Agreed");
  assert.equal(named?.itemId, "hall");
  assert.equal(note?.label, "Posted a progress note");
  assert.equal(note?.detail, "Stalls are up.");
  assert.equal(note?.itemId, "held");
  assert.equal(
    describeMemberActivity({
      id: "chat",
      kind: 1,
      content: "hello",
      created_at: 30,
      tags: [],
    }),
    null,
  );

  const recent = recentMemberActivity(
    [
      {
        id: "vote",
        kind: 50003,
        content: "",
        created_at: 10,
        tags: [],
      },
      {
        id: "note",
        kind: 50102,
        content: "plain note",
        created_at: 20,
        tags: [],
      },
    ],
    1,
  );
  assert.deepEqual(
    recent.map((entry) => entry.id),
    ["note"],
  );
  assert.equal(formatProfileWhen(100, 130), "just now");
  assert.equal(formatProfileWhen(100, 100 + 120), "2m ago");
});

test("profile activity shows five until the rest is opened", () => {
  const entries = [0, 1, 2, 3, 4, 5, 6];
  assert.equal(PROFILE_ACTIVITY_PREVIEW, 5);
  assert.deepEqual(visibleProfileActivity(entries, false), [0, 1, 2, 3, 4]);
  assert.deepEqual(visibleProfileActivity(entries, true), entries);
  assert.deepEqual(visibleProfileActivity([1, 2], false), [1, 2]);
});
