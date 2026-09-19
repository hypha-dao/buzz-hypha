/**
 * Build, sign (`sign_event`), and publish every person-signed org command
 * (Protocol §4.8 / C-1 tag layout). Tags name the target; content carries
 * the rest. A draft `e` tag is `["e", <draft>, "", "draft"]` when settling
 * a card. Do not invent tags.
 *
 * Money (`50013`/`50014`) and join (`50016`) are reserved — the relay
 * refuses them today — but the tag layout is specified, so the builders
 * exist for the same reason C-1's do.
 */

import { relayClient } from "@/shared/api/relayClient";
import { signRelayEvent } from "@/shared/api/tauri";
import type { RelayEvent } from "@/shared/api/types";
import {
  KIND_IO_ACCEPT,
  KIND_IO_DECLINE,
  KIND_IO_DIRECTION_PROPOSE,
  KIND_IO_DONE,
  KIND_IO_DRAFT_DECIDE,
  KIND_IO_DRI_PROPOSE,
  KIND_IO_HEALTH_RATE,
  KIND_IO_JOIN_PROPOSE,
  KIND_IO_MONEY_PROPOSE,
  KIND_IO_MONEY_RELEASED,
  KIND_IO_OFFER,
  KIND_IO_PROFILE_SET,
  KIND_IO_PROJECT_PROPOSE,
  KIND_IO_RELEASE,
  KIND_IO_REOPEN,
  KIND_IO_SET_DUE,
  KIND_IO_SHAPER_ACCEPT,
  KIND_IO_SHAPER_STEP_DOWN,
  KIND_IO_SHAPERS_PROPOSE,
  KIND_IO_TICKET_CREATE,
  KIND_IO_VOTE,
} from "@/shared/constants/kinds";
import { normalizePubkey } from "@/shared/lib/pubkey";

import {
  draftTag,
  receiptTag,
  TAG_BAND,
  TAG_BASE,
  TAG_DUE,
  TAG_ITEM,
  TAG_OP,
  TAG_OUTCOME,
  TAG_PARENT,
  TAG_REASON,
  TAG_TX,
  TAG_VOTE,
  TAG_WEEK,
} from "./tags";

const HEX_64 = /^[0-9a-f]{64}$/;
const UUID =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const ISO_WEEK = /^[0-9]{4}-W(?:0[1-9]|[1-4][0-9]|5[0-3])$/;

export type DirectionSlug = "mission" | "vision" | "objectives" | "strategy";
export type VoteChoice = "agree" | "decline";
export type ShapersOp = "add" | "remove" | "rules" | "agent";
export type DraftDecision = "accept" | "decline";
export type HealthBand = "struggling" | "wobbly" | "healthy";
export type DeclineReason =
  | "already_covered"
  | "not_what_the_line_meant"
  | "too_big"
  | "too_small"
  | "wrong_holder"
  | "not_now"
  | "other";

export type UnsignedOrgCommand = {
  kind: number;
  content: string;
  tags: string[][];
};

export class OrgCommandError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "OrgCommandError";
  }
}

function requireHex64(value: string, field: string): string {
  const normalized = field === "p" || field.endsWith("pubkey") || field === "payee"
    ? normalizePubkey(value)
    : value.toLowerCase();
  if (!HEX_64.test(normalized)) {
    throw new OrgCommandError(`${field} must be a 64-char hex string`);
  }
  return normalized;
}

function requireUuid(value: string, field: string): string {
  const normalized = value.toLowerCase();
  if (!UUID.test(normalized)) {
    throw new OrgCommandError(`${field} must be a lowercase RFC 4122 uuid`);
  }
  return normalized;
}

function requireNonEmpty(value: string, field: string): string {
  if (value.trim().length === 0) {
    throw new OrgCommandError(`${field} must not be empty`);
  }
  return value;
}

function whyJson(why?: string): string {
  return why === undefined ? "{}" : JSON.stringify({ why });
}

function optionalDraft(tags: string[][], draftId?: string): void {
  if (draftId !== undefined) {
    tags.push(draftTag(requireHex64(draftId, "draft")));
  }
}

function optionalVoteAgree(tags: string[][], voteAgree?: boolean): void {
  if (voteAgree) tags.push([TAG_VOTE, "agree"]);
}

export type ShapersProposal =
  | { op: "add" | "remove"; pubkey: string; why?: string }
  | {
      op: "rules";
      rules: Record<string, unknown>;
      decision_window_secs?: number;
      offer_window_secs?: number;
    }
  | { op: "agent"; pubkey?: string; why?: string };

export function buildIoShapersPropose(
  proposal: ShapersProposal,
  voteAgree = false,
): UnsignedOrgCommand {
  const tags: string[][] = [[TAG_OP, proposal.op]];
  let content: string;
  if (proposal.op === "add" || proposal.op === "remove") {
    tags.push(["p", requireHex64(proposal.pubkey, "p")]);
    content = whyJson(proposal.why);
  } else if (proposal.op === "rules") {
    const body: Record<string, unknown> = { rules: proposal.rules };
    if (proposal.decision_window_secs !== undefined) {
      body.decision_window_secs = proposal.decision_window_secs;
    }
    if (proposal.offer_window_secs !== undefined) {
      body.offer_window_secs = proposal.offer_window_secs;
    }
    content = JSON.stringify(body);
  } else {
    if (proposal.pubkey !== undefined) {
      tags.push(["p", requireHex64(proposal.pubkey, "p")]);
    }
    content = whyJson(proposal.why);
  }
  optionalVoteAgree(tags, voteAgree);
  return { kind: KIND_IO_SHAPERS_PROPOSE, tags, content };
}

export function buildIoDirectionPropose(input: {
  slug: DirectionSlug;
  base: number;
  body: string;
  lines?: unknown[];
  why?: string;
  draftId?: string;
  voteAgree?: boolean;
}): UnsignedOrgCommand {
  const tags: string[][] = [
    ["d", input.slug],
    [TAG_BASE, String(input.base)],
  ];
  optionalDraft(tags, input.draftId);
  optionalVoteAgree(tags, input.voteAgree);
  const content: Record<string, unknown> = {
    body: requireNonEmpty(input.body, "body"),
  };
  if (input.lines !== undefined) content.lines = input.lines;
  if (input.why !== undefined) content.why = input.why;
  return { kind: KIND_IO_DIRECTION_PROPOSE, tags, content: JSON.stringify(content) };
}

export function buildIoVote(input: {
  proposal: string;
  vote: VoteChoice;
  reason?: string;
}): UnsignedOrgCommand {
  return {
    kind: KIND_IO_VOTE,
    tags: [
      ["e", requireUuid(input.proposal, "e")],
      [TAG_VOTE, input.vote],
    ],
    content: input.reason === undefined ? "{}" : JSON.stringify({ reason: input.reason }),
  };
}

export function buildIoShaperAccept(proposal: string): UnsignedOrgCommand {
  return {
    kind: KIND_IO_SHAPER_ACCEPT,
    tags: [["e", requireUuid(proposal, "e")]],
    content: "{}",
  };
}

export function buildIoShaperStepDown(why?: string): UnsignedOrgCommand {
  return { kind: KIND_IO_SHAPER_STEP_DOWN, tags: [], content: whyJson(why) };
}

export function buildIoProjectPropose(input: {
  title: string;
  brief: string;
  dueAt: number;
  objectiveRef?: string;
  suggestedDri?: string;
  draftId?: string;
  voteAgree?: boolean;
}): UnsignedOrgCommand {
  requireNonEmpty(input.title, "title");
  const tags: string[][] = [];
  optionalDraft(tags, input.draftId);
  optionalVoteAgree(tags, input.voteAgree);
  const content: Record<string, unknown> = {
    title: input.title,
    brief: input.brief,
    due_at: input.dueAt,
  };
  if (input.objectiveRef !== undefined) content.objective_ref = input.objectiveRef;
  if (input.suggestedDri !== undefined) {
    content.suggested_dri = requireHex64(input.suggestedDri, "suggested_dri");
  }
  return { kind: KIND_IO_PROJECT_PROPOSE, tags, content: JSON.stringify(content) };
}

export function buildIoTicketCreate(input: {
  parent: string;
  title: string;
  brief: string;
  dueAt: number;
  offerTo?: string;
  after?: string[];
  draftId?: string;
}): UnsignedOrgCommand {
  requireNonEmpty(input.title, "title");
  const tags: string[][] = [[TAG_PARENT, requireUuid(input.parent, "u")]];
  if (input.offerTo !== undefined) {
    tags.push(["p", requireHex64(input.offerTo, "p")]);
  }
  optionalDraft(tags, input.draftId);
  const content: Record<string, unknown> = {
    title: input.title,
    brief: input.brief,
    due_at: input.dueAt,
  };
  if (input.after && input.after.length > 0) {
    content.after = input.after.map((id) => requireUuid(id, "after[]"));
  }
  return { kind: KIND_IO_TICKET_CREATE, tags, content: JSON.stringify(content) };
}

export function buildIoOffer(input: {
  item: string;
  pubkey: string;
  draftId?: string;
}): UnsignedOrgCommand {
  const tags: string[][] = [
    [TAG_ITEM, requireUuid(input.item, "i")],
    ["p", requireHex64(input.pubkey, "p")],
  ];
  optionalDraft(tags, input.draftId);
  return { kind: KIND_IO_OFFER, tags, content: "{}" };
}

export function buildIoAccept(item: string): UnsignedOrgCommand {
  return {
    kind: KIND_IO_ACCEPT,
    tags: [[TAG_ITEM, requireUuid(item, "i")]],
    content: "{}",
  };
}

export function buildIoDecline(item: string): UnsignedOrgCommand {
  return {
    kind: KIND_IO_DECLINE,
    tags: [[TAG_ITEM, requireUuid(item, "i")]],
    content: "{}",
  };
}

export function buildIoDone(input: {
  item: string;
  receiptId?: string;
  draftId?: string;
}): UnsignedOrgCommand {
  const tags: string[][] = [[TAG_ITEM, requireUuid(input.item, "i")]];
  if (input.receiptId !== undefined) {
    tags.push(receiptTag(requireHex64(input.receiptId, "receipt")));
  }
  optionalDraft(tags, input.draftId);
  return { kind: KIND_IO_DONE, tags, content: "{}" };
}

export function buildIoRelease(item: string, why?: string): UnsignedOrgCommand {
  return {
    kind: KIND_IO_RELEASE,
    tags: [[TAG_ITEM, requireUuid(item, "i")]],
    content: whyJson(why),
  };
}

export function buildIoSetDue(
  item: string,
  dueAt: number,
  why?: string,
): UnsignedOrgCommand {
  return {
    kind: KIND_IO_SET_DUE,
    tags: [
      [TAG_ITEM, requireUuid(item, "i")],
      [TAG_DUE, String(dueAt)],
    ],
    content: whyJson(why),
  };
}

export function buildIoReopen(item: string, why?: string): UnsignedOrgCommand {
  return {
    kind: KIND_IO_REOPEN,
    tags: [[TAG_ITEM, requireUuid(item, "i")]],
    content: whyJson(why),
  };
}

export function buildIoDraftDecide(input: {
  draftId: string;
  outcome: DraftDecision;
  reason?: DeclineReason;
}): UnsignedOrgCommand {
  const tags: string[][] = [
    ["e", requireHex64(input.draftId, "e")],
    [TAG_OUTCOME, input.outcome],
  ];
  if (input.reason !== undefined) tags.push([TAG_REASON, input.reason]);
  return { kind: KIND_IO_DRAFT_DECIDE, tags, content: "{}" };
}

export function buildIoMoneyPropose(input: {
  item: string;
  payee: string;
  amount: string;
  currency: string;
  note?: string;
  agreed?: { amount: string; heard: string };
  draftId?: string;
}): UnsignedOrgCommand {
  const tags: string[][] = [
    [TAG_ITEM, requireUuid(input.item, "i")],
    ["p", requireHex64(input.payee, "p")],
  ];
  optionalDraft(tags, input.draftId);
  const content: Record<string, unknown> = {
    amount: input.amount,
    currency: input.currency,
  };
  if (input.note !== undefined) content.note = input.note;
  if (input.agreed !== undefined) content.agreed = input.agreed;
  return { kind: KIND_IO_MONEY_PROPOSE, tags, content: JSON.stringify(content) };
}

export function buildIoMoneyReleased(input: {
  proposal: string;
  tx: string;
  chain: string;
  contract: string;
  amount: string;
  currency: string;
}): UnsignedOrgCommand {
  return {
    kind: KIND_IO_MONEY_RELEASED,
    tags: [
      ["e", requireUuid(input.proposal, "e")],
      [TAG_TX, requireNonEmpty(input.tx, "tx")],
    ],
    content: JSON.stringify({
      chain: input.chain,
      contract: input.contract,
      amount: input.amount,
      currency: input.currency,
    }),
  };
}

export function buildIoDriPropose(input: {
  item: string;
  pubkey: string;
  why?: string;
  draftId?: string;
  voteAgree?: boolean;
}): UnsignedOrgCommand {
  const tags: string[][] = [
    [TAG_ITEM, requireUuid(input.item, "i")],
    ["p", requireHex64(input.pubkey, "p")],
  ];
  optionalDraft(tags, input.draftId);
  optionalVoteAgree(tags, input.voteAgree);
  return { kind: KIND_IO_DRI_PROPOSE, tags, content: whyJson(input.why) };
}

export function buildIoJoinPropose(
  pubkey: string,
  note?: string,
): UnsignedOrgCommand {
  return {
    kind: KIND_IO_JOIN_PROPOSE,
    tags: [["p", requireHex64(pubkey, "p")]],
    content: note === undefined ? "{}" : JSON.stringify({ note }),
  };
}

export function buildIoHealthRate(input: {
  item: string;
  week: string;
  band: HealthBand;
}): UnsignedOrgCommand {
  if (!ISO_WEEK.test(input.week)) {
    throw new OrgCommandError(
      `week must be an ISO week like 2026-W38 (got ${input.week})`,
    );
  }
  return {
    kind: KIND_IO_HEALTH_RATE,
    tags: [
      [TAG_ITEM, requireUuid(input.item, "i")],
      [TAG_WEEK, input.week],
      [TAG_BAND, input.band],
    ],
    content: "{}",
  };
}

export function buildIoProfileSet(input: {
  about: string;
  skills: string[];
  openLimit?: number;
  draftId?: string;
}): UnsignedOrgCommand {
  const tags: string[][] = [];
  optionalDraft(tags, input.draftId);
  const content: Record<string, unknown> = {
    about: input.about,
    skills: input.skills,
  };
  if (input.openLimit !== undefined) content.open_limit = input.openLimit;
  return { kind: KIND_IO_PROFILE_SET, tags, content: JSON.stringify(content) };
}

/**
 * Sign with `sign_event` and publish over the existing EVENT path.
 * Failures propagate (no catch-log-and-return-success).
 */
export async function publishOrgCommand(
  command: UnsignedOrgCommand,
): Promise<RelayEvent> {
  const event = await signRelayEvent({
    kind: command.kind,
    content: command.content,
    tags: command.tags,
  });
  await relayClient.publishEvent(
    event,
    "Timed out publishing the org command.",
    "Failed to publish the org command.",
  );
  return event;
}
