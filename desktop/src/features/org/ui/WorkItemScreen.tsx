import * as React from "react";

import { useLiveDoorEvents, useWorkItemEvents } from "@/features/org/hooks";
import {
  KIND_IO_DRAFT,
  KIND_IO_DRAFT_OUTCOME,
  KIND_IO_HEALTH,
  KIND_IO_WORK_ITEM,
  KIND_IO_WORK_PROMPT,
  KIND_PROJECT_ANNOUNCEMENT,
} from "@/shared/constants/kinds";

import { ORG_HISTORY_LIMIT } from "../hooks/filters";
import { TAG_ITEM } from "../tags";
import {
  childrenOf,
  itemById,
  latestHealth,
  latestWorkPrompt,
  linkedRepositories,
  openTicketDrafts,
  parseOfferedTicket,
  projectCoordinate,
  withDraftDetail,
} from "../work/model";
import { ORG_EMPTY_NOT_SET_YET, OrgDoorScreen } from "./OrgDoorScreen";
import { WorkItemView } from "./work/WorkItemView";

/** Item page — brief, holder, dates, breadcrumb, health (D-3). */
export function WorkItemScreen({ itemId }: { itemId: string }) {
  const { events, isLoading } = useWorkItemEvents(itemId);
  const draft =
    events.find(
      (event) => event.id === itemId && event.kind === KIND_IO_DRAFT,
    ) ?? null;
  const parsed =
    itemById(events, itemId) ?? (draft ? parseOfferedTicket(draft) : null);
  const sourceDraftId = parsed?.sourceDraftId ?? null;
  const parentId = parsed?.parent ?? "";
  const parentFilters = React.useMemo(
    () =>
      parentId
        ? [
            {
              kinds: [KIND_IO_WORK_ITEM],
              "#d": [parentId],
              limit: ORG_HISTORY_LIMIT,
            },
          ]
        : [],
    [parentId],
  );
  const healthFilters = React.useMemo(
    () =>
      itemId
        ? [
            {
              kinds: [KIND_IO_HEALTH],
              [`#${TAG_ITEM}`]: [itemId],
              limit: ORG_HISTORY_LIMIT,
            },
          ]
        : [],
    [itemId],
  );
  const projectRef = parsed?.home?.project ?? null;
  const projectFilters = React.useMemo(() => {
    const coordinate = projectCoordinate(projectRef);
    if (!coordinate) return [];
    return [
      {
        kinds: [KIND_PROJECT_ANNOUNCEMENT],
        authors: [coordinate.owner],
        "#d": [coordinate.slug],
        limit: 1,
      },
    ];
  }, [projectRef]);
  const promptIds = React.useMemo(() => {
    const ids = [itemId];
    if (sourceDraftId && sourceDraftId !== itemId) ids.push(sourceDraftId);
    return ids;
  }, [itemId, sourceDraftId]);
  const promptFilters = React.useMemo(
    () =>
      parsed?.type === "ticket"
        ? [
            {
              kinds: [KIND_IO_WORK_PROMPT],
              [`#${TAG_ITEM}`]: promptIds,
              limit: ORG_HISTORY_LIMIT,
            },
          ]
        : [],
    [parsed?.type, promptIds],
  );
  const sourceDraftFilters = React.useMemo(
    () =>
      sourceDraftId && sourceDraftId !== itemId
        ? [
            {
              ids: [sourceDraftId],
              kinds: [KIND_IO_DRAFT],
              limit: 1,
            },
          ]
        : [],
    [sourceDraftId, itemId],
  );
  const draftIds = React.useMemo(
    () =>
      events
        .filter((event) => event.kind === KIND_IO_DRAFT)
        .map((event) => event.id)
        .slice(0, 128),
    [events],
  );
  const outcomeFilters = React.useMemo(
    () =>
      draftIds.length > 0
        ? [
            {
              kinds: [KIND_IO_DRAFT_OUTCOME],
              "#d": draftIds,
              limit: ORG_HISTORY_LIMIT,
            },
          ]
        : [],
    [draftIds],
  );
  const parentEvents = useLiveDoorEvents(parentFilters);
  const healthEvents = useLiveDoorEvents(healthFilters);
  const projectEvents = useLiveDoorEvents(projectFilters);
  const promptEvents = useLiveDoorEvents(promptFilters);
  const sourceDraftEvents = useLiveDoorEvents(sourceDraftFilters);
  const outcomeEvents = useLiveDoorEvents(outcomeFilters);
  const sourceDraft = sourceDraftEvents.events.find(
    (event) => event.id === sourceDraftId && event.kind === KIND_IO_DRAFT,
  );
  const offered = sourceDraft ? parseOfferedTicket(sourceDraft) : null;
  const item = parsed ? withDraftDetail(parsed, offered) : null;
  const parent = parentId ? itemById(parentEvents.events, parentId) : null;
  const liveKids = childrenOf(events, item?.id ?? itemId);
  const kids = [
    ...liveKids,
    ...openTicketDrafts(
      events,
      outcomeEvents.events,
      item?.id ?? itemId,
      liveKids,
    ),
  ];
  const health = latestHealth(healthEvents.events, itemId);
  const repositories = linkedRepositories(projectEvents.events[0]?.tags ?? []);

  if (!isLoading && !item) {
    return (
      <OrgDoorScreen
        empty={ORG_EMPTY_NOT_SET_YET}
        testId="org-work-item"
        title="Work"
      />
    );
  }

  return (
    <OrgDoorScreen testId="org-work-item" title="Work">
      <div className="min-h-0 flex-1 overflow-y-auto">
        {item ? (
          <WorkItemView
            childItems={kids}
            draft={draft}
            health={health}
            item={item}
            parent={parent}
            prompt={latestWorkPrompt(promptEvents.events, promptIds)}
            repositories={repositories}
          />
        ) : null}
      </div>
    </OrgDoorScreen>
  );
}
