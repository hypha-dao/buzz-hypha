import * as React from "react";

import { useIdentityQuery } from "@/shared/api/hooks";
import { KIND_IO_WORK_ITEM } from "@/shared/constants/kinds";

import { useDraftLifecycle } from "../draftLifecycle";
import { channelOfTags, ownChatDrafts, type OwnDraft } from "../myDrafts";
import { useOrgAgentQuery } from "../useOrgAgent";
import { latestWorkItems, parseWorkItem } from "../work/model";
import { myDraftFilters, ORG_HISTORY_LIMIT } from "./filters";
import { useLiveDoorEvents } from "./useLiveReq";

/** Titles for the work items drafts name. Fetches nothing when none do. */
export function useWorkItemTitles(
  itemIds: readonly string[],
): ReadonlyMap<string, string> {
  const joined = [...new Set(itemIds)].sort().slice(0, 128).join(",");
  const filters = React.useMemo(
    () =>
      joined
        ? [
            {
              kinds: [KIND_IO_WORK_ITEM],
              "#d": joined.split(","),
              limit: ORG_HISTORY_LIMIT,
            },
          ]
        : [],
    [joined],
  );
  const { events } = useLiveDoorEvents(filters);
  return React.useMemo(() => {
    const titles = new Map<string, string>();
    for (const event of latestWorkItems(events).values()) {
      const item = parseWorkItem(event);
      if (item?.title) titles.set(item.id, item.title);
    }
    return titles;
  }, [events]);
}

export type MyDrafts = {
  drafts: OwnDraft[];
  isLoading: boolean;
  /** False when storage refused the write. */
  markPublished: (messageId: string) => boolean;
  /** False when storage refused the write. */
  markDeleted: (messageId: string) => boolean;
};

/** Open drafts the viewer asked the org agent for, in every chat. */
export function useMyDrafts(): MyDrafts {
  const viewer = useIdentityQuery().data?.pubkey ?? null;
  const agentQuery = useOrgAgentQuery();
  const agent = agentQuery.data?.pubkey ?? null;
  const filters = React.useMemo(
    () => (agent ? myDraftFilters(agent) : []),
    [agent],
  );
  const { events, isLoading } = useLiveDoorEvents(filters);
  const { closed, markPublished, markDeleted } = useDraftLifecycle(viewer);
  const drafts = React.useMemo(() => {
    if (!agent || !viewer) return [];
    const messages = events.map((event) => ({
      id: event.id,
      createdAt: event.created_at,
      pubkey: event.pubkey,
      tags: event.tags,
      channelId: channelOfTags(event.tags),
    }));
    return ownChatDrafts(messages, agent, viewer, closed);
  }, [agent, closed, events, viewer]);
  return {
    drafts,
    isLoading: agentQuery.isLoading || isLoading,
    markPublished,
    markDeleted,
  };
}
