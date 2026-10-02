import * as React from "react";

import {
  classifyContextFromEvents,
  classifyMyWork,
  memberDisplayName,
  type OrgCardModel,
} from "@/features/org/cards";
import {
  DecisionCard,
  ProposalDetailPanel,
} from "@/features/org/cards/DecisionCard";
import { OrgCardShell } from "@/features/org/cards/OrgCardShell";
import { useMyWorkEvents } from "@/features/org/hooks";
import { useLiveDoorEvents } from "@/features/org/hooks/useLiveReq";
import {
  currentDirectionProposals,
  proposalDetail,
  proposalMessageId,
  type ChatProposalPreview,
} from "@/features/org/proposalDetail";
import { resolveUserLabel } from "@/features/profile/lib/identity";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import { useIdentityQuery } from "@/shared/api/hooks";
import type { RelayEvent } from "@/shared/api/types";
import type { RelaySubscriptionFilter } from "@/shared/api/relayClientShared";
import { KIND_IO_PROPOSAL } from "@/shared/constants/kinds";

const CHAT_PROPOSALS: RelaySubscriptionFilter[] = [
  {
    kinds: [KIND_IO_PROPOSAL],
    "#t": ["project", "direction"],
    "#s": ["open", "passed"],
    limit: 40,
  },
];

export type ShapersChatMessage = {
  id: string;
  createdAt: number;
  body?: string;
  tags?: readonly (readonly string[])[];
};

type FooterMap = Record<string, React.ReactNode>;

function newestById(events: readonly RelayEvent[]): RelayEvent[] {
  const newest = new Map<string, RelayEvent>();
  for (const event of events) {
    if (event.kind !== KIND_IO_PROPOSAL) continue;
    const id = event.tags.find((tag) => tag[0] === "d")?.[1];
    if (!id) continue;
    const current = newest.get(id);
    if (!current || event.created_at >= current.created_at)
      newest.set(id, event);
  }
  return [...newest.values()].sort(
    (left, right) => right.created_at - left.created_at,
  );
}

function previewModel(preview: ChatProposalPreview): OrgCardModel {
  return {
    event: {
      id: `preview-${preview.messageId}`,
      pubkey: "",
      kind: KIND_IO_PROPOSAL,
      content: "{}",
      created_at: 0,
      tags: [],
    },
    cardType: "decision",
    column: "needs_answer",
    kickerKind: "drafted",
    kicker: "Up for a vote",
    claim: preview.title,
    facts: [],
    receipts: [],
    needed: null,
    draftKind: null,
    suggestedName: null,
    itemId: null,
    proposalId: preview.proposalId,
    needsViewer: false,
    fromAgent: true,
    dueAt: preview.dueAt,
    itemKind: null,
    parentId: null,
    parentTitle: null,
  };
}

function PassedCard({ event }: { event: RelayEvent }) {
  const detail = proposalDetail(event);
  if (!detail) return null;
  const model: OrgCardModel = {
    ...previewModel({
      messageId: event.id,
      kind: detail.kind,
      title: detail.title,
      body: detail.body,
      proposalId: detail.proposalId,
      dueAt: detail.dueAt,
    }),
    event,
    kicker: detail.kind === "project" ? "Project" : "Direction",
  };
  return (
    <OrgCardShell
      detail={<ProposalDetailPanel event={event} />}
      model={model}
    />
  );
}

function Cards({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="flex max-w-xl flex-col gap-2 py-1"
      data-testid="shapers-chat-proposal"
    >
      {children}
    </div>
  );
}

/**
 * Proposal cards in the #shapers thread, under the message they belong to.
 * A click opens the proposal page. Agree and decline stay on the card.
 * An unpublished draft is not a vote card. Nothing is pinned
 * above the conversation.
 */
export function ShapersChatProposals({
  messages,
  onFooters,
}: {
  messages: readonly ShapersChatMessage[];
  onFooters: (signature: string, footers: FooterMap) => void;
}) {
  const viewer = useIdentityQuery().data?.pubkey ?? null;
  const mine = useMyWorkEvents();
  const listed = useLiveDoorEvents(CHAT_PROPOSALS);
  const events = React.useMemo(() => {
    const byId = new Map<string, RelayEvent>();
    for (const event of mine.events) byId.set(event.id, event);
    for (const event of listed.events) byId.set(event.id, event);
    return [...byId.values()];
  }, [listed.events, mine.events]);
  const pubkeys = React.useMemo(() => {
    const found = new Set<string>();
    for (const event of events) {
      found.add(event.pubkey);
      for (const tag of event.tags) {
        if (tag[0] === "p" && tag[1]) found.add(tag[1]);
      }
    }
    return [...found];
  }, [events]);
  const profiles = useUsersBatchQuery(pubkeys).data?.profiles;
  const open = React.useMemo(() => {
    if (!viewer) return [];
    const nameOf = (pubkey: string) =>
      resolveUserLabel({ pubkey, currentPubkey: viewer, profiles });
    const displayNameOf = (pubkey: string) =>
      memberDisplayName(pubkey, viewer, profiles);
    return classifyMyWork(events, {
      ...classifyContextFromEvents(events, viewer, nameOf),
      displayNameOf,
    }).needs_answer.filter((model) => model.event.kind === KIND_IO_PROPOSAL);
  }, [events, profiles, viewer]);
  const passed = React.useMemo(
    () =>
      currentDirectionProposals(
        newestById(listed.events).filter((event) => {
          const status = event.tags.find((tag) => tag[0] === "s")?.[1];
          return status === "passed" && proposalDetail(event) !== null;
        }),
      ),
    [listed.events],
  );

  const placed = React.useMemo(() => {
    const attached = new Map<string, OrgCardModel[]>();
    const passedAttached = new Map<string, RelayEvent[]>();
    for (const model of open) {
      const detail = proposalDetail(model.event);
      const messageId = detail
        ? proposalMessageId(messages, detail, model.event.created_at)
        : null;
      if (!messageId) continue;
      const list = attached.get(messageId) ?? [];
      list.push(model);
      attached.set(messageId, list);
    }
    for (const event of passed) {
      const detail = proposalDetail(event);
      const messageId = detail
        ? proposalMessageId(messages, detail, event.created_at)
        : null;
      if (!messageId) continue;
      const list = passedAttached.get(messageId) ?? [];
      list.push(event);
      passedAttached.set(messageId, list);
    }
    return { attached, passedAttached };
  }, [messages, open, passed]);

  const signature = React.useMemo(() => {
    const attached = [...placed.attached.entries()]
      .map(
        ([messageId, models]) =>
          `${messageId}:${models.map((model) => model.event.id).join(",")}`,
      )
      .join("|");
    const passedRows = [...placed.passedAttached.entries()]
      .map(
        ([messageId, events]) =>
          `${messageId}:${events.map((event) => event.id).join(",")}`,
      )
      .join("|");
    return `${attached}#${passedRows}`;
  }, [placed]);

  const footers = React.useMemo(() => {
    const record: FooterMap = {};
    for (const [messageId, models] of placed.attached) {
      record[messageId] = (
        <Cards>
          {models.map((model) => (
            <DecisionCard key={model.event.id} model={model} />
          ))}
          {(placed.passedAttached.get(messageId) ?? []).map((event) => (
            <PassedCard event={event} key={event.id} />
          ))}
        </Cards>
      );
    }
    for (const [messageId, events] of placed.passedAttached) {
      if (record[messageId]) continue;
      record[messageId] = (
        <Cards>
          {events.map((event) => (
            <PassedCard event={event} key={event.id} />
          ))}
        </Cards>
      );
    }
    return record;
  }, [placed]);

  React.useEffect(() => {
    onFooters(signature, footers);
  }, [footers, onFooters, signature]);

  React.useEffect(() => {
    return () => onFooters("", {});
  }, [onFooters]);

  return null;
}
