/**
 * Backfill/live overlap for a door's REQ set (AGENTS.md review-proven rule 2).
 *
 * Subscribe live first, then fetch history. Events that arrive in the gap
 * are kept. A completing in-flight load is fenced by generation so a stale
 * fetch cannot overwrite a newer subscription. Reconnect re-fetches history
 * against the still-live subscription (the same overlap). Callers that pass
 * the same filters share one load.
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
  subscribeLive: (filter, onEvent) =>
    relayClient.subscribeLive(filter, onEvent),
  subscribeToReconnects: (listener) =>
    relayClient.subscribeToReconnects(listener),
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
 * Subscribes run together so one slow readiness wait does not delay the
 * others. `onLiveEvent` fires for events that arrive after subscribe
 * (including during the history fetch). `onHistoryPage` fires as each
 * history REQ completes, so a door can paint before the slowest filter
 * returns. The production hooks pass these to update state.
 */
export async function loadOverlapping(
  filters: readonly RelaySubscriptionFilter[],
  client: LiveReqClient,
  onLiveEvent?: (event: RelayEvent) => void,
  onHistoryPage?: (events: RelayEvent[]) => void,
): Promise<OverlappingLoad> {
  const live = new Map<string, RelayEvent>();
  const unsubscribes: Array<() => void> = [];
  const unsubscribeAll = () => {
    for (const unsubscribe of unsubscribes) {
      void unsubscribe();
    }
  };

  try {
    await Promise.all(
      filters.map(async (filter) => {
        const unsubscribe = await client.subscribeLive(
          liveFilter(filter),
          (event) => {
            live.set(event.id, event);
            onLiveEvent?.(event);
          },
        );
        unsubscribes.push(unsubscribe);
      }),
    );

    const pages = await Promise.all(
      filters.map(async (filter) => {
        const page = await client.fetchEvents(filter);
        onHistoryPage?.(page);
        return page;
      }),
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

const IDLE_SNAPSHOT: LiveDoorEvents = { events: [], isLoading: false };
const LOADING_SNAPSHOT: LiveDoorEvents = { events: [], isLoading: true };

type SharedDoor = {
  client: LiveReqClient;
  filters: RelaySubscriptionFilter[];
  generation: number;
  listeners: Set<() => void>;
  retain: number;
  snapshot: LiveDoorEvents;
  stop: (() => void) | undefined;
};

const doors = new Map<string, SharedDoor>();
let doorEpoch = 0;

/**
 * A filter change used to swap in an empty loading snapshot. Hooks that
 * derive the next filter from the current events (work health `#i`) then
 * cleared that derivation and fetched the old filter again. Hold the last
 * events until the new filter returns something, and drop them when the
 * community epoch changes.
 */
export function holdDoorSnapshot(
  held: LiveDoorEvents,
  heldEpoch: number,
  live: LiveDoorEvents,
  epoch: number,
): { held: LiveDoorEvents; heldEpoch: number; view: LiveDoorEvents } {
  if (heldEpoch !== epoch || !live.isLoading || live.events.length > 0) {
    return { held: live, heldEpoch: epoch, view: live };
  }
  if (held.events.length > 0) {
    return {
      held,
      heldEpoch,
      view: { events: held.events, isLoading: true },
    };
  }
  return { held: live, heldEpoch: epoch, view: live };
}

function filterKeyOf(filters: readonly RelaySubscriptionFilter[]): string {
  return JSON.stringify(filters);
}

function publishDoor(
  door: SharedDoor,
  events: RelayEvent[],
  isLoading: boolean,
): void {
  if (
    door.snapshot.events === events &&
    door.snapshot.isLoading === isLoading
  ) {
    return;
  }
  door.snapshot = { events, isLoading };
  for (const listener of door.listeners) listener();
}

function startSharedDoor(door: SharedDoor): () => void {
  const generation = ++door.generation;
  let disposed = false;
  let unsubscribe: (() => void) | undefined;
  const live = new Map<string, RelayEvent>();
  const current = () => !disposed && door.generation === generation;

  const paint = (events: RelayEvent[], isLoading: boolean) => {
    if (!current()) return;
    publishDoor(door, mergeById([events], live), isLoading);
  };

  void loadOverlapping(
    door.filters,
    door.client,
    (event) => {
      live.set(event.id, event);
      paint(door.snapshot.events, door.snapshot.isLoading);
    },
    (page) => {
      paint(mergeById([door.snapshot.events, page], live), true);
    },
  )
    .then((result) => {
      if (!current()) {
        result.unsubscribe();
        return;
      }
      unsubscribe = result.unsubscribe;
      paint(mergeById([result.events, door.snapshot.events], live), false);
    })
    .catch((error) => {
      console.error("Failed to subscribe to org door events", error);
      if (current()) publishDoor(door, door.snapshot.events, false);
    });

  const unsubReconnect = door.client.subscribeToReconnects?.(() => {
    if (!current()) return;
    void Promise.all(
      door.filters.map((filter) => door.client.fetchEvents(filter)),
    )
      .then((pages) => {
        if (!current()) return;
        publishDoor(door, mergeById(pages, live), false);
      })
      .catch((error) => {
        console.error(
          "Failed to refresh org door events after reconnect",
          error,
        );
      });
  });

  return () => {
    disposed = true;
    unsubReconnect?.();
    unsubscribe?.();
  };
}

/**
 * One in-flight load per filter set. Overview is read from the app shell and
 * from the Overview door; they share this so opening the door does not start
 * a second history fetch.
 */
export function subscribeSharedDoor(
  filters: readonly RelaySubscriptionFilter[],
  listener: () => void,
  client: LiveReqClient = defaultClient,
): () => void {
  const filterKey = filterKeyOf(filters);
  let door = doors.get(filterKey);
  if (!door) {
    door = {
      client,
      filters: [...filters],
      generation: 0,
      listeners: new Set(),
      retain: 0,
      snapshot: filters.length === 0 ? IDLE_SNAPSHOT : LOADING_SNAPSHOT,
      stop: undefined,
    };
    doors.set(filterKey, door);
  }
  door.listeners.add(listener);
  door.retain += 1;
  if (door.retain === 1 && door.filters.length > 0) {
    door.stop = startSharedDoor(door);
  }
  return () => {
    const current = doors.get(filterKey);
    if (!current) return;
    current.listeners.delete(listener);
    current.retain -= 1;
    if (current.retain === 0) {
      current.stop?.();
      current.stop = undefined;
      doors.delete(filterKey);
    }
  };
}

/** Latest snapshot for `filters`. Stable while nothing has changed. */
export function readSharedDoor(
  filters: readonly RelaySubscriptionFilter[],
): LiveDoorEvents {
  const door = doors.get(filterKeyOf(filters));
  if (door) return door.snapshot;
  return filters.length === 0 ? IDLE_SNAPSHOT : LOADING_SNAPSHOT;
}

/** Drop door caches when the community changes. Remount starts a fresh load. */
export function resetLiveDoorEvents(): void {
  doorEpoch += 1;
  for (const door of doors.values()) {
    door.stop?.();
    door.stop = undefined;
    door.generation += 1;
    door.snapshot =
      door.filters.length === 0 ? IDLE_SNAPSHOT : LOADING_SNAPSHOT;
    for (const listener of door.listeners) listener();
    if (door.retain > 0 && door.filters.length > 0) {
      door.stop = startSharedDoor(door);
    }
  }
}

/**
 * One live REQ set for a door. `filters` is the Protocol §6.5 list; an empty
 * list (identity not ready) subscribes to nothing. Callers with the same
 * filters share one subscription.
 */
export function useLiveDoorEvents(
  filters: readonly RelaySubscriptionFilter[],
  client: LiveReqClient = defaultClient,
): LiveDoorEvents {
  const filterKey = filterKeyOf(filters);
  const subscribe = React.useCallback(
    (listener: () => void) =>
      subscribeSharedDoor(
        JSON.parse(filterKey) as RelaySubscriptionFilter[],
        listener,
        client,
      ),
    [client, filterKey],
  );
  const getSnapshot = React.useCallback(
    () =>
      doors.get(filterKey)?.snapshot ??
      (filterKey === "[]" ? IDLE_SNAPSHOT : LOADING_SNAPSHOT),
    [filterKey],
  );
  const live = React.useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
  const held = React.useRef({
    epoch: doorEpoch,
    snapshot: live,
    view: live,
  });
  const next = holdDoorSnapshot(
    held.current.snapshot,
    held.current.epoch,
    live,
    doorEpoch,
  );
  if (
    held.current.view === live &&
    next.view === live &&
    held.current.epoch === next.heldEpoch &&
    held.current.snapshot === next.held
  ) {
    return live;
  }
  if (
    next.view !== live &&
    held.current.view.events === next.view.events &&
    held.current.view.isLoading === next.view.isLoading &&
    held.current.epoch === next.heldEpoch
  ) {
    return held.current.view;
  }
  const view = next.view === live ? live : next.view;
  held.current = { epoch: next.heldEpoch, snapshot: next.held, view };
  return view;
}
