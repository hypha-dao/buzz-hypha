/**
 * Backfill/live overlap for a door's REQ set (AGENTS.md review-proven rule 2).
 *
 * Subscribe live first, then fetch history. Events that arrive in the gap
 * are kept. A completing in-flight load is fenced by generation so a stale
 * fetch cannot overwrite a newer subscription. Reconnect re-fetches history
 * against the still-live subscription (the same overlap).
 */

import * as React from "react";

import { relayClient } from "@/shared/api/relayClient";
import type { RelaySubscriptionFilter } from "@/shared/api/relayClientShared";
import type { RelayEvent } from "@/shared/api/types";

export type LiveReqClient = {
  fetchEvents: (filter: RelaySubscriptionFilter) => Promise<RelayEvent[]>;
  subscribeLive: (
    filter: RelaySubscriptionFilter,
    onEvent: (event: RelayEvent) => void,
  ) => Promise<() => void>;
  subscribeToReconnects?: (listener: () => void) => () => void;
};

const defaultClient: LiveReqClient = {
  fetchEvents: (filter) => relayClient.fetchEvents(filter),
  subscribeLive: (filter, onEvent) => relayClient.subscribeLive(filter, onEvent),
  subscribeToReconnects: (listener) => relayClient.subscribeToReconnects(listener),
};

export type OverlappingLoad = {
  events: RelayEvent[];
  unsubscribe: () => void;
};

function liveFilter(filter: RelaySubscriptionFilter): RelaySubscriptionFilter {
  return { ...filter, limit: 0 };
}

function mergeById(
  pages: readonly RelayEvent[][],
  live: ReadonlyMap<string, RelayEvent>,
): RelayEvent[] {
  const byId = new Map<string, RelayEvent>();
  for (const page of pages) {
    for (const event of page) byId.set(event.id, event);
  }
  for (const event of live.values()) byId.set(event.id, event);
  return [...byId.values()];
}

/**
 * Start every live REQ, then fetch every history page, then merge.
 * `onLiveEvent` fires for events that arrive after subscribe (including
 * during the history fetch). The production hooks pass this to update state.
 */
export async function loadOverlapping(
  filters: readonly RelaySubscriptionFilter[],
  client: LiveReqClient,
  onLiveEvent?: (event: RelayEvent) => void,
): Promise<OverlappingLoad> {
  const live = new Map<string, RelayEvent>();
  const unsubscribes: Array<() => void> = [];
  const unsubscribeAll = () => {
    for (const unsubscribe of unsubscribes) {
      void unsubscribe();
    }
  };

  try {
    for (const filter of filters) {
      const unsubscribe = await client.subscribeLive(
        liveFilter(filter),
        (event) => {
          live.set(event.id, event);
          onLiveEvent?.(event);
        },
      );
      unsubscribes.push(unsubscribe);
    }

    const pages = await Promise.all(
      filters.map((filter) => client.fetchEvents(filter)),
    );

    return {
      events: mergeById(pages, live),
      unsubscribe: unsubscribeAll,
    };
  } catch (error) {
    unsubscribeAll();
    throw error;
  }
}

export type LiveDoorEvents = {
  events: RelayEvent[];
  isLoading: boolean;
};

/**
 * One live REQ set for a door. `filters` is the Protocol §6.5 list; an empty
 * list (identity not ready) subscribes to nothing.
 */
export function useLiveDoorEvents(
  filters: readonly RelaySubscriptionFilter[],
  client: LiveReqClient = defaultClient,
): LiveDoorEvents {
  const [events, setEvents] = React.useState<RelayEvent[]>([]);
  const [isLoading, setIsLoading] = React.useState(filters.length > 0);
  const filterKey = JSON.stringify(filters);

  React.useEffect(() => {
    const parsed = JSON.parse(filterKey) as RelaySubscriptionFilter[];
    if (parsed.length === 0) {
      setEvents([]);
      setIsLoading(false);
      return;
    }

    let disposed = false;
    let generation = 0;
    let unsubscribe: (() => void) | undefined;
    const live = new Map<string, RelayEvent>();

    const applyLive = (event: RelayEvent) => {
      live.set(event.id, event);
      if (!disposed) {
        setEvents((current) => {
          const next = new Map(current.map((item) => [item.id, item]));
          next.set(event.id, event);
          return [...next.values()];
        });
      }
    };

    const load = () => {
      const thisGeneration = ++generation;
      void loadOverlapping(parsed, client, applyLive)
        .then((result) => {
          if (disposed || thisGeneration !== generation) {
            result.unsubscribe();
            return;
          }
          unsubscribe = result.unsubscribe;
          setEvents(result.events);
          setIsLoading(false);
        })
        .catch((error) => {
          console.error("Failed to subscribe to org door events", error);
          if (!disposed && thisGeneration === generation) setIsLoading(false);
        });
    };

    setIsLoading(true);
    load();

    const unsubReconnect = client.subscribeToReconnects?.(() => {
      if (disposed) return;
      void Promise.all(parsed.map((filter) => client.fetchEvents(filter)))
        .then((pages) => {
          if (disposed) return;
          setEvents(mergeById(pages, live));
        })
        .catch((error) => {
          console.error("Failed to refresh org door events after reconnect", error);
        });
    });

    return () => {
      disposed = true;
      unsubReconnect?.();
      unsubscribe?.();
    };
  }, [client, filterKey]);

  return { events, isLoading };
}
