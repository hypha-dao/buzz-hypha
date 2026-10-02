import * as React from "react";

import type { TimelineMessage } from "@/features/messages/types";
import {
  type OrgAgentTypingAck,
  latestOrgAgentMessage,
  orgAgentPromptFromFresh,
  orgAgentTypingActive,
  ORG_AGENT_TYPING_ACK_MS,
} from "@/features/org/orgAgentTyping";

/** A line already on screen when the channel opens is history, not a new prompt. */
const LIVE_PROMPT_SEC = 15;

/**
 * Local "Org. Agent is typing…" while that agent is answering this channel.
 * Starts when the member sends a line the agent will answer, and when such a
 * line arrives from anyone else. Clears when a newer agent message is on the
 * timeline, the send fails, the channel changes, or the ack times out.
 */
export function useOrgAgentTypingAck({
  answersEveryLine,
  channelId,
  enabled,
  messages,
  orgAgentPubkey,
}: {
  answersEveryLine: boolean;
  channelId: string | null;
  enabled: boolean;
  messages: readonly TimelineMessage[];
  orgAgentPubkey: string | null;
}) {
  const generationRef = React.useRef(0);
  const primedRef = React.useRef(false);
  const seenIdsRef = React.useRef<Set<string>>(new Set());
  const [ack, setAck] = React.useState<OrgAgentTypingAck | null>(null);
  const [ackChannelId, setAckChannelId] = React.useState(channelId);
  if (ackChannelId !== channelId) {
    setAckChannelId(channelId);
    setAck(null);
    primedRef.current = false;
    seenIdsRef.current = new Set();
  }
  const latestAgent = React.useMemo(
    () =>
      orgAgentPubkey ? latestOrgAgentMessage(messages, orgAgentPubkey) : null,
    [messages, orgAgentPubkey],
  );
  const latestAgentRef = React.useRef(latestAgent);
  latestAgentRef.current = latestAgent;

  const noteSendStarted = React.useCallback(() => {
    const generation = generationRef.current + 1;
    generationRef.current = generation;
    const now = Date.now();
    setAck({
      generation,
      startedAt: now,
      sentAtSec: Math.floor(now / 1000),
      baselineAgentMessageId: latestAgentRef.current?.id ?? null,
    });
    return generation;
  }, []);

  const noteSendFailed = React.useCallback((generation: number) => {
    setAck((current) => (current?.generation === generation ? null : current));
  }, []);

  React.useEffect(() => {
    if (!enabled || !orgAgentPubkey) {
      primedRef.current = false;
      seenIdsRef.current = new Set();
      return;
    }
    if (!primedRef.current) {
      primedRef.current = true;
      for (const message of messages) seenIdsRef.current.add(message.id);
      return;
    }
    const nowSec = Math.floor(Date.now() / 1000);
    const fresh: TimelineMessage[] = [];
    for (const message of messages) {
      if (seenIdsRef.current.has(message.id)) continue;
      seenIdsRef.current.add(message.id);
      if (nowSec - message.createdAt <= LIVE_PROMPT_SEC) fresh.push(message);
    }
    if (
      orgAgentPromptFromFresh(messages, fresh, orgAgentPubkey, answersEveryLine)
    ) {
      noteSendStarted();
    }
  }, [answersEveryLine, enabled, messages, noteSendStarted, orgAgentPubkey]);

  const latestAgentId = latestAgent?.id ?? null;
  const latestAgentCreatedAt = latestAgent?.createdAt ?? null;
  const replyLanded =
    ack !== null &&
    !orgAgentTypingActive({
      ack,
      enabled,
      now: Date.now(),
      latestAgentId,
      latestAgentCreatedAt,
    }) &&
    Date.now() - ack.startedAt < ORG_AGENT_TYPING_ACK_MS;

  React.useEffect(() => {
    if (!replyLanded) return;
    setAck((current) =>
      current?.generation === ack?.generation ? null : current,
    );
  }, [ack?.generation, replyLanded]);

  React.useEffect(() => {
    if (!ack) return;
    const remaining = ORG_AGENT_TYPING_ACK_MS - (Date.now() - ack.startedAt);
    if (remaining <= 0) {
      setAck((current) =>
        current?.generation === ack.generation ? null : current,
      );
      return;
    }
    const timer = window.setTimeout(() => {
      setAck((current) =>
        current?.generation === ack.generation ? null : current,
      );
    }, remaining);
    return () => window.clearTimeout(timer);
  }, [ack]);

  const typingPubkey =
    orgAgentPubkey &&
    orgAgentTypingActive({
      ack,
      enabled,
      now: Date.now(),
      latestAgentId,
      latestAgentCreatedAt,
    })
      ? orgAgentPubkey
      : null;

  return { noteSendFailed, noteSendStarted, typingPubkey };
}
