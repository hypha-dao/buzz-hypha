import * as React from "react";

import { KIND_IO_WORK_ITEM } from "@/shared/constants/kinds";

import { useLiveDoorEvents } from "./useLiveReq";
import { workFilters } from "./filters";

function itemIdsFromWorkEvents(
  events: readonly { kind: number; tags: string[][] }[],
): string[] {
  const ids = new Set<string>();
  for (const event of events) {
    if (event.kind !== KIND_IO_WORK_ITEM) continue;
    const d = event.tags.find((tag) => tag[0] === "d")?.[1];
    if (d) ids.add(d);
  }
  return [...ids].sort();
}

/** Work door — Protocol §6.5. Health `#i` joins once the tree has ids. */
export function useWorkEvents() {
  const [itemIds, setItemIds] = React.useState<string[]>([]);
  const result = useLiveDoorEvents(workFilters(itemIds));

  React.useEffect(() => {
    const next = itemIdsFromWorkEvents(result.events);
    setItemIds((current) =>
      current.length === next.length && current.every((id, i) => id === next[i])
        ? current
        : next,
    );
  }, [result.events]);

  return result;
}
