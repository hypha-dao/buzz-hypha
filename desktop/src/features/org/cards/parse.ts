/**
 * Read 50100 / 39101 / 39102 into the fields the card set needs.
 * Tag names are Protocol §4; do not invent any.
 */

import {
  KIND_IO_DRAFT,
  KIND_IO_PROPOSAL,
  KIND_IO_SHAPERS,
  KIND_IO_WORK_ITEM,
} from "@/shared/constants/kinds";
import { normalizePubkey } from "@/shared/lib/pubkey";

import {
  MARKER_RECEIPT,
  NEEDS_SHAPER,
  STATUS_OPEN,
  TAG_NEEDS,
  TAG_PARENT,
  TAG_STATUS,
  TAG_TYPE,
} from "../tags";
import {
  allTags,
  anyTag,
  asNumber,
  asString,
  firstTag,
  parseJsonObject,
} from "./tags";
import type { DraftKind, OrgEventLike, OrgReceipt } from "./types";

const DRAFT_KINDS = new Set<DraftKind>([
  "project",
  "dri",
  "ticket",
  "done",
  "review",
  "objectives",
  "direction",
  "profile",
  "money",
]);

const HEX_PREFIX = 8;

export function isShadowDraft(event: OrgEventLike): boolean {
  return event.tags.some((tag) => tag[0] === "shadow" && tag[1] === "true");
}

export function newestAddressable(
  events: readonly OrgEventLike[],
  kind: number,
): OrgEventLike[] {
  const byD = new Map<string, OrgEventLike>();
  for (const event of events) {
    if (event.kind !== kind) continue;
    const dTag = anyTag(event.tags, "d") ?? event.id;
    const existing = byD.get(dTag);
    if (!existing || event.created_at >= existing.created_at) {
      byD.set(dTag, event);
    }
  }
  return [...byD.values()];
}

export function draftKindOf(event: OrgEventLike): DraftKind | null {
  if (event.kind !== KIND_IO_DRAFT) return null;
  const value = anyTag(event.tags, TAG_TYPE);
  if (!value || !DRAFT_KINDS.has(value as DraftKind)) return null;
  return value as DraftKind;
}

export function draftNeeds(event: OrgEventLike): string | null {
  return anyTag(event.tags, TAG_NEEDS);
}

export function suggestedPubkey(event: OrgEventLike): string | null {
  const tagged = firstTag(event.tags, "p", "suggested");
  if (tagged) return normalizePubkey(tagged);
  const content = parseJsonObject(event.content);
  if (!content) return null;
  const fromContent =
    asString(content.suggested_dri) ??
    asString(content.suggested_holder) ??
    asString(content.suggested);
  return fromContent ? normalizePubkey(fromContent) : null;
}

export function workItemId(event: OrgEventLike): string | null {
  if (event.kind === KIND_IO_WORK_ITEM) {
    return (
      anyTag(event.tags, "d") ?? asString(parseJsonObject(event.content)?.id)
    );
  }
  return (
    anyTag(event.tags, "i") ??
    anyTag(event.tags, TAG_PARENT) ??
    asString(parseJsonObject(event.content)?.item) ??
    asString(parseJsonObject(event.content)?.parent)
  );
}

export function workState(event: OrgEventLike): string | null {
  return (
    anyTag(event.tags, TAG_STATUS) ??
    asString(parseJsonObject(event.content)?.state)
  );
}

export function workDri(event: OrgEventLike): string | null {
  const tagged = firstTag(event.tags, "p");
  if (tagged) return normalizePubkey(tagged);
  const content = asString(parseJsonObject(event.content)?.dri);
  return content ? normalizePubkey(content) : null;
}

export function workOfferedTo(event: OrgEventLike): string | null {
  const tagged = firstTag(event.tags, "p", "offered");
  if (tagged) return normalizePubkey(tagged);
  const content = asString(parseJsonObject(event.content)?.offered_to);
  return content ? normalizePubkey(content) : null;
}

export function workOfferedBy(event: OrgEventLike): string | null {
  const content = asString(parseJsonObject(event.content)?.offered_by);
  if (!content || content === "agent") return content;
  return normalizePubkey(content);
}

export function proposalIdOf(event: OrgEventLike): string | null {
  return anyTag(event.tags, "d");
}

export function proposalNeeded(event: OrgEventLike): {
  agrees: number;
  needed: number;
} | null {
  const content = parseJsonObject(event.content);
  if (!content) return null;
  const needed = asNumber(content.needed);
  if (needed === null) return null;
  const votes = Array.isArray(content.votes) ? content.votes : [];
  const agrees = votes.filter((vote) => {
    if (typeof vote !== "object" || vote === null) return false;
    return (vote as { vote?: unknown }).vote === "agree";
  }).length;
  return { agrees, needed };
}

export function proposalIsOpen(event: OrgEventLike): boolean {
  return (
    (anyTag(event.tags, TAG_STATUS) ??
      asString(parseJsonObject(event.content)?.status)) === STATUS_OPEN
  );
}

export function proposalEligible(event: OrgEventLike): string[] {
  const tagged = allTags(event.tags, "p", "eligible").map(normalizePubkey);
  if (tagged.length > 0) return tagged;
  const content = parseJsonObject(event.content);
  if (!Array.isArray(content?.eligible)) return [];
  return content.eligible
    .filter((value): value is string => typeof value === "string")
    .map(normalizePubkey);
}

export function proposalSubject(event: OrgEventLike): string | null {
  const tagged = firstTag(event.tags, "p", "subject");
  return tagged ? normalizePubkey(tagged) : null;
}

export function receiptsOf(event: OrgEventLike): OrgReceipt[] {
  const receipts: OrgReceipt[] = [];
  for (const tag of event.tags) {
    if (tag[0] === "e" && tag[1] && tag[3] === MARKER_RECEIPT) {
      receipts.push({
        id: tag[1],
        kind: "e",
        label: `Receipt ${tag[1].slice(0, HEX_PREFIX)}`,
      });
    } else if (tag[0] === "a" && tag[1] && tag[3] === MARKER_RECEIPT) {
      receipts.push({ id: tag[1], kind: "a", label: tag[1] });
    } else if (tag[0] === "ref" && tag[1]) {
      receipts.push({ id: tag[1], kind: "ref", label: tag[1] });
    }
  }
  return receipts;
}

export function claimOf(event: OrgEventLike): string {
  const content = parseJsonObject(event.content);
  if (event.kind === KIND_IO_DRAFT) {
    const kind = draftKindOf(event);
    const title = asString(content?.title);
    const why = asString(content?.why);
    const brief = asString(content?.brief);
    if (kind === "done") return why ?? "Mark this done?";
    if (kind === "review") return brief ?? why ?? "What next for this project?";
    if (kind === "dri") return why ?? "A holder for this work";
    if (kind === "profile") return asString(content?.about) ?? "About & skills";
    if (kind === "direction" || kind === "objectives") {
      return why ?? asString(content?.diff) ?? "A direction change";
    }
    return title ?? why ?? brief ?? "A draft";
  }
  if (event.kind === KIND_IO_WORK_ITEM) {
    return asString(content?.title) ?? "A piece of work";
  }
  if (event.kind === KIND_IO_PROPOSAL) {
    const payload =
      content?.payload && typeof content.payload === "object"
        ? (content.payload as Record<string, unknown>)
        : null;
    return (
      asString(payload?.title) ??
      asString(payload?.body) ??
      asString(content?.kind) ??
      "A decision"
    );
  }
  return "Needs your answer";
}

export function viewerIsNamedShaper(
  events: readonly OrgEventLike[],
  pubkey: string,
): boolean {
  const newest = newestAddressable(events, KIND_IO_SHAPERS)[0];
  if (!newest) return false;
  const content = parseJsonObject(newest.content);
  if (!Array.isArray(content?.shapers)) return false;
  const me = normalizePubkey(pubkey);
  return content.shapers.some(
    (entry) => typeof entry === "string" && normalizePubkey(entry) === me,
  );
}

export function needsViewer(
  event: OrgEventLike,
  viewer: string,
  isShaper: boolean,
): boolean {
  const needs = draftNeeds(event);
  if (!needs) return false;
  if (needs === NEEDS_SHAPER) return isShaper;
  return normalizePubkey(needs) === normalizePubkey(viewer);
}

export function isOrgInboxKind(kind: number): boolean {
  return (
    kind === KIND_IO_DRAFT ||
    kind === KIND_IO_WORK_ITEM ||
    kind === KIND_IO_PROPOSAL
  );
}
