/**
 * A project the viewer already agreed to should not ask them to Accept
 * again on My Work.
 *
 * The relay now creates that root as held when the suggested holder
 * agreed on the proposal. Offers written before that still need one
 * `io_accept`, which is the same yes they already gave in chat.
 */

import { KIND_IO_PROPOSAL, KIND_IO_WORK_ITEM } from "@/shared/constants/kinds";
import { normalizePubkey } from "@/shared/lib/pubkey";

import {
  newestAddressable,
  workItemId,
  workKindOf,
  workOfferedTo,
  workState,
} from "./cards/parse";
import { anyTag, parseJsonObject } from "./cards/tags";
import type { OrgEventLike } from "./cards/types";
import { TAG_STATUS } from "./tags";

const attempted = new Set<string>();
const attemptOrder: string[] = [];
const MAX_ATTEMPTS = 200;

/** Drop claims when the community changes. Item ids are community-scoped. */
export function resetAgreedOfferAttempts(): void {
  attempted.clear();
  attemptOrder.length = 0;
}

/** One automatic accept per item. A failed publish stays claimed so a live refresh cannot loop. */
export function claimAgreedOffer(itemId: string): boolean {
  if (attempted.has(itemId)) return false;
  attempted.add(itemId);
  attemptOrder.push(itemId);
  if (attemptOrder.length > MAX_ATTEMPTS) {
    const dropped = attemptOrder.shift();
    if (dropped) attempted.delete(dropped);
  }
  return true;
}

function statusOf(event: OrgEventLike): string | null {
  const tagged = anyTag(event.tags, TAG_STATUS);
  if (tagged) return tagged;
  const status = parseJsonObject(event.content)?.status;
  return typeof status === "string" ? status : null;
}

function agreedItemId(event: OrgEventLike, viewer: string): string | null {
  if (event.kind !== KIND_IO_PROPOSAL) return null;
  if (statusOf(event) !== "passed") return null;
  const content = parseJsonObject(event.content);
  if (!content) return null;
  if (content.kind !== "project") return null;
  const executed = content.executed;
  if (!executed || typeof executed !== "object" || Array.isArray(executed)) {
    return null;
  }
  const itemId = (executed as { id?: unknown }).id;
  if (typeof itemId !== "string" || itemId.length === 0) return null;
  const votes = Array.isArray(content.votes) ? content.votes : [];
  const agreed = votes.some((vote) => {
    if (!vote || typeof vote !== "object") return false;
    const row = vote as { p?: unknown; vote?: unknown };
    return (
      row.vote === "agree" &&
      typeof row.p === "string" &&
      normalizePubkey(row.p) === viewer
    );
  });
  return agreed ? itemId : null;
}

/**
 * Offered projects whose suggested holder is the viewer and whose
 * passing proposal already carries their agree.
 */
export function agreedOfferItemIds(
  events: readonly OrgEventLike[],
  viewer: string,
): string[] {
  const me = normalizePubkey(viewer);
  if (!me) return [];
  const agreed = new Set<string>();
  for (const event of newestAddressable(events, KIND_IO_PROPOSAL)) {
    const itemId = agreedItemId(event, me);
    if (itemId) agreed.add(itemId);
  }
  if (agreed.size === 0) return [];
  const ids: string[] = [];
  for (const event of newestAddressable(events, KIND_IO_WORK_ITEM)) {
    if (workKindOf(event) !== "project") continue;
    if (workState(event) !== "offered") continue;
    if (workOfferedTo(event) !== me) continue;
    const itemId = workItemId(event);
    if (itemId && agreed.has(itemId)) ids.push(itemId);
  }
  return ids;
}
