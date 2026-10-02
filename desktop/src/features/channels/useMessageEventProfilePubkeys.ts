import * as React from "react";

import {
  collectMessageAuthorPubkeys,
  collectMessageMentionPubkeys,
  collectReactionActorPubkeys,
} from "@/features/messages/lib/formatTimelineMessages";
import { holderPubkeysFromTags } from "@/features/org/chatDraft";
import type { RelayEvent } from "@/shared/api/types";

export function useMessageEventProfilePubkeys(
  messages: RelayEvent[],
  threadReplies: RelayEvent[],
  relaySelfPubkey: string | null | undefined,
) {
  return React.useMemo(() => {
    const events = [...messages, ...threadReplies];
    return [
      ...new Set([
        ...collectMessageAuthorPubkeys(events, relaySelfPubkey),
        ...collectMessageMentionPubkeys(events),
        ...collectReactionActorPubkeys(events, relaySelfPubkey),
        ...events.flatMap((event) => holderPubkeysFromTags(event.tags)),
      ]),
    ];
  }, [messages, relaySelfPubkey, threadReplies]);
}
