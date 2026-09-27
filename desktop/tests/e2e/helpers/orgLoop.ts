/**
 * Stage seeds for D-6 — Playwright coverage of the loop (Phase 0 § Keeping
 * the agent honest — desktop). Each stage is a snapshot the mock bridge can
 * serve; the bridge does not settle proposals, so the loop is proved as
 * sequenced taps + signed-event asserts rather than one reactive run.
 */

import type { RelayEvent } from "../../../src/shared/api/types";
import { ORG_AGENT_PUBKEY } from "../../../src/testing/orgAgentFixture";

export const MOCK_VIEWER = "deadbeef".repeat(8);
export const ALICE =
  "953d3363262e86b770419834c53d2446409db6d918a57f8f339d495d54ab001f";

export const DIRECTION_PROPOSAL_ID = "aaaaaaaa-1111-4111-8111-111111111111";
export const DIRECTION_DECISION_ID = "c1".repeat(32);

export const PROJECT_DRAFT_ID = "c2".repeat(32);
export const PROJECT_PROPOSAL_ID = "bbbbbbbb-2222-4222-8222-222222222222";
export const PROJECT_DECISION_ID = "c3".repeat(32);

export const ROOT_ID = "11111111-1111-4111-8111-111111111111";
export const CHILD_ID = "22222222-2222-4222-8222-222222222222";

export const DRI_DRAFT_ID = "c4".repeat(32);
export const OFFER_CHILD_EVENT_ID = "c5".repeat(32);
export const ACCEPTED_CHILD_EVENT_ID = "c6".repeat(32);
export const DONE_DRAFT_ID = "c7".repeat(32);
export const RECEIPT_ID = "e1".repeat(32);

const RELAY = "f".repeat(64);

function hexId(seed: string): string {
  return seed.replace(/-/g, "").padEnd(64, "0");
}

function ev(
  id: string,
  kind: number,
  content: unknown,
  tags: string[][],
  pubkey = ORG_AGENT_PUBKEY,
  createdAt = 1_700_000_000,
): RelayEvent {
  return {
    id,
    pubkey,
    kind,
    content: JSON.stringify(content),
    created_at: createdAt,
    tags,
    sig: "mocksig".repeat(20).slice(0, 128),
  };
}

function workItem(input: {
  eventId: string;
  id: string;
  title: string;
  brief?: string;
  state: "open" | "offered" | "accepted";
  type: "project" | "ticket";
  parent?: string | null;
  dri?: string | null;
  offeredTo?: string | null;
  offeredBy?: string | null;
  children?: {
    open: number;
    offered: number;
    accepted: number;
    done: number;
  };
  createdAt?: number;
}): RelayEvent {
  const parent = input.parent ?? null;
  const root = parent ?? input.id;
  const tags: string[][] = [
    ["d", input.id],
    ["s", input.state],
    ["root", root],
    ["t", input.type],
  ];
  if (parent) tags.push(["u", parent]);
  if (input.dri) tags.push(["p", input.dri]);
  if (input.offeredTo) tags.push(["p", input.offeredTo, "", "offered"]);
  return ev(
    input.eventId,
    39101,
    {
      id: input.id,
      parent,
      root,
      depth: parent ? 1 : 0,
      path: parent ? [root] : [],
      title: input.title,
      brief: input.brief ?? "",
      state: input.state,
      dri: input.dri ?? null,
      offered_to: input.offeredTo ?? null,
      offered_by: input.offeredBy ?? null,
      due_at: null,
      approved_at: 1_757_900_000,
      children: input.children ?? {
        open: 0,
        offered: 0,
        accepted: 0,
        done: 0,
      },
      home: null,
    },
    tags,
    RELAY,
    input.createdAt ?? 1_700_000_100,
  );
}

/** Open direction proposal — Alice already agreed; viewer confirms (1 of 2). */
export function stageDirectionConfirm(): RelayEvent[] {
  return [
    ev(
      DIRECTION_DECISION_ID,
      39102,
      {
        id: DIRECTION_PROPOSAL_ID,
        kind: "direction",
        status: "open",
        needed: 2,
        eligible: [MOCK_VIEWER, ALICE],
        votes: [{ p: ALICE, vote: "agree", at: 1_700_000_010 }],
        payload: {
          slug: "objectives",
          body: "Book the weekday hall.",
          title: "Confirm objectives",
        },
      },
      [
        ["d", DIRECTION_PROPOSAL_ID],
        ["t", "direction"],
        ["s", "open"],
        ["p", MOCK_VIEWER, "", "eligible"],
        ["p", ALICE, "", "eligible"],
      ],
      RELAY,
      1_700_000_090,
    ),
  ];
}

/** Project draft the agent asks the viewer to open. */
export function stageProjectCard(): RelayEvent[] {
  return [
    ev(
      PROJECT_DRAFT_ID,
      50100,
      {
        title: "Weekday hall",
        brief: "Book the hall for weekday evenings.",
        why: "objectives line 1 is unserved",
        due_at: 1_785_000_000,
        objective_ref: "objectives@3#l_7f3a",
      },
      [
        ["n", MOCK_VIEWER],
        ["t", "project"],
        ["move", "1"],
        ["origin", "gap"],
        ["p", MOCK_VIEWER, "", "needs"],
        ["e", RECEIPT_ID, "", "receipt"],
        ["ref", "objectives@3#l_7f3a"],
      ],
      ORG_AGENT_PUBKEY,
      1_700_000_100,
    ),
  ];
}

/** Open project proposal — Alice agreed; viewer's Agree is the pass. */
export function stageProposal1of2(): RelayEvent[] {
  return [
    ev(
      PROJECT_DECISION_ID,
      39102,
      {
        id: PROJECT_PROPOSAL_ID,
        kind: "project",
        status: "open",
        needed: 2,
        eligible: [MOCK_VIEWER, ALICE],
        votes: [{ p: ALICE, vote: "agree", at: 1_700_000_050 }],
        payload: { title: "Weekday hall" },
      },
      [
        ["d", PROJECT_PROPOSAL_ID],
        ["t", "project"],
        ["s", "open"],
        ["p", MOCK_VIEWER, "", "eligible"],
        ["p", ALICE, "", "eligible"],
      ],
      RELAY,
      1_700_000_110,
    ),
  ];
}

/** Passed project as a live root on Work. */
export function stageRootOnWork(): RelayEvent[] {
  return [
    workItem({
      eventId: hexId(ROOT_ID),
      id: ROOT_ID,
      title: "Weekday hall",
      brief: "Book the hall for weekday evenings.",
      state: "open",
      type: "project",
      children: { open: 0, offered: 0, accepted: 0, done: 0 },
      createdAt: 1_700_000_120,
    }),
  ];
}

/** Viewer holds the root; open child + DRI draft — Offer to the suggested holder. */
export function stageOffer(): RelayEvent[] {
  return [
    workItem({
      eventId: hexId(`held-${ROOT_ID}`),
      id: ROOT_ID,
      title: "Weekday hall",
      brief: "Book the hall for weekday evenings.",
      state: "accepted",
      type: "project",
      dri: MOCK_VIEWER,
      children: { open: 1, offered: 0, accepted: 0, done: 0 },
      createdAt: 1_700_000_130,
    }),
    workItem({
      eventId: hexId(`open-${CHILD_ID}`),
      id: CHILD_ID,
      title: "Electrics",
      brief: "Wire the lighting.",
      state: "open",
      type: "ticket",
      parent: ROOT_ID,
      createdAt: 1_700_000_135,
    }),
    ev(
      DRI_DRAFT_ID,
      50100,
      {
        item: CHILD_ID,
        parent: ROOT_ID,
        title: "Electrics",
        why: "Alice wired the hall last year",
        suggested_dri: ALICE,
      },
      [
        ["n", MOCK_VIEWER],
        ["t", "dri"],
        ["move", "2"],
        ["origin", "gap"],
        ["i", CHILD_ID],
        ["u", ROOT_ID],
        ["p", ALICE, "", "suggested"],
        ["e", RECEIPT_ID, "", "receipt"],
      ],
      ORG_AGENT_PUBKEY,
      1_700_000_140,
    ),
  ];
}

/** Child ticket offered to the viewer — Accept. */
export function stageAccept(): RelayEvent[] {
  return [
    workItem({
      eventId: hexId(`held2-${ROOT_ID}`),
      id: ROOT_ID,
      title: "Weekday hall",
      brief: "Book the hall for weekday evenings.",
      state: "accepted",
      type: "project",
      dri: MOCK_VIEWER,
      children: { open: 0, offered: 1, accepted: 0, done: 0 },
      createdAt: 1_700_000_150,
    }),
    workItem({
      eventId: OFFER_CHILD_EVENT_ID,
      id: CHILD_ID,
      title: "Electrics",
      brief: "Wire the lighting.",
      state: "offered",
      type: "ticket",
      parent: ROOT_ID,
      offeredTo: MOCK_VIEWER,
      offeredBy: "agent",
      createdAt: 1_700_000_160,
    }),
  ];
}

/** Accepted child on Work + done card on My Work. */
export function stageChildAndDone(): RelayEvent[] {
  return [
    workItem({
      eventId: hexId(`held3-${ROOT_ID}`),
      id: ROOT_ID,
      title: "Weekday hall",
      brief: "Book the hall for weekday evenings.",
      state: "accepted",
      type: "project",
      dri: MOCK_VIEWER,
      children: { open: 0, offered: 0, accepted: 1, done: 0 },
      createdAt: 1_700_000_170,
    }),
    workItem({
      eventId: ACCEPTED_CHILD_EVENT_ID,
      id: CHILD_ID,
      title: "Electrics",
      brief: "Wire the lighting.",
      state: "accepted",
      type: "ticket",
      parent: ROOT_ID,
      dri: MOCK_VIEWER,
      createdAt: 1_700_000_180,
    }),
    ev(
      DONE_DRAFT_ID,
      50100,
      { item: CHILD_ID, why: "Lighting is wired and tested." },
      [
        ["n", MOCK_VIEWER],
        ["t", "done"],
        ["move", "3"],
        ["origin", "gap"],
        ["u", CHILD_ID],
        ["e", RECEIPT_ID, "", "receipt"],
      ],
      ORG_AGENT_PUBKEY,
      1_700_000_190,
    ),
  ];
}
