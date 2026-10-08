import * as React from "react";

import type { TimelineMessage } from "@/features/messages/types";
import { useIdentityQuery } from "@/shared/api/hooks";
import type { RelayEvent } from "@/shared/api/types";
import { KIND_STREAM_MESSAGE } from "@/shared/constants/kinds";

import { nextChatAct, type ChatActMessage } from "./actFromChat";
import {
  buildIoDone,
  buildIoProfileSet,
  buildIoTicketCreate,
  buildIoWithdraw,
  publishOrgCommand,
} from "./commands";
import { useLiveDoorEvents } from "./hooks/useLiveReq";
import { useOverviewEvents } from "./hooks/useOverviewEvents";
import { useWorkEvents } from "./hooks/useWorkEvents";
import { useOrgAgentPubkey } from "./useOrgAgent";
import { parseShapersState } from "./ui/overview/parseOverview";
import { latestWorkItems, parseWorkItem } from "./work/model";

const STORAGE_KEY = "buzz.org.actFromChat";
const MAX_HANDLED = 200;
/** Shared across the channel pane and the background signer. */
const claimed = new Set<string>();

function claim(id: string): boolean {
  if (claimed.has(id)) return false;
  claimed.add(id);
  return true;
}

function loadHandled(): Set<string> {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(parsed)) return new Set();
    return new Set(parsed.filter((id): id is string => typeof id === "string"));
  } catch {
    return new Set();
  }
}

function persistHandled(ids: ReadonlySet<string>): void {
  try {
    sessionStorage.setItem(
      STORAGE_KEY,
      JSON.stringify([...ids].slice(-MAX_HANDLED)),
    );
  } catch {
    // A private window can refuse storage. The in-memory set still dedupes
    // this view.
  }
}

function asChat(message: TimelineMessage): ChatActMessage {
  return {
    id: message.id,
    createdAt: message.createdAt,
    pubkey: message.signerPubkey ?? message.pubkey,
    signerPubkey: message.signerPubkey,
    tags: message.tags,
    pending: message.pending,
  };
}

/**
 * When the Org. Agent tags a reply as done, a ticket, or a direct removal,
 * sign that command with the member's key. Proposals stay drafts in the
 * chat until the member opens one and publishes it.
 */
export function useActFromChat(input: {
  messages: readonly TimelineMessage[];
  currentPubkey: string | null;
  orgAgentPubkey: string | null;
  room?: "dm" | "shapers" | "channel";
}): void {
  const { events: overviewEvents } = useOverviewEvents();
  const { events: workEvents, isLoading: workLoading } = useWorkEvents();
  const [handled, setHandled] =
    React.useState<ReadonlySet<string>>(loadHandled);
  const handledRef = React.useRef(handled);
  const inflight = React.useRef<string | null>(null);
  handledRef.current = handled;

  const remember = React.useCallback((id: string) => {
    if (handledRef.current.has(id)) return;
    const next = new Set(handledRef.current);
    next.add(id);
    handledRef.current =
      next.size > MAX_HANDLED ? new Set([...next].slice(-MAX_HANDLED)) : next;
    setHandled(handledRef.current);
    persistHandled(handledRef.current);
  }, []);

  React.useEffect(() => {
    if (!input.currentPubkey || !input.orgAgentPubkey) return;
    if (inflight.current) return;
    const shapers = parseShapersState(overviewEvents);
    const items = workLoading
      ? null
      : [...latestWorkItems(workEvents).values()].flatMap((event) => {
          const item = parseWorkItem(event);
          if (!item || item.state === "withdrawn") return [];
          const offered = item.offeredByMember ?? item.offeredBy;
          return [
            {
              id: item.id,
              state: item.state,
              dri: item.dri,
              parent: item.parent,
              createdBy: item.createdBy,
              offeredBy:
                offered && /^[0-9a-f]{64}$/.test(offered) ? offered : null,
            },
          ];
        });
    let act = nextChatAct({
      messages: input.messages.map(asChat),
      currentPubkey: input.currentPubkey,
      orgAgentPubkey: input.orgAgentPubkey,
      shaperPubkeys: shapers ? shapers.shapers : null,
      items,
      handledIds: handledRef.current,
      room: input.room,
    });
    let skips = 0;
    while (act !== "wait" && act !== null && act.kind === "skip") {
      remember(act.agentEventId);
      skips += 1;
      if (skips > 20) return;
      act = nextChatAct({
        messages: input.messages.map(asChat),
        currentPubkey: input.currentPubkey,
        orgAgentPubkey: input.orgAgentPubkey,
        shaperPubkeys: shapers ? shapers.shapers : null,
        items,
        handledIds: handledRef.current,
        room: input.room,
      });
    }
    if (act === "wait" || act === null) return;
    if (
      act.kind === "project" ||
      act.kind === "dri" ||
      act.kind === "revise" ||
      act.kind === "removeProposal"
    ) {
      remember(act.agentEventId);
      return;
    }
    const command =
      act.kind === "done"
        ? buildIoDone({ item: act.itemId })
        : act.kind === "ticket"
          ? buildIoTicketCreate({
              parent: act.parentId,
              title: act.title,
              brief: act.brief,
              dueAt: act.dueAt,
              offerTo: act.offerTo,
            })
          : act.kind === "profile"
            ? buildIoProfileSet({
                about: act.about,
                skills: act.skills,
                socials: act.socials,
                openLimit: act.openLimit,
              })
            : buildIoWithdraw({ item: act.itemId, why: "Removed in chat." });
    const eventId = act.agentEventId;
    if (!claim(eventId)) return;
    inflight.current = eventId;
    void publishOrgCommand(command)
      .then(() => {
        remember(eventId);
      })
      .catch((error: unknown) => {
        console.error("Org. Agent could not record that", error);
        remember(eventId);
      })
      .finally(() => {
        inflight.current = null;
      });
  }, [
    input.currentPubkey,
    input.messages,
    input.orgAgentPubkey,
    input.room,
    overviewEvents,
    remember,
    workEvents,
    workLoading,
  ]);
}

function chatMessage(event: RelayEvent): TimelineMessage {
  return {
    id: event.id,
    createdAt: event.created_at,
    pubkey: event.pubkey,
    signerPubkey: event.pubkey,
    author: "",
    body: event.content,
    time: "",
    depth: 0,
    kind: event.kind,
    tags: event.tags,
    pending: event.pending,
  };
}

/**
 * Signs Org. Agent acts in `#shapers` even after you leave the channel.
 * The channel pane still signs the room you are looking at.
 */
export function OrgChatActs(): null {
  const { events: overview } = useOverviewEvents();
  const room = React.useMemo(
    () => parseShapersState(overview)?.room ?? null,
    [overview],
  );
  const filters = React.useMemo(
    () =>
      room ? [{ kinds: [KIND_STREAM_MESSAGE], "#h": [room], limit: 80 }] : [],
    [room],
  );
  const { events } = useLiveDoorEvents(filters);
  const messages = React.useMemo(() => events.map(chatMessage), [events]);
  const currentPubkey = useIdentityQuery().data?.pubkey ?? null;
  const orgAgentPubkey = useOrgAgentPubkey();
  useActFromChat({
    messages,
    currentPubkey,
    orgAgentPubkey,
    room: "channel",
  });
  return null;
}

/** Mounted in every channel. The DM confirms in one step; other rooms leave the vote open. */
export function OrgActsFromChat(input: {
  messages: readonly TimelineMessage[];
  currentPubkey: string | null;
  orgAgentPubkey: string | null;
  room?: "dm" | "shapers" | "channel";
}): null {
  useActFromChat(input);
  return null;
}
