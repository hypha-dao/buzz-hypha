import assert from "node:assert/strict";
import test from "node:test";

import {
  assembleWorkDoor,
  canChangeTicketDue,
  childrenOf,
  formatChildrenCounts,
  formatReviewDate,
  homeChannel,
  itemById,
  latestHealth,
  parseWorkItem,
  stateChipLabel,
  trailForItem,
  workBoardColumn,
} from "./model.ts";

const ROOT = "11111111-1111-4111-8111-111111111111";
const CHILD = "22222222-2222-4222-8222-222222222222";
const GRAND = "33333333-3333-4333-8333-333333333333";
const RELAY = "f".repeat(64);
const HOLDER = "deadbeef".repeat(8);

function event(input) {
  return {
    id: input.id ?? "a".repeat(64),
    pubkey: input.pubkey ?? RELAY,
    created_at: input.created_at ?? 100,
    kind: input.kind,
    tags: input.tags ?? [],
    content: input.content ?? "{}",
    sig: "s".repeat(128),
  };
}

function itemEvent(id, content, tags, createdAt = 100) {
  return event({
    id: id.replace(/-/g, "").padEnd(64, "0"),
    kind: 39101,
    created_at: createdAt,
    tags: [["d", id], ...tags],
    content: JSON.stringify(content),
  });
}

const rootEvent = itemEvent(
  ROOT,
  {
    id: ROOT,
    parent: null,
    root: ROOT,
    depth: 0,
    path: [],
    title: "Weekday hall",
    brief: "The hall.",
    state: "accepted",
    dri: HOLDER,
    offered_to: null,
    due_at: 1785000000,
    approved_at: 1757900000,
    children: { open: 0, offered: 1, accepted: 1, done: 3 },
    home: { channel: "9a1657ac-f7aa-5db0-b632-d8bbeb6dfb50" },
  },
  [
    ["s", "accepted"],
    ["root", ROOT],
    ["t", "project"],
    ["p", HOLDER],
  ],
);

const childEvent = itemEvent(
  CHILD,
  {
    id: CHILD,
    parent: ROOT,
    root: ROOT,
    depth: 1,
    path: [ROOT],
    title: "Electrics",
    brief: "Wire it.",
    state: "accepted",
    dri: HOLDER,
    offered_to: null,
    children: { open: 0, offered: 1, accepted: 0, done: 0 },
  },
  [
    ["s", "accepted"],
    ["root", ROOT],
    ["u", ROOT],
    ["t", "ticket"],
    ["p", HOLDER],
  ],
);

const grandEvent = itemEvent(
  GRAND,
  {
    id: GRAND,
    parent: CHILD,
    root: ROOT,
    depth: 2,
    path: [ROOT, CHILD],
    title: "Rota",
    brief: "Who locks up.",
    state: "offered",
    dri: null,
    offered_to: "a".repeat(64),
    children: { open: 0, offered: 0, accepted: 0, done: 0 },
  },
  [
    ["s", "offered"],
    ["root", ROOT],
    ["u", CHILD],
    ["t", "ticket"],
    ["p", "a".repeat(64), "", "offered"],
  ],
);

test("assembleWorkDoor lists projects and leaves tickets off the door", () => {
  const loose = "77777777-7777-4777-8777-777777777777";
  const looseTicket = itemEvent(
    loose,
    {
      id: loose,
      parent: null,
      root: loose,
      title: "Loose ticket",
      state: "open",
    },
    [
      ["s", "open"],
      ["t", "ticket"],
    ],
  );
  const tree = assembleWorkDoor([
    rootEvent,
    childEvent,
    grandEvent,
    looseTicket,
  ]);
  assert.equal(tree.length, 1);
  assert.equal(tree[0].item.title, "Weekday hall");
  assert.equal(tree[0].item.type, "project");
  assert.deepEqual(tree[0].children, []);
  assert.equal(
    tree.some((row) => row.item.id === CHILD || row.item.id === GRAND),
    false,
  );
  assert.equal(
    tree.some((row) => row.item.id === loose),
    false,
  );
});

test("withdrawn work leaves the live board", () => {
  const gone = "55555555-5555-4555-8555-555555555555";
  const withdrawnChild = itemEvent(
    gone,
    {
      id: gone,
      parent: ROOT,
      root: ROOT,
      title: "Old tiles",
      state: "withdrawn",
    },
    [
      ["s", "withdrawn"],
      ["u", ROOT],
      ["t", "ticket"],
    ],
  );
  const withdrawnRoot = itemEvent(
    "66666666-6666-4666-8666-666666666666",
    {
      id: "66666666-6666-4666-8666-666666666666",
      parent: null,
      root: "66666666-6666-4666-8666-666666666666",
      title: "Duplicate hall",
      state: "withdrawn",
    },
    [
      ["s", "withdrawn"],
      ["t", "project"],
    ],
  );
  const tree = assembleWorkDoor([
    rootEvent,
    childEvent,
    withdrawnChild,
    withdrawnRoot,
  ]);
  assert.deepEqual(
    tree.map((row) => row.item.title),
    ["Weekday hall"],
  );
  assert.deepEqual(tree[0].children, []);
  assert.equal(
    tree.some((row) => row.item.title === "Electrics"),
    false,
  );
  assert.equal(childrenOf([withdrawnChild], ROOT).length, 0);
  assert.equal(
    stateChipLabel({ state: "withdrawn", type: "ticket" }),
    "removed",
  );
});

test("childrenOf on the child page is the grandchild — deeper than the door", () => {
  const kids = childrenOf([rootEvent, childEvent, grandEvent], CHILD);
  assert.deepEqual(
    kids.map((row) => row.title),
    ["Rota"],
  );
});

test("a rewritten parent 39101 is the children-counter source", () => {
  const older = itemEvent(
    ROOT,
    {
      ...JSON.parse(rootEvent.content),
      children: { open: 9, offered: 9, accepted: 9, done: 9 },
    },
    rootEvent.tags,
    50,
  );
  const item = itemById([older, rootEvent], ROOT);
  assert.deepEqual(item?.children, {
    open: 0,
    offered: 1,
    accepted: 1,
    done: 3,
  });
  assert.equal(
    formatChildrenCounts(item.children),
    "0 open · 1 offered · 1 accepted · 3 done",
  );
});

test("homeChannel is absent until 39101.home.channel is set", () => {
  const withHome = parseWorkItem(rootEvent);
  const without = parseWorkItem(
    itemEvent(
      "44444444-4444-4444-8444-444444444444",
      {
        id: "44444444-4444-4444-8444-444444444444",
        parent: null,
        root: "44444444-4444-4444-8444-444444444444",
        title: "No home",
        state: "open",
      },
      [
        ["s", "open"],
        ["t", "project"],
      ],
    ),
  );
  assert.equal(homeChannel(withHome), "9a1657ac-f7aa-5db0-b632-d8bbeb6dfb50");
  assert.equal(homeChannel(without), null);
});

test("state chips follow the prototype grammar", () => {
  assert.equal(
    stateChipLabel({ state: "accepted", type: "ticket" }),
    "in progress",
  );
  assert.equal(
    stateChipLabel({ state: "offered", type: "ticket" }),
    "waiting on a yes",
  );
  assert.equal(
    stateChipLabel({ state: "open", type: "project" }),
    "needs a DRI",
  );
  assert.equal(stateChipLabel({ state: "open", type: "ticket" }), "open");
  assert.equal(stateChipLabel({ state: "done", type: "ticket" }), "done");
});

test("trailForItem is #i commands newest first", () => {
  const older = event({
    id: "b".repeat(64),
    kind: 50007,
    created_at: 10,
    tags: [["i", ROOT]],
  });
  const newer = event({
    id: "c".repeat(64),
    kind: 50009,
    created_at: 20,
    tags: [["i", ROOT]],
  });
  const other = event({
    id: "d".repeat(64),
    kind: 50009,
    created_at: 30,
    tags: [["i", CHILD]],
  });
  const trail = trailForItem([older, newer, other], ROOT);
  assert.deepEqual(
    trail.map((row) => row.kind),
    [50009, 50007],
  );
  assert.equal(trail[0].label, "io_done");
});

test("latestHealth is the newest 50101 for that item", () => {
  const stale = event({
    id: "e".repeat(64),
    kind: 50101,
    created_at: 1,
    tags: [
      ["i", ROOT],
      ["band", "healthy"],
    ],
    content: JSON.stringify({
      item: ROOT,
      band: "healthy",
      pct: 0.9,
      sentences: [{ text: "old", rows: ["aa"] }],
    }),
  });
  const fresh = event({
    id: "f".repeat(64),
    kind: 50101,
    created_at: 2,
    tags: [
      ["i", ROOT],
      ["band", "wobbly"],
    ],
    content: JSON.stringify({
      item: ROOT,
      week: "2026-W38",
      band: "wobbly",
      pct: 0.62,
      sentences: [
        { text: "Two pieces are past their date.", rows: ["row-1", "row-2"] },
      ],
    }),
  });
  const health = latestHealth([stale, fresh], ROOT);
  assert.equal(health?.band, "wobbly");
  assert.equal(health?.sentences[0]?.rows[1], "row-2");
});

test("review dates read as day month year", () => {
  assert.equal(formatReviewDate(1_785_000_000), "25 Jul 2026");
});

test("a ticket due date moves for the holder and the creator", () => {
  const held = parseWorkItem(childEvent);
  assert.equal(held?.type, "ticket");
  assert.equal(canChangeTicketDue(held, HOLDER), true);
  assert.equal(canChangeTicketDue(held, "c".repeat(64)), false);
  assert.equal(canChangeTicketDue(parseWorkItem(rootEvent), HOLDER), false);

  const creator = "c".repeat(64);
  const offered = parseWorkItem(
    itemEvent(
      CHILD,
      {
        id: CHILD,
        parent: ROOT,
        root: ROOT,
        depth: 1,
        path: [ROOT],
        title: "Electrics",
        brief: "Wire it.",
        state: "offered",
        dri: null,
        created_by: creator,
        children: { open: 0, offered: 0, accepted: 0, done: 0 },
      },
      [
        ["s", "offered"],
        ["root", ROOT],
        ["u", ROOT],
        ["t", "ticket"],
      ],
    ),
  );
  assert.equal(canChangeTicketDue(offered, creator), true);
  assert.equal(canChangeTicketDue(offered, HOLDER), false);
  assert.ok(held);
  assert.equal(canChangeTicketDue({ ...held, state: "done" }, HOLDER), false);
});

test("open and offered sit in not-accepted; held work is ongoing", () => {
  assert.equal(workBoardColumn({ state: "open" }), "waiting");
  assert.equal(workBoardColumn({ state: "offered" }), "waiting");
  assert.equal(workBoardColumn({ state: "accepted" }), "ongoing");
  assert.equal(workBoardColumn({ state: "in_review" }), "ongoing");
  assert.equal(workBoardColumn({ state: "done" }), "ongoing");
});
