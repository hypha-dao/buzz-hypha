import type { RelayEvent } from "../../../src/shared/api/types";

export const MOCK_VIEWER = "deadbeef".repeat(8);
export const ALICE =
  "953d3363262e86b770419834c53d2446409db6d918a57f8f339d495d54ab001f";
export const GENERAL_CHANNEL_ID = "9a1657ac-f7aa-5db0-b632-d8bbeb6dfb50";

export const ROOT_ID = "11111111-1111-4111-8111-111111111111";
export const CHILD_ID = "22222222-2222-4222-8222-222222222222";
export const GRAND_ID = "33333333-3333-4333-8333-333333333333";
export const OPEN_ROOT_ID = "44444444-4444-4444-8444-444444444444";

const RELAY = "f".repeat(64);

function hexId(seed: string): string {
  return seed.replace(/-/g, "").padEnd(64, "0");
}

export function orgEvent(input: {
  id?: string;
  kind: number;
  tags?: string[][];
  content?: string | Record<string, unknown>;
  createdAt?: number;
  pubkey?: string;
}): RelayEvent {
  return {
    id: input.id ?? hexId(`e${input.kind}${input.createdAt ?? 1}`),
    pubkey: input.pubkey ?? RELAY,
    created_at: input.createdAt ?? 1_700_000_000,
    kind: input.kind,
    tags: input.tags ?? [],
    content:
      typeof input.content === "string"
        ? input.content
        : JSON.stringify(input.content ?? {}),
    sig: "s".repeat(128),
  };
}

export function workItemEvent(input: {
  id: string;
  title: string;
  brief?: string;
  state: "open" | "offered" | "accepted" | "in_review" | "done";
  type: "project" | "ticket";
  parent?: string | null;
  root?: string;
  dri?: string | null;
  offeredTo?: string | null;
  dueAt?: number | null;
  approvedAt?: number | null;
  children?: {
    open: number;
    offered: number;
    accepted: number;
    done: number;
  };
  homeChannel?: string | null;
  createdAt?: number;
}): RelayEvent {
  const root = input.root ?? input.id;
  const parent = input.parent ?? null;
  const tags: string[][] = [
    ["d", input.id],
    ["s", input.state],
    ["root", root],
    ["t", input.type],
  ];
  if (parent) tags.push(["u", parent]);
  if (input.dri) tags.push(["p", input.dri]);
  if (input.offeredTo) tags.push(["p", input.offeredTo, "", "offered"]);
  if (input.dueAt !== undefined && input.dueAt !== null) {
    tags.push(["due", String(input.dueAt)]);
  }
  return orgEvent({
    id: hexId(input.id),
    kind: 39101,
    createdAt: input.createdAt ?? 1_700_000_100,
    tags,
    content: {
      id: input.id,
      parent,
      root,
      depth: parent ? (parent === root ? 1 : 2) : 0,
      path: parent ? (parent === root ? [root] : [root, parent]) : [],
      title: input.title,
      brief: input.brief ?? "",
      state: input.state,
      dri: input.dri ?? null,
      offered_to: input.offeredTo ?? null,
      due_at: input.dueAt ?? null,
      approved_at: input.approvedAt ?? null,
      children: input.children ?? {
        open: 0,
        offered: 0,
        accepted: 0,
        done: 0,
      },
      home: input.homeChannel
        ? { channel: input.homeChannel, repo: null, project: null }
        : null,
    },
  });
}

export function depth3WorkEvents(): RelayEvent[] {
  return [
    workItemEvent({
      id: ROOT_ID,
      title: "Weekday hall",
      brief: "Book the hall for weekday evenings.\nDone looks like a rota.",
      state: "accepted",
      type: "project",
      dri: MOCK_VIEWER,
      dueAt: 1_785_000_000,
      approvedAt: 1_757_900_000,
      children: { open: 0, offered: 1, accepted: 1, done: 3 },
      homeChannel: GENERAL_CHANNEL_ID,
    }),
    workItemEvent({
      id: CHILD_ID,
      title: "Electrics",
      brief: "Wire the lighting.",
      state: "accepted",
      type: "ticket",
      parent: ROOT_ID,
      root: ROOT_ID,
      dri: MOCK_VIEWER,
      children: { open: 0, offered: 1, accepted: 0, done: 0 },
    }),
    workItemEvent({
      id: GRAND_ID,
      title: "Rota",
      brief: "Who locks up.",
      state: "offered",
      type: "ticket",
      parent: CHILD_ID,
      root: ROOT_ID,
      offeredTo: ALICE,
    }),
    workItemEvent({
      id: OPEN_ROOT_ID,
      title: "Cold storage",
      brief: "Needs a DRI.",
      state: "open",
      type: "project",
    }),
    orgEvent({
      id: hexId("trail-offer"),
      kind: 50006,
      createdAt: 1_700_000_200,
      pubkey: MOCK_VIEWER,
      tags: [
        ["i", ROOT_ID],
        ["p", MOCK_VIEWER],
      ],
      content: {},
    }),
    orgEvent({
      id: hexId("trail-accept"),
      kind: 50007,
      createdAt: 1_700_000_300,
      pubkey: MOCK_VIEWER,
      tags: [["i", ROOT_ID]],
      content: {},
    }),
    orgEvent({
      id: hexId("trail-due"),
      kind: 50011,
      createdAt: 1_700_000_400,
      pubkey: MOCK_VIEWER,
      tags: [
        ["i", ROOT_ID],
        ["due", "1785000000"],
      ],
      content: {},
    }),
    orgEvent({
      id: hexId("health-stale"),
      kind: 50101,
      createdAt: 1_700_000_010,
      tags: [
        ["i", ROOT_ID],
        ["week", "2026-W37"],
        ["band", "healthy"],
      ],
      content: {
        item: ROOT_ID,
        week: "2026-W37",
        pct: 0.9,
        band: "healthy",
        sentences: [{ text: "Last week was fine.", rows: ["old-row"] }],
      },
    }),
    orgEvent({
      id: hexId("health-fresh"),
      kind: 50101,
      createdAt: 1_700_000_500,
      tags: [
        ["i", ROOT_ID],
        ["week", "2026-W38"],
        ["band", "wobbly"],
      ],
      content: {
        item: ROOT_ID,
        week: "2026-W38",
        pct: 0.62,
        band: "wobbly",
        sentences: [
          {
            text: "Two pieces are past their date.",
            rows: ["row-overdue-1", "row-overdue-2"],
          },
        ],
      },
    }),
  ];
}
