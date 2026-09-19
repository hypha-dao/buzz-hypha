import type { DirectionSlug } from "../../commands";

import { ORG_EMPTY_NOT_SET_YET } from "../OrgEmptyState";

/** Prototype map § Direction kickers — title and the question each card answers. */
export const DIRECTION_LABEL: Record<
  DirectionSlug,
  { title: string; question: string }
> = {
  mission: { title: "Mission", question: "why we exist" },
  vision: { title: "Vision", question: "where we are going" },
  objectives: {
    title: "Objectives",
    question: "what we aim to have done soon",
  },
  strategy: { title: "Strategy", question: "how we get there" },
};

export const NOT_SET_YET = ORG_EMPTY_NOT_SET_YET;

export const RULE_KINDS = ["direction", "project", "dri", "shapers"] as const;

export type RuleKind = (typeof RULE_KINDS)[number];
