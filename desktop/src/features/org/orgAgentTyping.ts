import { ORG_AGENT_LABEL } from "@/features/org/orgAgent";
import type { UserProfileLookup } from "@/features/profile/lib/identity";
import { normalizePubkey } from "@/shared/lib/pubkey";

/**
 * How long the composer shows "Org. Agent is typing…" after a send when no
 * reply has arrived. Covers one model call (60s) plus pickup time; a dead
 * agent does not leave the row up.
 */
export const ORG_AGENT_TYPING_ACK_MS = 90_000;

/** Messages older than the send by more than this are history, not the reply. */
const REPLY_CLOCK_SLACK_SEC = 5;

export type OrgAgentTypingAck = {
  generation: number;
  startedAt: number;
  sentAtSec: number;
  baselineAgentMessageId: string | null;
};

type AgentMessage = {
  id: string;
  pubkey?: string;
  signerPubkey?: string;
  createdAt: number;
};

/** Newest timeline row signed by the org agent, if one is loaded. */
export function latestOrgAgentMessage<T extends AgentMessage>(
  messages: readonly T[],
  orgAgentPubkey: string,
): { id: string; createdAt: number } | null {
  const agent = normalizePubkey(orgAgentPubkey);
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (!message) continue;
    const author = message.pubkey ?? message.signerPubkey;
    if (author && normalizePubkey(author) === agent) {
      return { id: message.id, createdAt: message.createdAt };
    }
  }
  return null;
}

/**
 * True when a timeline row is the agent's answer to this send, not a message
 * that was already on screen (or older history that loaded afterward).
 */
export function agentReplyArrived({
  baselineId,
  latestId,
  latestCreatedAt,
  sentAtSec,
}: {
  baselineId: string | null;
  latestId: string | null;
  latestCreatedAt: number | null;
  sentAtSec: number;
}): boolean {
  if (!latestId || latestId === baselineId) return false;
  if (baselineId !== null) return true;
  if (latestCreatedAt == null) return false;
  return latestCreatedAt >= sentAtSec - REPLY_CLOCK_SLACK_SEC;
}

const CONCERN_PHRASES = [
  "can you",
  "could you",
  "would you",
  "will you",
  "please",
  "let's",
  "lets ",
  "we should",
  "we need",
  "i want",
  "i need",
  "mark it done",
  "mark done",
  "it's done",
  "its done",
  "open a",
  "set the",
  "set our",
] as const;

const CONCERN_WORDS = new Set([
  "mission",
  "vision",
  "objective",
  "objectives",
  "strategy",
  "project",
  "ticket",
  "dri",
  "holder",
  "proposal",
  "direction",
  "publish",
  "draft",
  "assign",
  "bootstrap",
  "shaper",
  "shapers",
]);

/**
 * Same gate the org agent uses before it calls the model in a channel.
 * A DM answers every line. A mention, a follow-up to its question, or a
 * line about the work it drafts does too. Other chatter does not.
 */
export function lineConcernsOrgAgent(body: string): boolean {
  const lower = body.toLowerCase();
  if (lower.includes("org agent") || lower.includes("org. agent")) return true;
  if (CONCERN_PHRASES.some((phrase) => lower.includes(phrase))) return true;
  const tokens = lower.split(/[^a-z0-9]+/).filter((token) => token.length > 0);
  return tokens.some((token) => CONCERN_WORDS.has(token));
}

export function orgAgentWillAnswer({
  answersEveryLine,
  body,
  followsAgentQuestion,
  mentionsAgent,
}: {
  answersEveryLine: boolean;
  body: string;
  followsAgentQuestion: boolean;
  mentionsAgent: boolean;
}): boolean {
  if (answersEveryLine || mentionsAgent || followsAgentQuestion) return true;
  return lineConcernsOrgAgent(body);
}

export function messageMentionsOrgAgent(
  tags: readonly (readonly string[])[] | undefined,
  orgAgentPubkey: string,
): boolean {
  const agent = normalizePubkey(orgAgentPubkey);
  return (
    tags?.some(
      (tag) => tag[0] === "p" && tag[1] && normalizePubkey(tag[1]) === agent,
    ) ?? false
  );
}

export function previousMessageIsOrgAgentQuestion(
  messages: readonly (AgentMessage & { body?: string })[],
  orgAgentPubkey: string,
): boolean {
  const previous = messages[messages.length - 1];
  if (!previous?.body?.includes("?")) return false;
  return isOrgAgentAuthor(previous, orgAgentPubkey);
}

export function isOrgAgentAuthor(
  message: AgentMessage,
  orgAgentPubkey: string,
): boolean {
  const author = message.pubkey ?? message.signerPubkey;
  if (!author) return false;
  return normalizePubkey(author) === normalizePubkey(orgAgentPubkey);
}

type PromptMessage = AgentMessage & {
  body?: string;
  tags?: readonly (readonly string[])[];
};

/**
 * A human line that just arrived and that the org agent will answer.
 * An agent reply in the same batch means the turn is already over.
 */
export function orgAgentPromptFromFresh<T extends PromptMessage>(
  messages: readonly T[],
  fresh: readonly T[],
  orgAgentPubkey: string,
  answersEveryLine: boolean,
): T | null {
  const human = [...fresh]
    .reverse()
    .find((message) => !isOrgAgentAuthor(message, orgAgentPubkey));
  if (!human) return null;
  const replied = fresh.some(
    (message) =>
      message.id !== human.id &&
      message.createdAt >= human.createdAt &&
      isOrgAgentAuthor(message, orgAgentPubkey),
  );
  if (replied) return null;
  const index = messages.findIndex((message) => message.id === human.id);
  const previous = index > 0 ? messages[index - 1] : undefined;
  const followsAgentQuestion = previous
    ? previous.body?.includes("?") === true &&
      isOrgAgentAuthor(previous, orgAgentPubkey)
    : false;
  if (
    !orgAgentWillAnswer({
      answersEveryLine,
      body: human.body ?? "",
      followsAgentQuestion,
      mentionsAgent: messageMentionsOrgAgent(human.tags, orgAgentPubkey),
    })
  ) {
    return null;
  }
  return human;
}

/** Whether the composer should show the org agent as typing for this ack. */
export function orgAgentTypingActive({
  ack,
  enabled,
  now,
  latestAgentId,
  latestAgentCreatedAt,
}: {
  ack: OrgAgentTypingAck | null;
  enabled: boolean;
  now: number;
  latestAgentId: string | null;
  latestAgentCreatedAt: number | null;
}): boolean {
  if (!enabled || !ack) return false;
  if (now - ack.startedAt >= ORG_AGENT_TYPING_ACK_MS) return false;
  return !agentReplyArrived({
    baselineId: ack.baselineAgentMessageId,
    latestId: latestAgentId,
    latestCreatedAt: latestAgentCreatedAt,
    sentAtSec: ack.sentAtSec,
  });
}

/** Append one typing pubkey without copying the list when it is already there. */
export function mergeTypingPubkey(
  pubkeys: readonly string[],
  extra: string | null,
): string[] {
  if (!extra) return pubkeys as string[];
  const agent = normalizePubkey(extra);
  if (pubkeys.some((pubkey) => normalizePubkey(pubkey) === agent)) {
    return pubkeys as string[];
  }
  return [...pubkeys, extra];
}

/**
 * Profile lookup for the typing row. Returns the same reference when the org
 * agent already has a display name, so a quiet composer does not re-render
 * neighbors. Fills "Org. Agent" when the kind:0 has not loaded yet.
 */
export function withOrgAgentTypingProfile(
  profiles: UserProfileLookup | undefined,
  orgAgentPubkey: string | null,
  active: boolean,
): UserProfileLookup | undefined {
  if (!active || !orgAgentPubkey) return profiles;
  const key = normalizePubkey(orgAgentPubkey);
  const existing = profiles?.[key];
  if (existing?.displayName?.trim()) return profiles;
  return {
    ...profiles,
    [key]: {
      displayName: ORG_AGENT_LABEL,
      name: existing?.name ?? null,
      avatarUrl: existing?.avatarUrl ?? null,
      nip05Handle: existing?.nip05Handle ?? null,
      ownerPubkey: existing?.ownerPubkey ?? null,
      isAgent: true,
    },
  };
}
