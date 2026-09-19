import { useWorkItemEvents } from "@/features/org/hooks";

import { ORG_EMPTY_NOT_SET_YET, OrgDoorScreen } from "./OrgDoorScreen";

/** Item-page skeleton — D-3 paints brief, trail, health. */
export function WorkItemScreen({ itemId }: { itemId: string }) {
  useWorkItemEvents(itemId);
  return (
    <OrgDoorScreen
      empty={ORG_EMPTY_NOT_SET_YET}
      testId="org-work-item"
      title="Work"
    />
  );
}
