import * as React from "react";

import { useLiveDoorEvents, useWorkItemEvents } from "@/features/org/hooks";
import {
  KIND_IO_HEALTH,
  KIND_IO_WORK_ITEM,
  KIND_PROJECT_ANNOUNCEMENT,
} from "@/shared/constants/kinds";

import { ORG_HISTORY_LIMIT } from "../hooks/filters";
import { TAG_ITEM } from "../tags";
import {
  childrenOf,
  itemById,
  latestHealth,
  linkedRepositories,
  projectCoordinate,
} from "../work/model";
import { ORG_EMPTY_NOT_SET_YET, OrgDoorScreen } from "./OrgDoorScreen";
import { WorkItemView } from "./work/WorkItemView";

/** Item page — brief, holder, dates, breadcrumb, health (D-3). */
export function WorkItemScreen({ itemId }: { itemId: string }) {
  const { events, isLoading } = useWorkItemEvents(itemId);
  const item = itemById(events, itemId);
  const parentId = item?.parent ?? "";
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
  const projectRef = item?.home?.project ?? null;
  const projectFilters = React.useMemo(() => {
    const parsed = projectCoordinate(projectRef);
    if (!parsed) return [];
    return [
      {
        kinds: [KIND_PROJECT_ANNOUNCEMENT],
        authors: [parsed.owner],
        "#d": [parsed.slug],
        limit: 1,
      },
    ];
  }, [projectRef]);
  const parentEvents = useLiveDoorEvents(parentFilters);
  const healthEvents = useLiveDoorEvents(healthFilters);
  const projectEvents = useLiveDoorEvents(projectFilters);
  const parent = parentId ? itemById(parentEvents.events, parentId) : null;
  const kids = childrenOf(events, itemId);
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
            health={health}
            item={item}
            parent={parent}
            repositories={repositories}
          />
        ) : null}
      </div>
    </OrgDoorScreen>
  );
}
