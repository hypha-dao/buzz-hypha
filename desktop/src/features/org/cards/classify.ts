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
  workOfferedBy,
  workOfferedTo,
  workState,
} from "./parse";
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
};

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
  return { kind: "person", text: "Needs your answer", suggestedName };
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
  if (event.kind === KIND_IO_WORK_ITEM) {
    const offeredTo = workOfferedTo(event);
    const dri = workDri(event);
    const state = workState(event);
    if (state) facts.push({ label: "State", value: state });
    if (offeredTo)
      facts.push({ label: "Offered to", value: ctx.nameOf(offeredTo) });
    if (dri) facts.push({ label: "Holds it", value: ctx.nameOf(dri) });
  }
  if (event.kind === KIND_IO_PROPOSAL) {
    const needed = proposalNeeded(event);
    if (needed) {
      facts.push({
        label: "Needs",
        value: `${needed.agrees} of ${needed.needed}`,
      });
    }
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
    const offeredTo = workOfferedTo(event);
    const dri = workDri(event);
    const offeredBy = workOfferedBy(event);
    const state = workState(event);
    if (state === "done") return null;
    if (offeredTo === me) {
      return { ...base, cardType: "offer", column: "needs_answer" };
    }
    if (dri === me) {
      return { ...base, cardType: "offer", column: "you_hold" };
    }
    if (offeredBy === me || offeredBy === "agent") {
      return { ...base, cardType: "offer", column: "you_offered" };
    }
    return null;
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
  models.sort((left, right) => right.event.created_at - left.event.created_at);
  return {
    needs_answer: models.filter((model) => model.column === "needs_answer"),
    you_hold: models.filter((model) => model.column === "you_hold"),
    you_offered: models.filter((model) => model.column === "you_offered"),
  };
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
