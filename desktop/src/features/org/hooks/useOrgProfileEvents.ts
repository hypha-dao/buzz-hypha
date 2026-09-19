import * as React from "react";

import type { RelayEvent } from "@/shared/api/types";

import {
  EMPTY_ORG_PROFILE,
  newestOrgProfile,
  type OrgProfile,
} from "../profile";
import { profileFilters } from "./filters";
import { useLiveDoorEvents } from "./useLiveReq";

/** Profile About & skills — Protocol §6.5 `{kinds:[39105], "#d":[pubkey]}`. */
export function useOrgProfileEvents(pubkey: string | null) {
  const filters = React.useMemo(
    () => (pubkey ? profileFilters(pubkey) : []),
    [pubkey],
  );
  return useLiveDoorEvents(filters);
}

export function useOrgProfile(pubkey: string | null): {
  events: RelayEvent[];
  isLoading: boolean;
  profile: OrgProfile;
} {
  const { events, isLoading } = useOrgProfileEvents(pubkey);
  const profile = React.useMemo(
    () => (pubkey ? newestOrgProfile(events, pubkey) : EMPTY_ORG_PROFILE),
    [events, pubkey],
  );
  return { events, isLoading, profile };
}
