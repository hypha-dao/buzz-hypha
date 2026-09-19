import { useMyWorkEvents } from "@/features/org/hooks";

import { ORG_EMPTY_NOTHING_NEEDS_YOU, OrgDoorScreen } from "./OrgDoorScreen";

/** My Work skeleton — D-2 paints the three columns. */
export function MyWorkScreen() {
  useMyWorkEvents();
  return (
    <OrgDoorScreen
      empty={ORG_EMPTY_NOTHING_NEEDS_YOU}
      testId="org-my-work"
      title="My Work"
    />
  );
}
