import type { RelayEvent } from "../../src/shared/api/types";
import { ORG_AGENT_PUBKEY } from "../../src/testing/orgAgentFixture";

/** Mock-bridge viewer (`DEFAULT_MOCK_IDENTITY`). */
export const MOCK_VIEWER = "deadbeef".repeat(8);
export const ALICE_PUBKEY =
  "953d3363262e86b770419834c53d2446409db6d918a57f8f339d495d54ab001f";

export const ASKING_ID = "a1".repeat(32);
export const SUGGEST_ID = "a2".repeat(32);
export const DRAFTED_ID = "a3".repeat(32);
export const OFFER_EVENT_ID = "b1".repeat(32);
export const HELD_EVENT_ID = "b2".repeat(32);
export const OFFERED_EVENT_ID = "b3".repeat(32);
export const DECISION_ID = "d1".repeat(32);
export const DONE_ID = "d2".repeat(32);
export const REVIEW_ID = "d3".repeat(32);
export const RECEIPT_ID = "e1".repeat(32);

export const ITEM_OFFER = "11111111-1111-4111-8111-111111111111";
export const ITEM_HELD = "22222222-2222-4222-8222-222222222222";
export const ITEM_OFFERED = "33333333-3333-4333-8333-333333333333";
export const ITEM_DONE = "44444444-4444-4444-8444-444444444444";
export const ITEM_REVIEW = "55555555-5555-4555-8555-555555555555";
export const PROPOSAL_ID = "66666666-6666-4666-8666-666666666666";

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

/** Seeded 50100 / 39101 / 39102 covering every card type and column. */
export function myWorkSeedEvents(): RelayEvent[] {
  return [
    ev(
      ASKING_ID,
      50100,
      {
        title: "Book the weekday hall",
        brief: "The hall is the first gap.",
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
      1_700_000_090,
    ),
    ev(
      SUGGEST_ID,
      50100,
      {
        title: "Saturday stall",
        brief: "A stall every Saturday.",
        suggested_dri: ALICE_PUBKEY,
        why: "alice hosted last year",
        due_at: 1_786_000_000,
      },
      [
        ["n", "shaper"],
        ["t", "project"],
        ["move", "1"],
        ["origin", "gap"],
        ["p", ALICE_PUBKEY, "", "suggested"],
        ["e", RECEIPT_ID, "", "receipt"],
      ],
      ORG_AGENT_PUBKEY,
      1_700_000_080,
    ),
    ev(
      DRAFTED_ID,
      50100,
      {
        title: "Cash teach-in",
        brief: "A session on the till.",
        why: "covers the cash brief",
        due_at: 1_787_000_000,
      },
      [
        ["n", "shaper"],
        ["t", "project"],
        ["move", "1"],
        ["origin", "gap"],
        ["e", RECEIPT_ID, "", "receipt"],
      ],
      ORG_AGENT_PUBKEY,
      1_700_000_070,
    ),
    ev(
      OFFER_EVENT_ID,
      39101,
      {
        id: ITEM_OFFER,
        title: "Rota this month",
        state: "offered",
        offered_to: MOCK_VIEWER,
        offered_by: "agent",
      },
      [
        ["d", ITEM_OFFER],
        ["s", "offered"],
        ["t", "ticket"],
        ["p", MOCK_VIEWER, "", "offered"],
      ],
      "f".repeat(64),
      1_700_000_060,
    ),
    ev(
      DECISION_ID,
      39102,
      {
        id: PROPOSAL_ID,
        kind: "project",
        status: "open",
        needed: 2,
        eligible: [MOCK_VIEWER, ALICE_PUBKEY],
        votes: [{ p: ALICE_PUBKEY, vote: "agree", at: 1_700_000_050 }],
        payload: { title: "Confirm the hall project" },
      },
      [
        ["d", PROPOSAL_ID],
        ["t", "project"],
        ["s", "open"],
        ["p", MOCK_VIEWER, "", "eligible"],
        ["p", ALICE_PUBKEY, "", "eligible"],
      ],
      "f".repeat(64),
      1_700_000_050,
    ),
    ev(
      DONE_ID,
      50100,
      { item: ITEM_DONE, why: "last child closed" },
      [
        ["n", MOCK_VIEWER],
        ["t", "done"],
        ["move", "3"],
        ["origin", "gap"],
        ["u", ITEM_DONE],
        ["e", RECEIPT_ID, "", "receipt"],
      ],
      ORG_AGENT_PUBKEY,
      1_700_000_040,
    ),
    ev(
      REVIEW_ID,
      50100,
      {
        item: ITEM_REVIEW,
        brief: [{ text: "The stall ran.", rows: [RECEIPT_ID] }],
        recommendation: {
          type: "follow_up",
          project: {
            title: "Winter stall",
            brief: "Keep it going.",
            due_at: 1_790_000_000,
          },
        },
      },
      [
        ["n", "shaper"],
        ["t", "review"],
        ["move", "3"],
        ["origin", "gap"],
        ["i", ITEM_REVIEW],
        ["e", RECEIPT_ID, "", "receipt"],
      ],
      ORG_AGENT_PUBKEY,
      1_700_000_030,
    ),
    ev(
      HELD_EVENT_ID,
      39101,
      {
        id: ITEM_HELD,
        title: "Weekday hall",
        state: "accepted",
        dri: MOCK_VIEWER,
      },
      [
        ["d", ITEM_HELD],
        ["s", "accepted"],
        ["t", "project"],
        ["p", MOCK_VIEWER],
      ],
      "f".repeat(64),
      1_700_000_020,
    ),
    ev(
      OFFERED_EVENT_ID,
      39101,
      {
        id: ITEM_OFFERED,
        title: "Flyer run",
        state: "offered",
        offered_by: MOCK_VIEWER,
        offered_to: ALICE_PUBKEY,
      },
      [
        ["d", ITEM_OFFERED],
        ["s", "offered"],
        ["t", "ticket"],
        ["p", ALICE_PUBKEY, "", "offered"],
      ],
      MOCK_VIEWER,
      1_700_000_010,
    ),
  ];
}
