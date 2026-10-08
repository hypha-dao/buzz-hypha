import * as React from "react";

import { channelNamesMatch } from "@/features/channels/lib/canonicalChannelName";
import type { MainTimelineEntry } from "@/features/messages/lib/threadPanel";
import type { TimelineMessage } from "@/features/messages/types";
import {
  nameHoldersInChat,
  shortProposalAnnouncement,
  stripProposalOpenLink,
} from "@/features/org/chatDraft";
import { isOrgAgentDm } from "@/features/org/orgAgent";
import {
  mergeTypingPubkey,
  orgAgentWillAnswer,
  previousMessageIsOrgAgentQuestion,
  withOrgAgentTypingProfile,
} from "@/features/org/orgAgentTyping";
import { useOrgAgentPubkey } from "@/features/org/useOrgAgent";
import { useOrgAgentTypingAck } from "@/features/org/useOrgAgentTypingAck";
import type { UserProfileLookup } from "@/features/profile/lib/identity";
import type { Channel } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export type OrgChatRoom = "dm" | "channel";

/**
 * Org-agent chat wired into a channel pane: proposal footers, draft
 * sentences, typing, and the names shown in place of holder prefixes.
 */
export function useOrgChannelChat({
  activeChannel,
  activeChannelId,
  currentPubkey,
  messages,
  profiles,
  threadHeadMessage,
  threadMessages,
  threadTypingPubkeys,
  typingPubkeys,
}: {
  activeChannel: Channel | null;
  activeChannelId: string | null;
  currentPubkey: string | undefined;
  messages: TimelineMessage[];
  profiles: UserProfileLookup | undefined;
  threadHeadMessage: TimelineMessage | null;
  threadMessages: MainTimelineEntry[];
  threadTypingPubkeys: string[];
  typingPubkeys: string[];
}) {
  const orgAgentPubkey = useOrgAgentPubkey();
  const orgAgentDm =
    activeChannel !== null &&
    isOrgAgentDm(activeChannel, orgAgentPubkey, currentPubkey ?? null);
  const shapersRoom =
    activeChannel !== null &&
    activeChannel.channelType !== "dm" &&
    channelNamesMatch(activeChannel.name, "shapers");
  const [shapersProposalFooters, setShapersProposalFooters] = React.useState<{
    signature: string;
    footers: Record<string, React.ReactNode>;
  }>({ signature: "", footers: {} });
  const onShapersProposalFooters = React.useCallback(
    (signature: string, footers: Record<string, React.ReactNode>) => {
      setShapersProposalFooters((current) =>
        current.signature === signature ? current : { signature, footers },
      );
    },
    [],
  );
  const [setupFooters, setSetupFooters] = React.useState<{
    signature: string;
    footers: Record<string, React.ReactNode>;
  }>({ signature: "", footers: {} });
  const onSetupFooters = React.useCallback(
    (signature: string, footers: Record<string, React.ReactNode>) => {
      setSetupFooters((current) =>
        current.signature === signature ? current : { signature, footers },
      );
    },
    [],
  );
  const [draftFooters, setDraftFooters] = React.useState<{
    signature: string;
    footers: Record<string, React.ReactNode>;
  }>({ signature: "", footers: {} });
  const onDraftFooters = React.useCallback(
    (signature: string, footers: Record<string, React.ReactNode>) => {
      setDraftFooters((current) =>
        current.signature === signature ? current : { signature, footers },
      );
    },
    [],
  );
  const [draftSentences, setDraftSentences] = React.useState<{
    signature: string;
    sentences: Record<string, string>;
  }>({ signature: "", sentences: {} });
  const onDraftSentences = React.useCallback(
    (signature: string, sentences: Record<string, string>) => {
      setDraftSentences((current) =>
        current.signature === signature ? current : { signature, sentences },
      );
    },
    [],
  );
  const orgChatRoom: OrgChatRoom | null = activeChannel
    ? orgAgentDm
      ? "dm"
      : "channel"
    : null;
  const proposalChat = shapersRoom || orgAgentDm;
  const messageFooters = React.useMemo(() => {
    const published = proposalChat ? shapersProposalFooters.footers : {};
    const drafts = orgChatRoom ? draftFooters.footers : {};
    const setup = orgAgentDm ? setupFooters.footers : {};
    const ids = new Set([
      ...Object.keys(published),
      ...Object.keys(drafts),
      ...Object.keys(setup),
    ]);
    if (ids.size === 0) return undefined;
    const merged: Record<string, React.ReactNode> = {};
    for (const id of ids) {
      merged[id] = React.createElement(
        React.Fragment,
        null,
        published[id] ? null : drafts[id],
        published[id],
        setup[id],
      );
    }
    return merged;
  }, [
    draftFooters.footers,
    orgAgentDm,
    orgChatRoom,
    proposalChat,
    setupFooters.footers,
    shapersProposalFooters.footers,
  ]);
  const {
    noteSendFailed: noteOrgAgentSendFailed,
    noteSendStarted: noteOrgAgentSendStarted,
    typingPubkey: orgAgentTypingPubkey,
  } = useOrgAgentTypingAck({
    answersEveryLine: orgAgentDm,
    channelId: activeChannelId,
    enabled: orgAgentPubkey !== null,
    messages,
    orgAgentPubkey,
  });
  const orgAgentAnswerRef = React.useRef({
    messages,
    orgAgentDm,
    orgAgentPubkey,
  });
  orgAgentAnswerRef.current = { messages, orgAgentDm, orgAgentPubkey };
  const ackOrgAgentSend = React.useCallback(
    (content: string, mentionPubkeys: string[]) => {
      const current = orgAgentAnswerRef.current;
      if (!current.orgAgentPubkey) return null;
      const agent = normalizePubkey(current.orgAgentPubkey);
      const mentionsAgent = mentionPubkeys.some(
        (pubkey) => normalizePubkey(pubkey) === agent,
      );
      if (
        !orgAgentWillAnswer({
          answersEveryLine: current.orgAgentDm,
          body: content,
          followsAgentQuestion: previousMessageIsOrgAgentQuestion(
            current.messages,
            current.orgAgentPubkey,
          ),
          mentionsAgent,
        })
      ) {
        return null;
      }
      return noteOrgAgentSendStarted();
    },
    [noteOrgAgentSendStarted],
  );
  const composerTypingPubkeys = React.useMemo(
    () => mergeTypingPubkey(typingPubkeys, orgAgentTypingPubkey),
    [orgAgentTypingPubkey, typingPubkeys],
  );
  const threadComposerTypingPubkeys = React.useMemo(
    () => mergeTypingPubkey(threadTypingPubkeys, orgAgentTypingPubkey),
    [orgAgentTypingPubkey, threadTypingPubkeys],
  );
  const orgAgentTypingProfiles = React.useMemo(
    () =>
      withOrgAgentTypingProfile(
        profiles,
        orgAgentPubkey,
        orgAgentTypingPubkey !== null,
      ),
    [orgAgentPubkey, orgAgentTypingPubkey, profiles],
  );
  const displayMessages = React.useMemo(() => {
    let changed = false;
    const sentences = orgChatRoom ? draftSentences.sentences : {};
    const carded = shapersRoom ? shapersProposalFooters.footers : {};
    const next = messages.map((message) => {
      const unlinked =
        sentences[message.id] ?? stripProposalOpenLink(message.body);
      const stripped = carded[message.id]
        ? shortProposalAnnouncement(unlinked)
        : unlinked;
      const body = orgChatRoom
        ? nameHoldersInChat(stripped, message.tags, profiles)
        : stripped;
      if (body === message.body) return message;
      changed = true;
      return { ...message, body };
    });
    return changed ? next : messages;
  }, [
    draftSentences.sentences,
    messages,
    orgChatRoom,
    profiles,
    shapersProposalFooters.footers,
    shapersRoom,
  ]);
  const displayThreadMessages = React.useMemo(() => {
    if (!orgChatRoom) return threadMessages;
    let changed = false;
    const next = threadMessages.map((entry) => {
      const body = nameHoldersInChat(
        entry.message.body,
        entry.message.tags,
        profiles,
      );
      if (body === entry.message.body) return entry;
      changed = true;
      return { ...entry, message: { ...entry.message, body } };
    });
    return changed ? next : threadMessages;
  }, [orgChatRoom, profiles, threadMessages]);
  const displayThreadHead = React.useMemo(() => {
    if (!orgChatRoom || !threadHeadMessage) return threadHeadMessage;
    const body = nameHoldersInChat(
      threadHeadMessage.body,
      threadHeadMessage.tags,
      profiles,
    );
    if (body === threadHeadMessage.body) return threadHeadMessage;
    return { ...threadHeadMessage, body };
  }, [orgChatRoom, profiles, threadHeadMessage]);

  return {
    ackOrgAgentSend,
    composerTypingPubkeys,
    displayMessages,
    displayThreadHead,
    displayThreadMessages,
    messageFooters,
    noteOrgAgentSendFailed,
    onDraftFooters,
    onDraftSentences,
    onSetupFooters,
    onShapersProposalFooters,
    orgAgentPubkey,
    orgAgentTypingProfiles,
    orgChatRoom,
    shapersRoom,
    threadComposerTypingPubkeys,
  };
}
