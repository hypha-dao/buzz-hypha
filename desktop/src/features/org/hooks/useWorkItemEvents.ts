import { useLiveDoorEvents } from "./useLiveReq";
import { workItemFilters } from "./filters";

/** Item page — Protocol §6.5. */
export function useWorkItemEvents(itemId: string) {
  return useLiveDoorEvents(itemId ? workItemFilters(itemId) : []);
}
