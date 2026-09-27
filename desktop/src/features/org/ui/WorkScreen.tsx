import { useWorkEvents } from "@/features/org/hooks";

import { assembleWorkDoor } from "../work/model";
import { ORG_EMPTY_NOTHING_NEEDS_YOU, OrgDoorScreen } from "./OrgDoorScreen";
import { WorkTree } from "./work/WorkTree";

/** Work door — tree from `39101`, roots + one level (D-3). */
export function WorkScreen() {
  const { events, isLoading } = useWorkEvents();
  const nodes = assembleWorkDoor(events);

  if (!isLoading && nodes.length === 0) {
    return (
      <OrgDoorScreen
        empty={ORG_EMPTY_NOTHING_NEEDS_YOU}
        testId="org-work"
        title="Work"
      />
    );
  }

  return (
    <OrgDoorScreen testId="org-work" title="Work">
      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {nodes.length > 0 ? <WorkTree nodes={nodes} /> : null}
      </div>
    </OrgDoorScreen>
  );
}
