/**
 * The drafts a member asked the org agent for, across every chat.
 *
 * Each chat shows its own open drafts (see `openChatDrafts`). My Work lists
 * the same ones, room by room, where the member is the one who asked.
 */

import {
  openChatDrafts,
  type ChatDraft,
  type ChatDraftMessage,
} from "./chatDraft";

export type OwnDraftMessage = ChatDraftMessage & {
  channelId: string | null;
};

export type OwnDraft = {
  draft: ChatDraft;
  channelId: string;
};

/** The `h` tag: the channel or DM a chat message was posted in. */
export function channelOfTags(
  tags: readonly (readonly string[])[] | undefined,
): string | null {
  const value = tags?.find((tag) => tag[0] === "h")?.[1]?.trim();
  return value ? value : null;
}

/**
 * Open drafts this member asked for, newest first. Each room is read on its
 * own, so a draft here is the same one its chat shows.
 */
export function ownChatDrafts(
  messages: readonly OwnDraftMessage[],
  orgAgentPubkey: string,
  viewer: string,
  closedIds: ReadonlySet<string>,
): OwnDraft[] {
  const me = viewer.trim().toLowerCase();
  const rooms = new Map<string, OwnDraftMessage[]>();
  for (const message of messages) {
    if (!message.channelId) continue;
    const room = rooms.get(message.channelId) ?? [];
    room.push(message);
    rooms.set(message.channelId, room);
  }
  const found: OwnDraft[] = [];
  for (const [channelId, roomMessages] of rooms) {
    for (const draft of openChatDrafts(
      roomMessages,
      orgAgentPubkey,
      closedIds,
    )) {
      if (draft.from === me) found.push({ draft, channelId });
    }
  }
  return found.sort(
    (left, right) =>
      right.draft.createdAt - left.draft.createdAt ||
      left.draft.messageId.localeCompare(right.draft.messageId),
  );
}

/** A DRI draft is signed by whoever asked; every other proposal by a Shaper. */
export function draftPublishableBy(
  draft: ChatDraft,
  viewer: string | null,
  shapers: readonly string[] | null | undefined,
): boolean {
  if (!viewer) return false;
  const me = viewer.trim().toLowerCase();
  if (draft.kind === "dri") return draft.from === me;
  return (shapers ?? []).some((pubkey) => pubkey.trim().toLowerCase() === me);
}

/** The person a draft names, for its label. */
export function draftPersonPubkey(draft: ChatDraft): string | null {
  switch (draft.kind) {
    case "project":
      return draft.suggestedDri;
    case "dri":
    case "shapers-add":
    case "shapers-remove":
    case "shapers-agent":
      return draft.pubkey;
    default:
      return null;
  }
}

/** The work item a draft is about, when it names one. */
export function draftItemId(draft: ChatDraft): string | null {
  return draft.kind === "dri" || draft.kind === "remove-project"
    ? draft.itemId
    : null;
}
