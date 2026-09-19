/**
 * Protocol §6.5 REQ filters — one set per door. Filter tags are single
 * letters. History pages are bounded (AGENTS.md review-proven rule 4).
 */

import type { RelaySubscriptionFilter } from "@/shared/api/relayClientShared";
import {
  KIND_IO_DIRECTION,
  KIND_IO_DRAFT,
  KIND_IO_HEALTH,
  KIND_IO_PROGRESS,
  KIND_IO_PROPOSAL,
  KIND_IO_SHAPERS,
  KIND_IO_WORK_ITEM,
} from "@/shared/constants/kinds";

import {
  NEEDS_SHAPER,
  SHAPERS_D_TAG,
  STATUS_OPEN,
  TAG_ITEM,
  TAG_NEEDS,
  TAG_PARENT,
  TAG_STATUS,
  TAG_TYPE,
  TYPE_PROJECT,
} from "../tags";

/** History page size for a door REQ. Live REQs use `limit: 0`. */
export const ORG_HISTORY_LIMIT = 500;

/** Every person-signed command kind — the item-page trail (§6.5). */
export const IO_COMMAND_KINDS: number[] = Array.from(
  { length: 21 },
  (_, index) => 50001 + index,
);

function history(filter: Omit<RelaySubscriptionFilter, "limit">): RelaySubscriptionFilter {
  return { ...filter, limit: ORG_HISTORY_LIMIT };
}

/** Overview: `{kinds:[39100]}`, `{kinds:[39103]}`, `{kinds:[39101], "#t":["project"]}`. */
export function overviewFilters(): RelaySubscriptionFilter[] {
  return [
    history({ kinds: [KIND_IO_DIRECTION] }),
    history({ kinds: [KIND_IO_SHAPERS], "#d": [SHAPERS_D_TAG] }),
    history({ kinds: [KIND_IO_WORK_ITEM], [`#${TAG_TYPE}`]: [TYPE_PROJECT] }),
  ];
}

/**
 * Work: `{kinds:[39101]}`, and `{kinds:[50101], "#i":[…]}` once the tree
 * has item ids. An empty `#i` is omitted — there is nothing to fetch.
 */
export function workFilters(itemIds: readonly string[] = []): RelaySubscriptionFilter[] {
  const filters = [history({ kinds: [KIND_IO_WORK_ITEM] })];
  if (itemIds.length > 0) {
    filters.push(
      history({
        kinds: [KIND_IO_HEALTH],
        [`#${TAG_ITEM}`]: itemIds.slice(0, 128),
      }),
    );
  }
  return filters;
}

/**
 * Item page: `{kinds:[39101], "#d":[id]}`, `{kinds:[39101], "#u":[id]}`,
 * `{kinds:[50001–50021], "#i":[id]}`, `{kinds:[50102], "#i":[id]}`.
 */
export function workItemFilters(itemId: string): RelaySubscriptionFilter[] {
  return [
    history({ kinds: [KIND_IO_WORK_ITEM], "#d": [itemId] }),
    history({
      kinds: [KIND_IO_WORK_ITEM],
      [`#${TAG_PARENT}`]: [itemId],
    }),
    history({
      kinds: IO_COMMAND_KINDS,
      [`#${TAG_ITEM}`]: [itemId],
    }),
    history({
      kinds: [KIND_IO_PROGRESS],
      [`#${TAG_ITEM}`]: [itemId],
    }),
  ];
}

/**
 * My Work: `{kinds:[39101], "#p":[me]}`, `{kinds:[50100], "#n":[me]}`,
 * `{kinds:[39102], "#p":[me], "#s":["open"]}`. Shapers add
 * `{kinds:[50100], "#n":["shaper"]}`. `39103` is the live read that tells
 * the hook whether to add that last filter — same event D-5 already
 * watches for `agent`.
 */
export function myWorkFilters(
  pubkey: string,
  includeShaperDrafts = false,
): RelaySubscriptionFilter[] {
  const filters = [
    history({ kinds: [KIND_IO_SHAPERS], "#d": [SHAPERS_D_TAG] }),
    history({ kinds: [KIND_IO_WORK_ITEM], "#p": [pubkey] }),
    history({ kinds: [KIND_IO_DRAFT], [`#${TAG_NEEDS}`]: [pubkey] }),
    history({
      kinds: [KIND_IO_PROPOSAL],
      "#p": [pubkey],
      [`#${TAG_STATUS}`]: [STATUS_OPEN],
    }),
  ];
  if (includeShaperDrafts) {
    filters.push(
      history({ kinds: [KIND_IO_DRAFT], [`#${TAG_NEEDS}`]: [NEEDS_SHAPER] }),
    );
  }
  return filters;
}
