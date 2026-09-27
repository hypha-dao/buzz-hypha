/**
 * Overview extras that are not in D-0's `overviewFilters` (Protocol §6.5
 * Overview is 39100 / 39103 / 39101#t=project). Tally is the plan-row read;
 * direction history is the Direction-page set. Consumed here, not pushed
 * into `hooks/filters.ts`.
 */

import type { RelaySubscriptionFilter } from "@/shared/api/relayClientShared";
import {
  KIND_IO_AGENT_NOTE,
  KIND_IO_DIRECTION,
  KIND_IO_PROPOSAL,
} from "@/shared/constants/kinds";

import type { DirectionSlug } from "../../commands";
import { ORG_HISTORY_LIMIT } from "../../hooks/filters";
import { TAG_STATUS, TAG_TYPE } from "../../tags";

function history(
  filter: Omit<RelaySubscriptionFilter, "limit">,
): RelaySubscriptionFilter {
  return { ...filter, limit: ORG_HISTORY_LIMIT };
}

/** `{kinds:[50103], "#t":["tally"]}` — Overview tally card (Shapers only). */
export function tallyFilters(): RelaySubscriptionFilter[] {
  return [
    history({ kinds: [KIND_IO_AGENT_NOTE], [`#${TAG_TYPE}`]: ["tally"] }),
  ];
}

/**
 * Protocol §6.5 Direction page — `{kinds:[39100], "#d":[slug]}` plus
 * `{kinds:[39102], "#t":["direction"], "#s":["passed"]}`.
 */
export function directionPageFilters(
  slug: DirectionSlug,
): RelaySubscriptionFilter[] {
  return [
    history({ kinds: [KIND_IO_DIRECTION], "#d": [slug] }),
    history({
      kinds: [KIND_IO_PROPOSAL],
      [`#${TAG_TYPE}`]: ["direction"],
      [`#${TAG_STATUS}`]: ["passed"],
    }),
  ];
}
