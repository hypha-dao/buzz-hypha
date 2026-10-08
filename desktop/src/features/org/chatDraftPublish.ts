/**
 * The command a person signs when they publish a chat draft.
 * Agree stays off the command: My work is where Shapers vote.
 */

import {
  buildIoDirectionPropose,
  buildIoDriPropose,
  buildIoKnowledgeSet,
  buildIoProjectPropose,
  buildIoShapersPropose,
  buildIoWithdrawPropose,
  type UnsignedOrgCommand,
} from "./commands";
import { emptyRules, type ChatDraft, type CodebaseItem } from "./chatDraft";

export type DraftPublishInput = {
  draft: ChatDraft;
  title: string;
  brief: string;
  body: string;
  dueAt: number;
  why: string;
  /** Live direction version. A new direction proposes `version` as base. */
  directionBase: number;
  lines?: { text: string }[];
  rules?: Record<string, string>;
  decisionWindowSecs?: number | null;
  offerWindowSecs?: number | null;
  agentPubkey?: string | null;
  /** Project holder. `null` publishes with nobody. Omitted keeps the chat tag. */
  suggestedDri?: string | null;
  /** Edited codebases list. Omitted publishes the chat tag. */
  codebases?: CodebaseItem[];
  /** DRI holder chosen in the draft. Omitted keeps the chat tag. */
  holder?: string;
};

function publishedHolder(
  chosen: string | null | undefined,
  fromChat: string | null,
): string | undefined {
  if (chosen === null || chosen === "") return undefined;
  if (typeof chosen === "string") return chosen;
  return fromChat ?? undefined;
}

/** The command Publish signs. No agree tag — My work is where Shapers vote. */
export function commandForDraft(input: DraftPublishInput): UnsignedOrgCommand {
  const { draft } = input;
  switch (draft.kind) {
    case "project":
      return buildIoProjectPropose({
        title: input.title,
        brief: input.brief,
        dueAt: input.dueAt,
        suggestedDri: publishedHolder(input.suggestedDri, draft.suggestedDri),
        plan: draft.plan,
        voteAgree: false,
      });
    case "revise-project":
      return buildIoProjectPropose({
        title: input.title,
        brief: input.brief,
        dueAt: input.dueAt,
        revises: draft.proposalId,
        voteAgree: false,
      });
    case "direction":
      return buildIoDirectionPropose({
        slug: draft.slug,
        base: input.directionBase,
        body: input.body,
        lines: input.lines,
        why: input.why.trim() || "Drafted in chat.",
        voteAgree: false,
      });
    case "revise-direction":
      return buildIoDirectionPropose({
        slug: draft.slug,
        base: draft.base,
        body: input.body,
        lines: input.lines,
        why: input.why.trim() || "Revised in chat.",
        revises: draft.proposalId,
        voteAgree: false,
      });
    case "dri":
      return buildIoDriPropose({
        item: draft.itemId,
        pubkey: input.holder?.trim() || draft.pubkey,
        why: input.why.trim() || "Named in chat.",
        voteAgree: false,
      });
    case "remove-project":
      return buildIoWithdrawPropose({
        item: draft.itemId,
        why: input.why.trim() || "Removed in chat.",
        voteAgree: false,
      });
    case "shapers-add":
      return buildIoShapersPropose(
        {
          op: "add",
          pubkey: draft.pubkey,
          why: input.why.trim() || draft.why,
        },
        false,
      );
    case "shapers-remove":
      return buildIoShapersPropose(
        {
          op: "remove",
          pubkey: draft.pubkey,
          why: input.why.trim() || draft.why,
        },
        false,
      );
    case "shapers-rules": {
      const decision = positiveOrOmit(input.decisionWindowSecs);
      const offer = positiveOrOmit(input.offerWindowSecs);
      return buildIoShapersPropose(
        {
          op: "rules",
          rules: input.rules ?? { ...emptyRules(), ...draft.rules },
          ...(decision === undefined ? {} : { decision_window_secs: decision }),
          ...(offer === undefined ? {} : { offer_window_secs: offer }),
        },
        false,
      );
    }
    case "codebases":
      return buildIoKnowledgeSet({
        items: input.codebases ?? draft.items,
      });
    case "shapers-agent": {
      const pubkey = (input.agentPubkey ?? draft.pubkey)?.trim();
      return buildIoShapersPropose(
        {
          op: "agent",
          ...(pubkey ? { pubkey } : {}),
          why: input.why.trim() || draft.why,
        },
        false,
      );
    }
  }
}

function positiveOrOmit(value: number | null | undefined): number | undefined {
  if (value == null || !Number.isFinite(value) || value <= 0) return undefined;
  return Math.floor(value);
}

export function dateInputValue(unix: number): string {
  return new Date(unix * 1000).toISOString().slice(0, 10);
}

/** Keep the original timestamp when the calendar day did not change. */
export function dateInputToUnix(value: string, previous: number): number {
  if (dateInputValue(previous) === value) return previous;
  const [year, month, day] = value.split("-").map(Number);
  if (!year || !month || !day) return previous;
  return Math.floor(Date.UTC(year, month - 1, day, 12) / 1000);
}
