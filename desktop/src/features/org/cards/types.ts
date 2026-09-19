import type { DeclineReason } from "@/features/org/commands";

/** The five card components Design § Surfaces names. */
export type OrgCardType = "draft" | "offer" | "decision" | "done" | "review";

export type MyWorkColumn = "needs_answer" | "you_hold" | "you_offered";

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
};

export type CardNameLookup = (pubkey: string) => string;

export type { DeclineReason };
