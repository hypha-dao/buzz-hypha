/**
 * Seeded Overview state for D-1 Playwright proofs. Free of app imports so
 * specs can load it without the desktop bundle.
 */

import { ORG_AGENT_PUBKEY } from "./orgAgentFixture.ts";

export const OVERVIEW_VIEWER_PUBKEY = "deadbeef".repeat(8);
export const OVERVIEW_CONFIRMER_PUBKEY =
  "953d3363262e86b770419834c53d2446409db6d918a57f8f339d495d54ab001f";
export const OVERVIEW_ADD_SHAPER_PUBKEY =
  "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899";
export const HALL_PROJECT_ID = "11111111-1111-4111-8111-111111111111";
export const HARVEST_PROJECT_ID = "33333333-3333-4333-8333-333333333333";
export const MISSION_PROPOSAL_ID = "aaaaaaaa-1111-4111-8111-111111111111";
export const TALLY_WEEK = "2026-W38";

type SeedEvent = {
  id: string;
  pubkey: string;
  created_at: number;
  kind: number;
  tags: string[][];
  content: string;
  sig: string;
};

function seedEvent(
  id: string,
  kind: number,
  content: unknown,
  tags: string[][],
  pubkey = "f".repeat(64),
  createdAt = 1_750_000_000,
): SeedEvent {
  return {
    id,
    pubkey,
    created_at: createdAt,
    kind,
    tags,
    content: JSON.stringify(content),
    sig: "mocksig".repeat(20).slice(0, 128),
  };
}

/** Three confirmed heads + one empty slot (strategy). */
export function createOverviewSeedEvents(): SeedEvent[] {
  return [
    seedEvent(
      "a".repeat(64),
      39100,
      {
        slug: "mission",
        version: 3,
        body: "Feed the Saturday market every week.",
        confirmed_by: OVERVIEW_CONFIRMER_PUBKEY,
        confirmed_at: 1_750_000_000,
        proposed_by: OVERVIEW_VIEWER_PUBKEY,
        proposal: MISSION_PROPOSAL_ID,
      },
      [
        ["d", "mission"],
        ["version", "3"],
        ["p", OVERVIEW_CONFIRMER_PUBKEY],
      ],
    ),
    seedEvent(
      "b".repeat(64),
      39100,
      {
        slug: "vision",
        version: 1,
        body: "A hall that stays open on weekdays.",
        confirmed_by: OVERVIEW_VIEWER_PUBKEY,
        confirmed_at: 1_740_000_000,
      },
      [
        ["d", "vision"],
        ["version", "1"],
        ["p", OVERVIEW_VIEWER_PUBKEY],
      ],
    ),
    seedEvent(
      "c".repeat(64),
      39100,
      {
        slug: "objectives",
        version: 2,
        body: "Book the weekday hall.",
        lines: [
          {
            n: 1,
            id: "l_7f3a",
            text: "Weekday hall booked",
            date: 1_780_000_000,
          },
        ],
        confirmed_by: OVERVIEW_CONFIRMER_PUBKEY,
        confirmed_at: 1_745_000_000,
      },
      [
        ["d", "objectives"],
        ["version", "2"],
        ["p", OVERVIEW_CONFIRMER_PUBKEY],
      ],
    ),
    seedEvent(
      "d".repeat(64),
      39101,
      {
        id: HALL_PROJECT_ID,
        parent: null,
        root: HALL_PROJECT_ID,
        depth: 0,
        title: "Weekday hall",
        brief: "Sign the licence.",
        state: "accepted",
        dri: OVERVIEW_CONFIRMER_PUBKEY,
      },
      [
        ["d", HALL_PROJECT_ID],
        ["t", "project"],
        ["s", "accepted"],
        ["p", OVERVIEW_CONFIRMER_PUBKEY],
      ],
    ),
    seedEvent(
      "e".repeat(64),
      39101,
      {
        id: HARVEST_PROJECT_ID,
        parent: null,
        root: HARVEST_PROJECT_ID,
        depth: 0,
        title: "Harvest",
        brief: "Needs a holder.",
        state: "open",
        dri: null,
      },
      [
        ["d", HARVEST_PROJECT_ID],
        ["t", "project"],
        ["s", "open"],
      ],
    ),
    seedEvent(
      "1".repeat(64),
      39102,
      {
        id: MISSION_PROPOSAL_ID,
        kind: "direction",
        status: "passed",
        payload: {
          slug: "mission",
          base: 2,
          body: "Feed the Saturday market every week.",
        },
        votes: [
          { p: OVERVIEW_CONFIRMER_PUBKEY, vote: "agree", at: 1_750_000_000 },
        ],
        decided_at: 1_750_000_000,
        executed: { kind: "direction", id: "mission" },
      },
      [
        ["d", MISSION_PROPOSAL_ID],
        ["t", "direction"],
        ["s", "passed"],
      ],
    ),
    seedEvent(
      "2".repeat(64),
      50103,
      {
        note: "tally",
        week: TALLY_WEEK,
        window_weeks: 4,
        moves: {
          1: {
            opened: 6,
            accepted: 3,
            amended: 1,
            declined: { already_covered: 1, not_now: 1 },
            dropped: { unresolved_receipt: 0, nag: 0 },
            shadow: 0,
          },
        },
        health: { reads: 4, rated: 8, agreed: 7 },
        open_older_than_5d: 0,
        receipt_rejected: 0,
      },
      [
        ["t", "tally"],
        ["week", TALLY_WEEK],
      ],
      ORG_AGENT_PUBKEY,
    ),
  ];
}
