import * as React from "react";
import {
  isChannelCreatedSystemMessage,
  isWelcomeSetupSystemMessage,
} from "@/features/channels/ui/ChannelPane.helpers";
import type { ChannelPaneProps } from "@/features/channels/ui/ChannelPane.types";
import { buildMainTimelineEntries } from "@/features/messages/lib/threadPanel";
import { getRecentMentionPubkeys } from "@/features/messages/lib/recentMentionPubkeys";
import { isWelcomeExperienceChannel } from "@/features/onboarding/welcome";
import { isOrgAgentDm } from "@/features/org/orgAgent";
import { isCannedOrgAgentOpening } from "@/features/org/orgAgentOpening";

type ChannelPaneMessagesOptions = Pick<
  ChannelPaneProps,
  | "activeChannel"
  | "currentPubkey"
  | "messages"
  | "profiles"
  | "threadSummaries"
> & {
  isHuddleTranscript: boolean;
  orgAgentPubkey?: string | null;
};

export function useChannelPaneMessages({
  activeChannel,
  currentPubkey,
  isHuddleTranscript,
  messages,
  orgAgentPubkey = null,
  profiles,
  threadSummaries,
}: ChannelPaneMessagesOptions) {
  const visibleMessages = React.useMemo(() => {
    const withoutWelcomeSetup = isWelcomeExperienceChannel(activeChannel)
      ? messages.filter((message) => !isWelcomeSetupSystemMessage(message))
      : messages;
    const withoutCannedOpening =
      activeChannel &&
      isOrgAgentDm(activeChannel, orgAgentPubkey, currentPubkey ?? null)
        ? withoutWelcomeSetup.filter(
            (message) => !isCannedOrgAgentOpening(message, orgAgentPubkey),
          )
        : withoutWelcomeSetup;

    return isHuddleTranscript
      ? withoutCannedOpening.filter(
          (message) => !isChannelCreatedSystemMessage(message),
        )
      : withoutCannedOpening;
  }, [
    activeChannel,
    currentPubkey,
    isHuddleTranscript,
    messages,
    orgAgentPubkey,
  ]);

  const mainTimelineEntries = React.useMemo(
    () =>
      isHuddleTranscript
        ? visibleMessages.map((message) => ({ message, summary: null }))
        : buildMainTimelineEntries(
            visibleMessages,
            new Set(),
            threadSummaries,
            profiles,
          ),
    [isHuddleTranscript, profiles, threadSummaries, visibleMessages],
  );

  const recentMentionPubkeys = React.useMemo(
    () => getRecentMentionPubkeys(messages, activeChannel?.channelType),
    [activeChannel?.channelType, messages],
  );

  return {
    mainTimelineEntries,
    recentMentions: recentMentionPubkeys,
    visibleMessages,
  };
}
