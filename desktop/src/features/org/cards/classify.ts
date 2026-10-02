/**
 * Column + card-type + kicker for My Work (Phase 0 § My Work;
 * Design § Surfaces cards; Prototype map § Cards).
 */

import {
  KIND_IO_DRAFT,
  KIND_IO_PROPOSAL,
  KIND_IO_WORK_ITEM,
} from "@/shared/constants/kinds";
import { normalizePubkey } from "@/shared/lib/pubkey";

import { parseOrgAgentFromShapers } from "../orgAgent";
import type { RelayEvent } from "@/shared/api/types";
import {
  claimOf,
  draftKindOf,
  draftNeeds,
  dueAtOf,
  isShadowDraft,
  needsViewer,
  newestAddressable,
  proposalEligible,
  proposalIdOf,
  proposalIsOpen,
  proposalNeeded,
  proposalSubject,
  receiptsOf,
  suggestedPubkey,
  viewerIsNamedShaper,
  workDri,
  workItemId,
  workKindOf,
  workOfferedBy,
  workOfferedTo,
  workParentId,
  workState,
} from "./parse";
import { anyTag } from "./tags";
import type {
  CardNameLookup,
  MyWorkColumn,
  OrgCardModel,
  OrgCardType,
  OrgEventLike,
  OrgKickerKind,
} from "./types";

export type ClassifyContext = {
  viewer: string;
  agentPubkey: string | null;
  isShaper: boolean;
  nameOf: CardNameLookup;
  /**
   * A member's display name, or null when this pubkey has no profile.
   * Keeps a relay-signed work item from reading as "asked by npub…".
   */
  displayNameOf?: (pubkey: string) => string | null;
};

const NO_DUE = Number.POSITIVE_INFINITY;

export function kickerFor(
  event: OrgEventLike,
  ctx: ClassifyContext,
  suggested: string | null,
): { kind: OrgKickerKind; text: string; suggestedName: string | null } {
  const fromAgent =
    ctx.agentPubkey !== null &&
    normalizePubkey(event.pubkey) === normalizePubkey(ctx.agentPubkey);
  const asking =
    event.kind === KIND_IO_DRAFT &&
    draftNeeds(event) !== null &&
    normalizePubkey(draftNeeds(event) ?? "") === normalizePubkey(ctx.viewer);
  const suggestedName = suggested ? ctx.nameOf(suggested) : null;

  if (fromAgent && suggested && suggested !== normalizePubkey(ctx.viewer)) {
    return {
      kind: "suggesting",
      text: `AI is suggesting for ${suggestedName}`,
      suggestedName,
    };
  }
  if (fromAgent && asking) {
    return { kind: "asking", text: "AI is asking you", suggestedName };
  }
  if (fromAgent) {
    return { kind: "drafted", text: "Drafted by the agent", suggestedName };
  }
  const known = ctx.displayNameOf?.(event.pubkey) ?? null;
  if (known) {
    return {
      kind: "person",
      text: `${known} is asking you`,
      suggestedName,
    };
  }
  return { kind: "person", text: "Needs your answer", suggestedName };
}

/**
 * Who asked for this piece. `offered_by` while an offer is open; the org
 * agent when it authored the event; a profile name when the author is a
 * member. Relay-signed state with no offerer stays unnamed.
 */
function askerLabel(event: OrgEventLike, ctx: ClassifyContext): string | null {
  const offeredBy = workOfferedBy(event);
  if (offeredBy === "agent") return "the agent";
  if (offeredBy) {
    if (normalizePubkey(offeredBy) === normalizePubkey(ctx.viewer)) return null;
    return ctx.nameOf(offeredBy);
  }
  if (
    ctx.agentPubkey !== null &&
    normalizePubkey(event.pubkey) === normalizePubkey(ctx.agentPubkey)
  ) {
    return "the agent";
  }
  return ctx.displayNameOf?.(event.pubkey) ?? null;
}

function workItemKicker(
  event: OrgEventLike,
  ctx: ClassifyContext,
  column: MyWorkColumn,
): { kind: OrgKickerKind; text: string } {
  const asker = askerLabel(event, ctx);
  const me = normalizePubkey(ctx.viewer);
  if (column === "needs_answer" && projectNeedsDri(event)) {
    return { kind: "person", text: "Needs a DRI" };
  }
  if (column === "needs_answer") {
    if (asker === "the agent") {
      return { kind: "asking", text: "AI is asking you" };
    }
    if (asker) return { kind: "person", text: `${asker} is asking you` };
    return { kind: "person", text: "Needs your answer" };
  }
  if (column === "you_offered") {
    const waiting = workOfferedTo(event);
    const holder = workDri(event);
    if (workState(event) === "offered" && waiting && waiting !== me) {
      return { kind: "person", text: `Waiting on ${ctx.nameOf(waiting)}` };
    }
    if (holder && holder !== me) {
      return { kind: "person", text: `${ctx.nameOf(holder)} holds it` };
    }
  }
  if (asker === "the agent") {
    return { kind: "drafted", text: "Asked by the agent" };
  }
  if (asker) return { kind: "person", text: `Asked by ${asker}` };
  return { kind: "person", text: "" };
}

/**
 * A live project with nobody named. Journey 2.5: it sits on every
 * Shaper's Needs your answer until someone is offered or named.
 */
function projectNeedsDri(event: OrgEventLike): boolean {
  return (
    workState(event) === "open" &&
    workKindOf(event) === "project" &&
    workDri(event) === null &&
    workOfferedTo(event) === null
  );
}

function placeWorkItem(
  event: OrgEventLike,
  me: string,
  isShaper: boolean,
): MyWorkColumn | null {
  if (workState(event) === "withdrawn") return null;
  const offeredTo = workOfferedTo(event);
  const dri = workDri(event);
  const offeredBy = workOfferedBy(event);
  if (offeredTo === me) return "needs_answer";
  if (dri === me) return "you_hold";
  if (offeredBy === me || offeredBy === "agent") return "you_offered";
  if (isShaper && projectNeedsDri(event)) return "needs_answer";
  return null;
}

/** Soonest due date first. Undated cards follow, newest first. */
export function compareByDue(left: OrgCardModel, right: OrgCardModel): number {
  const leftDue = left.dueAt ?? NO_DUE;
  const rightDue = right.dueAt ?? NO_DUE;
  if (leftDue !== rightDue) return leftDue - rightDue;
  return right.event.created_at - left.event.created_at;
}

function draftCardType(event: OrgEventLike): OrgCardType {
  const kind = draftKindOf(event);
  if (kind === "done") return "done";
  if (kind === "review") return "review";
  if (kind === "dri") return "offer";
  return "draft";
}

function factsFor(
  event: OrgEventLike,
  ctx: ClassifyContext,
  suggested: string | null,
): OrgCardModel["facts"] {
  const facts: OrgCardModel["facts"] = [];
  const kind = draftKindOf(event);
  if (kind) {
    facts.push({ label: "Kind", value: kind });
  }
  if (suggested) {
    facts.push({ label: "Suggested", value: ctx.nameOf(suggested) });
  }
  return facts;
}

export function classifyEvent(
  event: OrgEventLike,
  ctx: ClassifyContext,
): OrgCardModel | null {
  if (isShadowDraft(event)) return null;
  const me = normalizePubkey(ctx.viewer);
  const suggested = suggestedPubkey(event);
  const kicker = kickerFor(event, ctx, suggested);
  const receipts = receiptsOf(event);
  const needed = event.kind === KIND_IO_PROPOSAL ? proposalNeeded(event) : null;
  const base = {
    event,
    kickerKind: kicker.kind,
    kicker: kicker.text,
    claim: claimOf(event),
    facts: factsFor(event, ctx, suggested),
    receipts,
    needed,
    draftKind: draftKindOf(event),
    suggestedName: kicker.suggestedName,
    itemId: workItemId(event),
    proposalId: proposalIdOf(event),
    needsViewer: needsViewer(event, me, ctx.isShaper),
    fromAgent: kicker.kind !== "person",
    dueAt: dueAtOf(event),
    itemKind: workKindOf(event),
    parentId: workParentId(event),
    parentTitle: null,
  };

  if (event.kind === KIND_IO_DRAFT) {
    if (!needsViewer(event, me, ctx.isShaper)) return null;
    return {
      ...base,
      cardType: draftCardType(event),
      column: "needs_answer",
    };
  }

  if (event.kind === KIND_IO_WORK_ITEM) {
    const live = placeWorkItem(event, me, ctx.isShaper);
    if (!live) return null;
    const column = workState(event) === "done" ? "finished" : live;
    const kicker = workItemKicker(event, ctx, column);
    return {
      ...base,
      cardType: "offer",
      column,
      kickerKind: kicker.kind,
      kicker: kicker.text,
      fromAgent: kicker.kind !== "person",
    };
  }

  if (event.kind === KIND_IO_PROPOSAL) {
    if (!proposalIsOpen(event)) return null;
    const eligible = proposalEligible(event);
    const subject = proposalSubject(event);
    if (!eligible.includes(me) && subject !== me) return null;
    return { ...base, cardType: "decision", column: "needs_answer" };
  }

  return null;
}

export function classifyMyWork(
  events: readonly OrgEventLike[],
  ctx: ClassifyContext,
): Record<MyWorkColumn, OrgCardModel[]> {
  const drafts = events.filter((event) => event.kind === KIND_IO_DRAFT);
  const items = newestAddressable(events, KIND_IO_WORK_ITEM);
  const proposals = newestAddressable(events, KIND_IO_PROPOSAL);
  const models: OrgCardModel[] = [];
  for (const event of [...drafts, ...items, ...proposals]) {
    const model = classifyEvent(event, ctx);
    if (model) models.push(model);
  }
  const titles = new Map<string, string>();
  for (const event of items) {
    const id = anyTag(event.tags, "d");
    if (id) titles.set(id, claimOf(event));
  }
  const placed = models.map((model) =>
    model.parentId
      ? { ...model, parentTitle: titles.get(model.parentId) ?? null }
      : model,
  );
  placed.sort(compareByDue);
  return {
    needs_answer: placed.filter((model) => model.column === "needs_answer"),
    you_hold: placed.filter((model) => model.column === "you_hold"),
    you_offered: placed.filter((model) => model.column === "you_offered"),
    finished: placed.filter((model) => model.column === "finished"),
  };
}

/** Profile name for a pubkey, skipping the viewer and keys with no profile. */
export function memberDisplayName(
  pubkey: string,
  viewer: string,
  profiles:
    | Record<
        string,
        {
          displayName?: string | null;
          name?: string | null;
          nip05Handle?: string | null;
        }
      >
    | undefined,
): string | null {
  if (normalizePubkey(pubkey) === normalizePubkey(viewer)) return null;
  const profile = profiles?.[normalizePubkey(pubkey)] ?? profiles?.[pubkey];
  const name =
    profile?.displayName?.trim() ||
    profile?.name?.trim() ||
    profile?.nip05Handle?.trim();
  return name || null;
}

export function classifyContextFromEvents(
  events: readonly OrgEventLike[],
  viewer: string,
  nameOf: CardNameLookup,
): ClassifyContext {
  const agent = parseOrgAgentFromShapers(events as RelayEvent[]);
  return {
    viewer: normalizePubkey(viewer),
    agentPubkey: agent.pubkey,
    isShaper: viewerIsNamedShaper(events, viewer),
    nameOf,
  };
}

export function toOrgEvent(event: {
  id: string;
  pubkey: string;
  kind: number;
  content: string;
  createdAt?: number;
  created_at?: number;
  tags: string[][];
}): OrgEventLike {
  return {
    id: event.id,
    pubkey: event.pubkey,
    kind: event.kind,
    content: event.content,
    created_at: event.created_at ?? event.createdAt ?? 0,
    tags: event.tags,
  };
}
