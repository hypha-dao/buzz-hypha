import type { DeclineReason } from "@/features/org/commands";

/** The five card components Design § Surfaces names. */
export type OrgCardType = "draft" | "offer" | "decision" | "done" | "review";

export type MyWorkColumn =
  | "needs_answer"
  | "you_hold"
  | "you_offered"
  | "finished";

/** A work-item card is a project (root) or a ticket (child). */
export type WorkCardKind = "project" | "ticket";

export type OrgKickerKind = "asking" | "suggesting" | "drafted" | "person";

export type OrgReceipt = {
  id: string;
  kind: "e" | "a" | "ref";
  label: string;
};

export type OrgFact = {
  label: string;
  value: string;
};

export type OrgEventLike = {
  id: string;
  pubkey: string;
  kind: number;
  content: string;
  created_at: number;
  tags: string[][];
};

export type DraftKind =
  | "project"
  | "dri"
  | "ticket"
  | "done"
  | "review"
  | "objectives"
  | "direction"
  | "profile"
  | "money";

export type OrgCardModel = {
  event: OrgEventLike;
  cardType: OrgCardType;
  column: MyWorkColumn;
  kickerKind: OrgKickerKind;
  kicker: string;
  claim: string;
  facts: OrgFact[];
  receipts: OrgReceipt[];
  needed: { agrees: number; needed: number } | null;
  draftKind: DraftKind | null;
  suggestedName: string | null;
  itemId: string | null;
  proposalId: string | null;
  needsViewer: boolean;
  fromAgent: boolean;
  /** Unix seconds. Absent when the event has no due date. */
  dueAt: number | null;
  /** Set on `39101` cards. Drafts and decisions stay null. */
  itemKind: WorkCardKind | null;
  /** Parent project id for a ticket. */
  parentId: string | null;
  /** Parent project title, when that project is in the same read. */
  parentTitle: string | null;
};

export type CardNameLookup = (pubkey: string) => string;

export type { DeclineReason };
