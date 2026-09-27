/**
 * Fixed decline-reason list (Phase 0 § Cards, by move; Protocol §4.6).
 * Chips write `["reason", <code>]` on `io_draft_decide` — do not invent codes.
 */

import type { DeclineReason } from "@/features/org/commands";

export const DECLINE_REASONS: readonly DeclineReason[] = [
  "already_covered",
  "not_what_the_line_meant",
  "too_big",
  "too_small",
  "wrong_holder",
  "not_now",
  "other",
];

export const DECLINE_REASON_LABEL: Record<DeclineReason, string> = {
  already_covered: "already covered",
  not_what_the_line_meant: "not what the line meant",
  too_big: "too big",
  too_small: "too small",
  wrong_holder: "wrong holder",
  not_now: "not now",
  other: "other",
};
