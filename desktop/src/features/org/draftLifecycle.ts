/**
 * Which chat drafts this member published or deleted.
 *
 * A draft is the org agent's chat message, so the relay keeps no state for
 * it. The closed ids live here, per member, so My Work and every chat hide
 * the same drafts.
 */

import * as React from "react";

const STORAGE_PREFIX = "buzz.org.draftLifecycle.v1";
const LEGACY_PUBLISHED_KEY = "buzz.org.publishedDrafts";
export const MAX_CLOSED_DRAFTS = 400;

export type DraftLifecycle = {
  published: string[];
  deleted: string[];
};

export type DraftClose = "published" | "deleted";

export const EMPTY_LIFECYCLE: DraftLifecycle = Object.freeze({
  published: [],
  deleted: [],
}) as DraftLifecycle;

function idList(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter(
    (id): id is string => typeof id === "string" && id.length > 0,
  );
}

export function parseDraftLifecycle(raw: string | null): DraftLifecycle {
  if (!raw) return EMPTY_LIFECYCLE;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return EMPTY_LIFECYCLE;
    }
    const record = parsed as Record<string, unknown>;
    return {
      published: idList(record.published).slice(-MAX_CLOSED_DRAFTS),
      deleted: idList(record.deleted).slice(-MAX_CLOSED_DRAFTS),
    };
  } catch {
    return EMPTY_LIFECYCLE;
  }
}

/** Append one id, newest last, keeping each list bounded. */
export function closeDraft(
  state: DraftLifecycle,
  messageId: string,
  how: DraftClose,
): DraftLifecycle {
  const list = state[how].filter((id) => id !== messageId);
  list.push(messageId);
  return { ...state, [how]: list.slice(-MAX_CLOSED_DRAFTS) };
}

export function closedDraftIds(state: DraftLifecycle): Set<string> {
  return new Set([...state.published, ...state.deleted]);
}

function storageKey(pubkey: string): string {
  return `${STORAGE_PREFIX}:${pubkey.trim().toLowerCase()}`;
}

function readRaw(pubkey: string): string | null {
  try {
    return window.localStorage.getItem(storageKey(pubkey));
  } catch {
    return null;
  }
}

/** Ids the chat hid this session, before the store existed. */
function legacyPublished(): string[] {
  try {
    return idList(
      JSON.parse(window.sessionStorage.getItem(LEGACY_PUBLISHED_KEY) ?? "[]"),
    );
  } catch {
    return [];
  }
}

function withLegacy(stored: DraftLifecycle): DraftLifecycle {
  const legacy = legacyPublished();
  if (legacy.length === 0) return stored;
  let merged = stored;
  for (const id of legacy) {
    if (!merged.published.includes(id)) {
      merged = closeDraft(merged, id, "published");
    }
  }
  return merged;
}

export function readDraftLifecycle(pubkey: string): DraftLifecycle {
  return withLegacy(parseDraftLifecycle(readRaw(pubkey)));
}

const listeners = new Set<() => void>();

function notify(): void {
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  const onStorage = (event: StorageEvent) => {
    if (!event.key || event.key.startsWith(STORAGE_PREFIX)) listener();
  };
  window.addEventListener("storage", onStorage);
  return () => {
    listeners.delete(listener);
    window.removeEventListener("storage", onStorage);
  };
}

/** Record a published or deleted draft. False when storage refused the write. */
export function recordDraftClosed(
  pubkey: string,
  messageId: string,
  how: DraftClose,
): boolean {
  const next = closeDraft(readDraftLifecycle(pubkey), messageId, how);
  try {
    window.localStorage.setItem(storageKey(pubkey), JSON.stringify(next));
  } catch {
    return false;
  }
  notify();
  return true;
}

export type DraftLifecycleHandle = {
  closed: ReadonlySet<string>;
  /** False when storage refused the write. */
  markPublished: (messageId: string) => boolean;
  /** False when storage refused the write. */
  markDeleted: (messageId: string) => boolean;
};

const NO_IDS: ReadonlySet<string> = new Set();

export function useDraftLifecycle(pubkey: string | null): DraftLifecycleHandle {
  const raw = React.useSyncExternalStore(
    subscribe,
    () => (pubkey ? readRaw(pubkey) : null),
    () => null,
  );
  const closed = React.useMemo<ReadonlySet<string>>(
    () =>
      pubkey ? closedDraftIds(withLegacy(parseDraftLifecycle(raw))) : NO_IDS,
    [pubkey, raw],
  );
  const markPublished = React.useCallback(
    (messageId: string) =>
      pubkey ? recordDraftClosed(pubkey, messageId, "published") : false,
    [pubkey],
  );
  const markDeleted = React.useCallback(
    (messageId: string) =>
      pubkey ? recordDraftClosed(pubkey, messageId, "deleted") : false,
    [pubkey],
  );
  return { closed, markPublished, markDeleted };
}
