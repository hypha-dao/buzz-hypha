import * as React from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import { relayClient } from "@/shared/api/relayClient";
import type { RelaySubscriptionFilter } from "@/shared/api/relayClientShared";
import { KIND_IO_SHAPERS } from "@/shared/constants/kinds";
import {
  type OrgAgentIdentity,
  parseOrgAgentFromShapers,
  SHAPERS_D_TAG,
} from "./orgAgent";

/** Query key for the community's `kind:39103` org-agent identity. */
export const orgAgentQueryKey = ["org", "shapers-agent"] as const;

const SHAPERS_FILTER: RelaySubscriptionFilter = {
  kinds: [KIND_IO_SHAPERS],
  "#d": [SHAPERS_D_TAG],
  limit: 1,
};

/**
 * Fetch the relay-signed `kind:39103` and read `agent` / `agent_hosted` out
 * of it. The relay keeps one `39103` per community (addressable on
 * `d=shapers`) and refuses client-published state kinds, so the newest event
 * is authoritative. A community that has not run the `50001` bootstrap has no
 * `39103` at all — that is "no org agent", not an error.
 */
export async function fetchOrgAgentIdentity(): Promise<OrgAgentIdentity> {
  const events = await relayClient.fetchEvents(SHAPERS_FILTER);
  return parseOrgAgentFromShapers(events);
}

/**
 * The community's org agent identity. The query client is keyed per
 * community (`App.tsx`), so this never leaks across a community switch.
 */
export function useOrgAgentQuery() {
  return useQuery<OrgAgentIdentity>({
    queryKey: orgAgentQueryKey,
    queryFn: fetchOrgAgentIdentity,
    // `39103` changes only on bootstrap and on a passed `shapers/agent`
    // proposal; the live subscription below invalidates on both.
    staleTime: 5 * 60_000,
  });
}

/** The org agent's pubkey, or null while loading or when none is set. */
export function useOrgAgentPubkey(): string | null {
  return useOrgAgentQuery().data?.pubkey ?? null;
}

/**
 * Keep the org-agent identity current: invalidate on every live `39103`
 * `d=shapers` and on relay reconnect (a replacement published while offline
 * does not replay through the live REQ). Mount once near the app root.
 */
export function useOrgAgentLiveUpdates(): void {
  const queryClient = useQueryClient();

  React.useEffect(() => {
    let disposed = false;
    let dispose: (() => void) | undefined;
    const invalidate = () => {
      void queryClient.invalidateQueries({ queryKey: orgAgentQueryKey });
    };

    void relayClient
      .subscribeLive({ ...SHAPERS_FILTER, limit: 0 }, invalidate)
      .then((unsubscribe) => {
        if (disposed) {
          void unsubscribe();
        } else {
          dispose = () => {
            void unsubscribe();
          };
        }
      })
      .catch((error) => {
        console.error("Failed to subscribe to the org agent identity", error);
      });

    const unsubReconnect = relayClient.subscribeToReconnects(invalidate);

    return () => {
      disposed = true;
      unsubReconnect();
      dispose?.();
    };
  }, [queryClient]);
}
