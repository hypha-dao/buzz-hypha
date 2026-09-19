import { useWorkEvents } from "@/features/org/hooks";

import { ORG_EMPTY_NOTHING_NEEDS_YOU, OrgDoorScreen } from "./OrgDoorScreen";

/** Work skeleton — D-3 paints the tree. */
export function WorkScreen() {
  useWorkEvents();
  return (
    <OrgDoorScreen
      empty={ORG_EMPTY_NOTHING_NEEDS_YOU}
      testId="org-work"
      title="Work"
    />
  );
}
