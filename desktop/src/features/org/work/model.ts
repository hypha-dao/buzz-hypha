/**
 * Work-tree and item-page read model. Assembled client-side from `39101`
 * (Protocol §4.2 / §6.5). Children counters live on the parent content
 * (R-5a). Do not invent fields or tags.
 */

import type { RelayEvent } from "@/shared/api/types";
import { KIND_IO_HEALTH, KIND_IO_WORK_ITEM } from "@/shared/constants/kinds";

import { IO_COMMAND_KINDS } from "../hooks/filters";
import { TAG_ITEM, TAG_PARENT, TAG_STATUS, TAG_TYPE } from "../tags";

export const WORK_ITEM_STATES = [
  "open",
  "offered",
  "accepted",
  "in_review",
  "done",
] as const;

export type WorkItemState = (typeof WORK_ITEM_STATES)[number];
export type WorkItemType = "project" | "ticket";

export type WorkChildrenCounts = {
  open: number;
  offered: number;
  accepted: number;
  done: number;
};

export type WorkItemHome = {
  channel: string | null;
  repo: string | null;
  project: string | null;
};

export type WorkItem = {
  id: string;
  eventId: string;
  createdAt: number;
  parent: string | null;
  root: string;
  depth: number;
  path: string[];
  title: string;
  brief: string;
  state: WorkItemState;
  type: WorkItemType;
  dri: string | null;
  offeredTo: string | null;
  offeredBy: string | null;
  dueAt: number | null;
  approvedAt: number | null;
  children: WorkChildrenCounts;
  home: WorkItemHome | null;
  lastProgress: string | null;
};

export type WorkHealthSentence = {
  text: string;
  rows: string[];
};

export type WorkHealth = {
  eventId: string;
  createdAt: number;
  item: string;
  week: string;
  pct: number;
  band: "struggling" | "wobbly" | "healthy" | string;
  sentences: WorkHealthSentence[];
};

export type WorkTrailEntry = {
  id: string;
  kind: number;
  createdAt: number;
  pubkey: string;
  label: string;
};

export type WorkTreeNode = {
  item: WorkItem;
  children: WorkItem[];
};

const EMPTY_COUNTS: WorkChildrenCounts = {
  open: 0,
  offered: 0,
  accepted: 0,
  done: 0,
};

const COMMAND_LABELS: Record<number, string> = {
  50001: "io_shapers_propose",
  50002: "io_direction_propose",
  50003: "io_vote",
  50004: "io_project_propose",
  50005: "io_ticket_create",
  50006: "io_offer",
  50007: "io_accept",
  50008: "io_decline",
  50009: "io_done",
  50010: "io_release",
  50011: "io_set_due",
  50012: "io_draft_decide",
  50013: "io_money_propose",
  50014: "io_money_released",
  50015: "io_dri_propose",
  50016: "io_join_propose",
  50017: "io_health_rate",
  50018: "io_reopen",
  50019: "io_shaper_accept",
  50020: "io_shaper_step_down",
  50021: "io_profile_set",
};

function tagValue(tags: readonly string[][], name: string): string | null {
  return tags.find((tag) => tag[0] === name)?.[1] ?? null;
}

function taggedValues(
  tags: readonly string[][],
  name: string,
  marker?: string,
): string[] {
  return tags
    .filter((tag) => {
      if (tag[0] !== name) return false;
      if (marker === undefined) return !tag[3];
      return tag[3] === marker;
    })
    .map((tag) => tag[1])
    .filter((value): value is string => Boolean(value));
}

function isWorkItemState(value: unknown): value is WorkItemState {
  return (
    typeof value === "string" &&
    (WORK_ITEM_STATES as readonly string[]).includes(value)
  );
}

function asCount(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0
    ? value
    : 0;
}

function asUnix(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function asString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

/**
 * Newest addressable `39101` per `d`. A later rewrite (children counters,
 * holder, dates) replaces the previous head.
 */
export function latestWorkItems(
  events: readonly RelayEvent[],
): Map<string, RelayEvent> {
  const latest = new Map<string, RelayEvent>();
  for (const event of events) {
    if (event.kind !== KIND_IO_WORK_ITEM) continue;
    const id = tagValue(event.tags, "d");
    if (!id) continue;
    const current = latest.get(id);
    if (
      !current ||
      event.created_at > current.created_at ||
      (event.created_at === current.created_at && event.id > current.id)
    ) {
      latest.set(id, event);
    }
  }
  return latest;
}

export function parseWorkItem(event: RelayEvent): WorkItem | null {
  if (event.kind !== KIND_IO_WORK_ITEM) return null;
  const id = tagValue(event.tags, "d");
  if (!id) return null;

  let content: Record<string, unknown> = {};
  try {
    const parsed: unknown = JSON.parse(event.content || "{}");
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      content = parsed as Record<string, unknown>;
    }
  } catch {
    content = {};
  }

  const rawChildren = content.children;
  const children: WorkChildrenCounts =
    rawChildren &&
    typeof rawChildren === "object" &&
    !Array.isArray(rawChildren)
      ? {
          open: asCount((rawChildren as Record<string, unknown>).open),
          offered: asCount((rawChildren as Record<string, unknown>).offered),
          accepted: asCount((rawChildren as Record<string, unknown>).accepted),
          done: asCount((rawChildren as Record<string, unknown>).done),
        }
      : EMPTY_COUNTS;

  const rawHome = content.home;
  const home: WorkItemHome | null =
    rawHome && typeof rawHome === "object" && !Array.isArray(rawHome)
      ? {
          channel: asString((rawHome as Record<string, unknown>).channel),
          repo: asString((rawHome as Record<string, unknown>).repo),
          project: asString((rawHome as Record<string, unknown>).project),
        }
      : null;

  const parent =
    asString(content.parent) ?? tagValue(event.tags, TAG_PARENT) ?? null;
  const root = asString(content.root) ?? tagValue(event.tags, "root") ?? id;
  const stateTag = tagValue(event.tags, TAG_STATUS);
  const state = isWorkItemState(content.state)
    ? content.state
    : isWorkItemState(stateTag)
      ? stateTag
      : "open";
  const typeTag = tagValue(event.tags, TAG_TYPE);
  const type: WorkItemType =
    typeTag === "ticket" || typeTag === "project"
      ? typeTag
      : parent
        ? "ticket"
        : "project";
  const offeredTo =
    asString(content.offered_to) ??
    taggedValues(event.tags, "p", "offered")[0] ??
    null;
  const dri = asString(content.dri) ?? taggedValues(event.tags, "p")[0] ?? null;

  return {
    id,
    eventId: event.id,
    createdAt: event.created_at,
    parent,
    root,
    depth: asCount(content.depth),
    path: Array.isArray(content.path)
      ? content.path.filter(
          (entry): entry is string => typeof entry === "string",
        )
      : [],
    title: asString(content.title) ?? id,
    brief: typeof content.brief === "string" ? content.brief : "",
    state,
    type,
    dri,
    offeredTo,
    offeredBy: asString(content.offered_by),
    dueAt: asUnix(content.due_at),
    approvedAt: asUnix(content.approved_at),
    children,
    home,
    lastProgress: asString(content.last_progress),
  };
}

export function isRootItem(item: WorkItem): boolean {
  return (
    item.parent === null || item.root === item.id || item.type === "project"
  );
}

/** Roots + one level — the Work door. Deeper rows stay for the item page. */
export function assembleWorkDoor(
  events: readonly RelayEvent[],
): WorkTreeNode[] {
  const items = [...latestWorkItems(events).values()]
    .map(parseWorkItem)
    .filter((item): item is WorkItem => item !== null);
  const byParent = new Map<string, WorkItem[]>();
  for (const item of items) {
    if (!item.parent) continue;
    const siblings = byParent.get(item.parent) ?? [];
    siblings.push(item);
    byParent.set(item.parent, siblings);
  }
  for (const siblings of byParent.values()) {
    siblings.sort((left, right) => left.title.localeCompare(right.title));
  }
  return items
    .filter(isRootItem)
    .sort((left, right) => left.title.localeCompare(right.title))
    .map((item) => ({
      item,
      children: byParent.get(item.id) ?? [],
    }));
}

export function childrenOf(
  events: readonly RelayEvent[],
  parentId: string,
): WorkItem[] {
  return [...latestWorkItems(events).values()]
    .map(parseWorkItem)
    .filter((item): item is WorkItem => item !== null)
    .filter((item) => item.parent === parentId)
    .sort((left, right) => left.title.localeCompare(right.title));
}

export function itemById(
  events: readonly RelayEvent[],
  itemId: string,
): WorkItem | null {
  const event = latestWorkItems(events).get(itemId);
  return event ? parseWorkItem(event) : null;
}

/** Prototype map StateChip labels. In progress always has a holder. */
export function stateChipLabel(item: Pick<WorkItem, "state" | "type">): string {
  switch (item.state) {
    case "done":
      return "done";
    case "accepted":
    case "in_review":
      return "in progress";
    case "offered":
      return "waiting on a yes";
    case "open":
      return item.type === "project" ? "needs a DRI" : "open";
    default:
      return item.state;
  }
}

export function stateChipTone(
  state: WorkItemState,
): "done" | "doing" | "waiting" | "open" {
  if (state === "done") return "done";
  if (state === "accepted" || state === "in_review") return "doing";
  if (state === "offered") return "waiting";
  return "open";
}

/** R-5a leftover: counters on the parent `39101`, rewritten, not added. */
export function formatChildrenCounts(counts: WorkChildrenCounts): string {
  return `${counts.open} open · ${counts.offered} offered · ${counts.accepted} accepted · ${counts.done} done`;
}

export function commandLabel(kind: number): string {
  return COMMAND_LABELS[kind] ?? `kind:${kind}`;
}

export function trailForItem(
  events: readonly RelayEvent[],
  itemId: string,
): WorkTrailEntry[] {
  const commandKinds = new Set(IO_COMMAND_KINDS);
  return events
    .filter(
      (event) =>
        commandKinds.has(event.kind) &&
        event.tags.some((tag) => tag[0] === TAG_ITEM && tag[1] === itemId),
    )
    .map((event) => ({
      id: event.id,
      kind: event.kind,
      createdAt: event.created_at,
      pubkey: event.pubkey,
      label: commandLabel(event.kind),
    }))
    .sort((left, right) => {
      if (right.createdAt !== left.createdAt) {
        return right.createdAt - left.createdAt;
      }
      return right.id.localeCompare(left.id);
    });
}

export function latestHealth(
  events: readonly RelayEvent[],
  itemId: string,
): WorkHealth | null {
  const matching = events.filter(
    (event) =>
      event.kind === KIND_IO_HEALTH &&
      event.tags.some((tag) => tag[0] === TAG_ITEM && tag[1] === itemId),
  );
  matching.sort((left, right) => {
    if (right.created_at !== left.created_at) {
      return right.created_at - left.created_at;
    }
    return right.id.localeCompare(left.id);
  });
  const event = matching[0];
  if (!event) return null;

  let content: Record<string, unknown> = {};
  try {
    const parsed: unknown = JSON.parse(event.content || "{}");
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      content = parsed as Record<string, unknown>;
    }
  } catch {
    content = {};
  }

  const sentences = Array.isArray(content.sentences)
    ? content.sentences.flatMap((entry): WorkHealthSentence[] => {
        if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
          return [];
        }
        const text = (entry as { text?: unknown }).text;
        const rows = (entry as { rows?: unknown }).rows;
        if (typeof text !== "string") return [];
        return [
          {
            text,
            rows: Array.isArray(rows)
              ? rows.filter((row): row is string => typeof row === "string")
              : [],
          },
        ];
      })
    : [];

  return {
    eventId: event.id,
    createdAt: event.created_at,
    item: asString(content.item) ?? itemId,
    week: asString(content.week) ?? tagValue(event.tags, "week") ?? "",
    pct: typeof content.pct === "number" ? content.pct : 0,
    band: asString(content.band) ?? tagValue(event.tags, "band") ?? "",
    sentences,
  };
}

export function homeChannel(item: WorkItem | null): string | null {
  const channel = item?.home?.channel;
  return channel && channel.length > 0 ? channel : null;
}

export function canMarkDone(item: WorkItem, viewer: string | null): boolean {
  if (!viewer) return false;
  if (item.state !== "accepted" && item.state !== "in_review") return false;
  return (item.dri ?? "").toLowerCase() === viewer.toLowerCase();
}

export function canRelease(item: WorkItem, viewer: string | null): boolean {
  return canMarkDone(item, viewer);
}

export function formatWorkDate(unix: number): string {
  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  }).format(new Date(unix * 1000));
}
