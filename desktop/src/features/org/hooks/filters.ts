/**
 * Protocol §6.5 REQ filters — one set per door. Filter tags are single
 * letters. History pages are bounded (AGENTS.md review-proven rule 4).
 */

import type { RelaySubscriptionFilter } from "@/shared/api/relayClientShared";
import {
  KIND_IO_DIRECTION,
  KIND_IO_DRAFT,
  KIND_IO_HEALTH,
  KIND_IO_PROFILE,
  KIND_IO_PROGRESS,
  KIND_IO_PROPOSAL,
  KIND_IO_SHAPERS,
  KIND_IO_WORK_ITEM,
  KIND_STREAM_MESSAGE,
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
  { length: 23 },
  (_, index) => 50001 + index,
);

function history(
  filter: Omit<RelaySubscriptionFilter, "limit">,
): RelaySubscriptionFilter {
  return { ...filter, limit: ORG_HISTORY_LIMIT };
}

/** Overview: direction, shapers, project roots, and profiles for the context line. */
export function overviewFilters(): RelaySubscriptionFilter[] {
  return [
    history({ kinds: [KIND_IO_DIRECTION] }),
    history({ kinds: [KIND_IO_SHAPERS], "#d": [SHAPERS_D_TAG] }),
    history({ kinds: [KIND_IO_WORK_ITEM], [`#${TAG_TYPE}`]: [TYPE_PROJECT] }),
    history({ kinds: [KIND_IO_PROFILE] }),
  ];
}

/**
 * Work: `{kinds:[39101]}`, and `{kinds:[50101], "#i":[…]}` once the tree
 * has item ids. An empty `#i` is omitted — there is nothing to fetch.
 */
export function workFilters(
  itemIds: readonly string[] = [],
): RelaySubscriptionFilter[] {
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
 * `{kinds:[50001–50023], "#i":[id]}`, `{kinds:[50102], "#i":[id]}`.
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
 * `{kinds:[39102], "#p":[me], "#s":["open"]}`, and
 * `{kinds:[39102], "#t":["project"], "#p":[me], "#s":["passed"]}` so a
 * project this person already agreed to can be recorded as held.
 * Shapers add `{kinds:[50100], "#n":["shaper"]}` and
 * `{kinds:[39101], "#t":["project"], "#s":["open"]}` — a root that still
 * needs a DRI (Journey 2.5). `39103` is the live read that tells the
 * hook whether to add those filters.
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
    history({
      kinds: [KIND_IO_PROPOSAL],
      "#p": [pubkey],
      [`#${TAG_TYPE}`]: [TYPE_PROJECT],
      [`#${TAG_STATUS}`]: ["passed"],
    }),
  ];
  if (includeShaperDrafts) {
    filters.push(
      history({ kinds: [KIND_IO_DRAFT], [`#${TAG_NEEDS}`]: [NEEDS_SHAPER] }),
      history({
        kinds: [KIND_IO_WORK_ITEM],
        [`#${TAG_TYPE}`]: [TYPE_PROJECT],
        [`#${TAG_STATUS}`]: [STATUS_OPEN],
      }),
    );
  }
  return filters;
}

/**
 * Your drafts: the org agent's chat lines in every room you can read
 * (`{kinds:[9], authors:[agent]}`). The relay scopes a REQ without `#h` to
 * your channels and DMs; drafts are the lines that carry a draft tag.
 */
export function myDraftFilters(
  orgAgentPubkey: string,
): RelaySubscriptionFilter[] {
  return [history({ kinds: [KIND_STREAM_MESSAGE], authors: [orgAgentPubkey] })];
}

/**
 * Profile — About & skills only (D-4): `{kinds:[39105], "#d":[pubkey]}`.
 * What you hold / Recent decisions (`39101` / `39102` `#p`) wait on D-3.
 */
export function profileFilters(pubkey: string): RelaySubscriptionFilter[] {
  return [history({ kinds: [KIND_IO_PROFILE], "#d": [pubkey] })];
}

/**
 * Recent org actions this person signed, plus proposals so a vote can name
 * what it was for. Bounded by the door history page.
 */
export function memberActivityFilters(
  pubkey: string,
): RelaySubscriptionFilter[] {
  return [
    history({ kinds: IO_COMMAND_KINDS, authors: [pubkey] }),
    history({ kinds: [KIND_IO_PROGRESS], authors: [pubkey] }),
    history({ kinds: [KIND_IO_PROPOSAL] }),
  ];
}
