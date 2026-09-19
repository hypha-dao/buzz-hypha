import { useLiveDoorEvents } from "./useLiveReq";
import { overviewFilters } from "./filters";

/** Overview door — Protocol §6.5. */
export function useOverviewEvents() {
  return useLiveDoorEvents(overviewFilters());
}
