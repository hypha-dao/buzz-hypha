import { useOverviewEvents } from "@/features/org/hooks";
import { useOrgCommandE2eBridge } from "@/features/org/useOrgCommands";

import { ORG_EMPTY_NOT_SET_YET, OrgDoorScreen } from "./OrgDoorScreen";

/** Overview skeleton — D-1 paints the direction cards. */
export function OverviewScreen() {
  useOverviewEvents();
  useOrgCommandE2eBridge();
  return (
    <OrgDoorScreen
      empty={ORG_EMPTY_NOT_SET_YET}
      testId="org-overview"
      title="Overview"
    />
  );
}
