import * as React from "react";

import { useIdentityQuery } from "@/shared/api/hooks";
import type { RelayEvent } from "@/shared/api/types";
import { KIND_IO_SHAPERS } from "@/shared/constants/kinds";
import { normalizePubkey } from "@/shared/lib/pubkey";

import { SHAPERS_D_TAG } from "../tags";
import { myWorkFilters } from "./filters";
import { useLiveDoorEvents } from "./useLiveReq";

export function viewerIsShaper(
  events: readonly Pick<RelayEvent, "kind" | "tags" | "content" | "created_at">[],
  pubkey: string,
): boolean {
  const newest = events
    .filter(
      (event) =>
        event.kind === KIND_IO_SHAPERS &&
        event.tags.some((tag) => tag[0] === "d" && tag[1] === SHAPERS_D_TAG),
    )
    .sort((left, right) => right.created_at - left.created_at)[0];
  if (!newest) return false;
  try {
    const content = JSON.parse(newest.content) as { shapers?: unknown };
    if (!Array.isArray(content.shapers)) return false;
    const me = normalizePubkey(pubkey);
    return content.shapers.some(
      (entry) => typeof entry === "string" && normalizePubkey(entry) === me,
    );
  } catch {
    return false;
  }
}

/** My Work door — Protocol §6.5. Shaper drafts join once `39103` names me. */
export function useMyWorkEvents() {
  const pubkey = useIdentityQuery().data?.pubkey ?? null;
  const [includeShaperDrafts, setIncludeShaperDrafts] = React.useState(false);
  const filters = React.useMemo(
    () => (pubkey ? myWorkFilters(pubkey, includeShaperDrafts) : []),
    [includeShaperDrafts, pubkey],
  );
  const result = useLiveDoorEvents(filters);

  React.useEffect(() => {
    if (!pubkey || includeShaperDrafts) return;
    if (viewerIsShaper(result.events, pubkey)) setIncludeShaperDrafts(true);
  }, [includeShaperDrafts, pubkey, result.events]);

  return result;
}
