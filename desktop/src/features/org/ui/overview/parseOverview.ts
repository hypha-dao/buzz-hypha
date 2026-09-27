/**
 * Overview door reads — Protocol §4.1 / §4.2 / §4.5 / §4.7c heads from the
 * D-0 REQ set (and the page-local tally / direction-history REQs).
 */

import type { RelayEvent } from "@/shared/api/types";
import {
  KIND_IO_AGENT_NOTE,
  KIND_IO_DIRECTION,
  KIND_IO_PROPOSAL,
  KIND_IO_SHAPERS,
  KIND_IO_WORK_ITEM,
} from "@/shared/constants/kinds";
import { normalizePubkey } from "@/shared/lib/pubkey";

import type { DirectionSlug } from "../../commands";
import { SHAPERS_D_TAG, TAG_STATUS, TAG_TYPE, TYPE_PROJECT } from "../../tags";

export const DIRECTION_SLUGS: readonly DirectionSlug[] = [
  "mission",
  "vision",
  "objectives",
  "strategy",
];

export type DirectionLine = {
  n: number;
  id: string;
  text: string;
  date?: number;
};

export type DirectionHead = {
  slug: DirectionSlug;
  version: number;
  body: string;
  lines: DirectionLine[];
  confirmedBy: string | null;
  confirmedAt: number | null;
};

export type DirectionSlot = {
  slug: DirectionSlug;
  head: DirectionHead | null;
};

export type ShapersOffered = {
  p: string;
  proposal: string;
  at: number;
};

export type ShapersState = {
  founder: string | null;
  shapers: string[];
  offered: ShapersOffered[];
  rules: Record<string, string | number>;
  agent: string | null;
  agentHosted: boolean;
  decisionWindowSecs: number | null;
  offerWindowSecs: number | null;
};

export type ProjectHold = {
  id: string;
  title: string;
  dri: string | null;
  state: string;
};

export type TallyMove = {
  opened: number;
  accepted: number;
  amended: number;
  declined: Record<string, number>;
  dropped: Record<string, number>;
  shadow: number;
};

export type TallyNote = {
  week: string;
  windowWeeks: number;
  moves: Record<string, TallyMove>;
  health: { reads: number; rated: number; agreed: number } | null;
  openOlderThan5d: number;
  receiptRejected: number;
};

export type DirectionVersion = {
  proposal: string;
  version: number;
  body: string;
  confirmedBy: string | null;
  decidedAt: number | null;
};

function tagValue(
  event: Pick<RelayEvent, "tags">,
  name: string,
): string | undefined {
  return event.tags.find((tag) => tag[0] === name)?.[1];
}

function tagValues(event: Pick<RelayEvent, "tags">, name: string): string[] {
  return event.tags
    .filter((tag) => tag[0] === name && tag[1])
    .map((tag) => tag[1] as string);
}

function newestByCreatedAt<T extends { created_at: number }>(
  events: T[],
): T | undefined {
  return [...events].sort(
    (left, right) => right.created_at - left.created_at,
  )[0];
}

function isDirectionSlug(value: string): value is DirectionSlug {
  return (DIRECTION_SLUGS as readonly string[]).includes(value);
}

function parseJson(content: string): Record<string, unknown> | null {
  try {
    const value = JSON.parse(content) as unknown;
    return value && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

function asString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function asNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function asPubkey(value: unknown): string | null {
  const text = asString(value);
  return text ? normalizePubkey(text) : null;
}

function parseLines(value: unknown): DirectionLine[] {
  if (!Array.isArray(value)) return [];
  const lines: DirectionLine[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== "object") continue;
    const row = entry as Record<string, unknown>;
    const n = asNumber(row.n);
    const id = asString(row.id);
    const text = asString(row.text);
    if (n === null || !id || !text) continue;
    const date = asNumber(row.date);
    lines.push(date === null ? { n, id, text } : { n, id, text, date });
  }
  return lines.sort((left, right) => left.n - right.n);
}

function parseDirectionHead(event: RelayEvent): DirectionHead | null {
  if (event.kind !== KIND_IO_DIRECTION) return null;
  const content = parseJson(event.content);
  const slugRaw = asString(content?.slug) ?? tagValue(event, "d");
  if (!slugRaw || !isDirectionSlug(slugRaw)) return null;
  const version =
    asNumber(content?.version) ??
    Number.parseInt(tagValue(event, "version") ?? "", 10);
  if (!Number.isFinite(version)) return null;
  const confirmedBy =
    asPubkey(content?.confirmed_by) ??
    (tagValue(event, "p")
      ? normalizePubkey(tagValue(event, "p") as string)
      : null);
  return {
    slug: slugRaw,
    version,
    body: asString(content?.body) ?? "",
    lines: parseLines(content?.lines),
    confirmedBy,
    confirmedAt: asNumber(content?.confirmed_at),
  };
}

/** One slot per slug; the newest `39100` per `d` is the head. */
export function directionSlots(
  events: readonly Pick<
    RelayEvent,
    "kind" | "tags" | "content" | "created_at" | "id"
  >[],
): DirectionSlot[] {
  const heads = new Map<
    DirectionSlug,
    { created_at: number; head: DirectionHead }
  >();
  for (const event of events) {
    const head = parseDirectionHead(event as RelayEvent);
    if (!head) continue;
    const existing = heads.get(head.slug);
    if (!existing || event.created_at > existing.created_at) {
      heads.set(head.slug, { created_at: event.created_at, head });
    }
  }
  return DIRECTION_SLUGS.map((slug) => ({
    slug,
    head: heads.get(slug)?.head ?? null,
  }));
}

export function parseShapersState(
  events: readonly Pick<
    RelayEvent,
    "kind" | "tags" | "content" | "created_at"
  >[],
): ShapersState | null {
  const newest = newestByCreatedAt(
    events.filter(
      (event) =>
        event.kind === KIND_IO_SHAPERS &&
        event.tags.some((tag) => tag[0] === "d" && tag[1] === SHAPERS_D_TAG),
    ),
  );
  if (!newest) return null;
  const content = parseJson(newest.content);
  const shapers = Array.isArray(content?.shapers)
    ? content.shapers
        .filter((entry): entry is string => typeof entry === "string")
        .map((entry) => normalizePubkey(entry))
    : tagValues(newest, "p").map((entry) => normalizePubkey(entry));
  const offered: ShapersOffered[] = [];
  if (Array.isArray(content?.offered)) {
    for (const entry of content.offered) {
      if (!entry || typeof entry !== "object") continue;
      const row = entry as Record<string, unknown>;
      const p = asPubkey(row.p);
      const proposal = asString(row.proposal);
      const at = asNumber(row.at) ?? 0;
      if (p && proposal) offered.push({ p, proposal, at });
    }
  }
  const rules: Record<string, string | number> = {};
  if (content?.rules && typeof content.rules === "object") {
    for (const [key, value] of Object.entries(
      content.rules as Record<string, unknown>,
    )) {
      if (typeof value === "string" || typeof value === "number") {
        rules[key] = value;
      }
    }
  }
  return {
    founder: asPubkey(content?.founder),
    shapers,
    offered,
    rules,
    agent: asPubkey(content?.agent),
    agentHosted: content?.agent_hosted === true,
    decisionWindowSecs: asNumber(content?.decision_window_secs),
    offerWindowSecs: asNumber(content?.offer_window_secs),
  };
}

function holderPubkey(
  event: RelayEvent,
  content: Record<string, unknown> | null,
): string | null {
  const fromContent = asPubkey(content?.dri);
  if (fromContent) return fromContent;
  const tag = event.tags.find(
    (entry) => entry[0] === "p" && entry[3] !== "offered",
  )?.[1];
  return tag ? normalizePubkey(tag) : null;
}

function parseProjectHold(event: RelayEvent): ProjectHold | null {
  if (event.kind !== KIND_IO_WORK_ITEM) return null;
  const type = tagValue(event, TAG_TYPE);
  if (type && type !== TYPE_PROJECT) return null;
  const content = parseJson(event.content);
  if (type !== TYPE_PROJECT) {
    const parent = content?.parent;
    const depth = asNumber(content?.depth);
    if (parent != null && parent !== "") return null;
    if (depth !== null && depth !== 0) return null;
  }
  const id = asString(content?.id) ?? tagValue(event, "d");
  const title = asString(content?.title);
  if (!id || !title) return null;
  return {
    id,
    title,
    dri: holderPubkey(event, content),
    state: asString(content?.state) ?? tagValue(event, "s") ?? "",
  };
}

/** Root `39101`s (`#t=project`) — who holds what. */
export function projectHolds(
  events: readonly Pick<
    RelayEvent,
    "kind" | "tags" | "content" | "created_at"
  >[],
): ProjectHold[] {
  const byId = new Map<string, { created_at: number; hold: ProjectHold }>();
  for (const event of events) {
    const hold = parseProjectHold(event as RelayEvent);
    if (!hold) continue;
    const existing = byId.get(hold.id);
    if (!existing || event.created_at > existing.created_at) {
      byId.set(hold.id, { created_at: event.created_at, hold });
    }
  }
  return [...byId.values()]
    .map((entry) => entry.hold)
    .sort((left, right) => left.title.localeCompare(right.title));
}

function parseCounts(value: unknown): Record<string, number> {
  if (!value || typeof value !== "object") return {};
  const counts: Record<string, number> = {};
  for (const [key, entry] of Object.entries(value as Record<string, unknown>)) {
    const n = asNumber(entry);
    if (n !== null) counts[key] = n;
  }
  return counts;
}

function parseTallyMove(value: unknown): TallyMove | null {
  if (!value || typeof value !== "object") return null;
  const row = value as Record<string, unknown>;
  return {
    opened: asNumber(row.opened) ?? 0,
    accepted: asNumber(row.accepted) ?? 0,
    amended: asNumber(row.amended) ?? 0,
    declined: parseCounts(row.declined),
    dropped: parseCounts(row.dropped),
    shadow: asNumber(row.shadow) ?? 0,
  };
}

/** Newest `50103` with `t=tally`. */
export function parseTallyNote(
  events: readonly Pick<
    RelayEvent,
    "kind" | "tags" | "content" | "created_at"
  >[],
): TallyNote | null {
  const newest = newestByCreatedAt(
    events.filter((event) => {
      if (event.kind !== KIND_IO_AGENT_NOTE) return false;
      if (tagValue(event, TAG_TYPE) === "tally") return true;
      const content = parseJson(event.content);
      return asString(content?.note) === "tally";
    }),
  );
  if (!newest) return null;
  const content = parseJson(newest.content);
  if (!content) return null;
  const moves: Record<string, TallyMove> = {};
  if (content.moves && typeof content.moves === "object") {
    for (const [key, value] of Object.entries(
      content.moves as Record<string, unknown>,
    )) {
      const move = parseTallyMove(value);
      if (move) moves[key] = move;
    }
  }
  const healthRaw =
    content.health && typeof content.health === "object"
      ? (content.health as Record<string, unknown>)
      : null;
  return {
    week: asString(content.week) ?? tagValue(newest, "week") ?? "",
    windowWeeks: asNumber(content.window_weeks) ?? 4,
    moves,
    health: healthRaw
      ? {
          reads: asNumber(healthRaw.reads) ?? 0,
          rated: asNumber(healthRaw.rated) ?? 0,
          agreed: asNumber(healthRaw.agreed) ?? 0,
        }
      : null,
    openOlderThan5d: asNumber(content.open_older_than_5d) ?? 0,
    receiptRejected: asNumber(content.receipt_rejected) ?? 0,
  };
}

function lastAgreeingVoter(votes: unknown): string | null {
  if (!Array.isArray(votes)) return null;
  for (let index = votes.length - 1; index >= 0; index -= 1) {
    const vote = votes[index];
    if (!vote || typeof vote !== "object") continue;
    const row = vote as Record<string, unknown>;
    if (row.vote === "agree") return asPubkey(row.p);
  }
  return null;
}

/** Passed `direction` proposals for one slug (Protocol §6.5 Direction page). */
export function directionHistory(
  events: readonly Pick<
    RelayEvent,
    "kind" | "tags" | "content" | "created_at"
  >[],
  slug: DirectionSlug,
): DirectionVersion[] {
  const versions: DirectionVersion[] = [];
  for (const event of events) {
    if (event.kind !== KIND_IO_PROPOSAL) continue;
    if (tagValue(event, TAG_TYPE) !== "direction") continue;
    if (tagValue(event, TAG_STATUS) !== "passed") continue;
    const content = parseJson(event.content);
    const payload =
      content?.payload && typeof content.payload === "object"
        ? (content.payload as Record<string, unknown>)
        : null;
    const executed =
      content?.executed && typeof content.executed === "object"
        ? (content.executed as Record<string, unknown>)
        : null;
    const payloadSlug = asString(payload?.slug);
    const executedId = asString(executed?.id);
    if (payloadSlug !== slug && executedId !== slug) continue;
    const base = asNumber(payload?.base);
    versions.push({
      proposal:
        asString(content?.id) ??
        tagValue(event, "d") ??
        event.created_at.toString(),
      version: base === null ? 0 : base + 1,
      body: asString(payload?.body) ?? "",
      confirmedBy: lastAgreeingVoter(content?.votes),
      decidedAt: asNumber(content?.decided_at),
    });
  }
  return versions.sort((left, right) => right.version - left.version);
}

export function collectOverviewPubkeys(
  slots: readonly DirectionSlot[],
  shapers: ShapersState | null,
  holds: readonly ProjectHold[],
): string[] {
  const pubkeys = new Set<string>();
  for (const slot of slots) {
    if (slot.head?.confirmedBy) pubkeys.add(slot.head.confirmedBy);
  }
  if (shapers) {
    if (shapers.founder) pubkeys.add(shapers.founder);
    if (shapers.agent) pubkeys.add(shapers.agent);
    for (const pubkey of shapers.shapers) pubkeys.add(pubkey);
    for (const seat of shapers.offered) pubkeys.add(seat.p);
  }
  for (const hold of holds) {
    if (hold.dri) pubkeys.add(hold.dri);
  }
  return [...pubkeys];
}
