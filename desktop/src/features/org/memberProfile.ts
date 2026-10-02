/**
 * What a member's profile shows: projects they are responsible for, tickets
 * they currently hold, work they finished, and the recent org actions they
 * signed.
 */

import type { RelayEvent } from "@/shared/api/types";
import { KIND_IO_PROGRESS, KIND_IO_PROPOSAL } from "@/shared/constants/kinds";
import { normalizePubkey } from "@/shared/lib/pubkey";

import type { WorkItem } from "./work/model";

const HELD_STATES = new Set(["accepted", "in_review"]);

/** How many activities the profile shows before "Show more". */
export const PROFILE_ACTIVITY_PREVIEW = 5;

/** Upper bound for the activity list, including what "Show more" reveals. */
export const PROFILE_ACTIVITY_CAP = 40;

const ACTIVITY_LABELS: Record<number, string> = {
  50001: "Proposed a Shapers change",
  50002: "Proposed a direction",
  50003: "Voted",
  50004: "Proposed a project",
  50005: "Created a ticket",
  50006: "Offered work",
  50007: "Accepted work",
  50008: "Declined work",
  50009: "Marked work done",
  50010: "Released work",
  50011: "Set a due date",
  50012: "Decided a draft",
  50017: "Rated health",
  50018: "Reopened work",
  50019: "Accepted a Shaper seat",
  50020: "Stepped down as a Shaper",
  50021: "Updated their profile",
  50022: "Removed work",
  50023: "Proposed removing a project",
};

export type MemberActivity = {
  id: string;
  createdAt: number;
  label: string;
  detail: string | null;
  itemId: string | null;
};

export type VoteSubject = {
  title: string;
  itemId: string | null;
};

export function holdsWork(item: WorkItem, pubkey: string): boolean {
  if (!item.dri) return false;
  return normalizePubkey(item.dri) === normalizePubkey(pubkey);
}

export type ProfileHeldWork = {
  projects: WorkItem[];
  tickets: WorkItem[];
  earlier: WorkItem[];
};

/**
 * Projects this person is responsible for (they are the DRI), tickets they
 * currently hold, and work they already finished.
 */
export function profileHeldWork(
  items: readonly WorkItem[],
  pubkey: string,
): ProfileHeldWork {
  const held = items.filter((item) => holdsWork(item, pubkey));
  const byTitle = (left: WorkItem, right: WorkItem) =>
    left.title.localeCompare(right.title);
  const current = (type: WorkItem["type"]) =>
    held
      .filter((item) => item.type === type && HELD_STATES.has(item.state))
      .sort(byTitle);
  const earlier = held
    .filter((item) => item.state === "done")
    .sort(
      (left, right) =>
        (right.approvedAt ?? right.createdAt) -
        (left.approvedAt ?? left.createdAt),
    );
  return {
    projects: current("project"),
    tickets: current("ticket"),
    earlier,
  };
}

function tagValue(tags: readonly string[][], name: string): string | null {
  return tags.find((tag) => tag[0] === name)?.[1] ?? null;
}

function contentRecord(content: string): Record<string, unknown> | null {
  const trimmed = content.trim();
  if (!trimmed) return null;
  try {
    const parsed: unknown = JSON.parse(trimmed);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    return null;
  }
  return null;
}

function textField(
  record: Record<string, unknown> | null,
  key: string,
): string | null {
  const value = record?.[key];
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

const PROPOSAL_KIND_LABEL: Record<string, string> = {
  direction: "Direction",
  project: "a project",
  dri: "a DRI",
  ticket: "a ticket",
  done: "marking work done",
  review: "a review",
  money: "a payment",
  join: "joining",
  shapers: "the Shapers",
  withdraw: "removing a project",
  profile: "a profile",
};

function proposalTitle(
  content: string,
  tags: readonly string[][],
): string | null {
  const record = contentRecord(content);
  const payload = record?.payload;
  const body =
    payload && typeof payload === "object" && !Array.isArray(payload)
      ? (payload as Record<string, unknown>)
      : null;
  const title = textField(body, "title") ?? textField(body, "body");
  if (title) return title.length > 180 ? `${title.slice(0, 177)}…` : title;
  const slug = textField(body, "slug");
  if (slug) return slug.charAt(0).toUpperCase() + slug.slice(1);
  const kind = textField(record, "kind") ?? tagValue(tags, "t");
  if (!kind) return null;
  return PROPOSAL_KIND_LABEL[kind] ?? kind;
}

/** Newest proposal title per id, so a vote can say what it decided. */
export function voteSubjects(
  events: readonly Pick<
    RelayEvent,
    "id" | "kind" | "content" | "created_at" | "tags"
  >[],
): Map<string, VoteSubject> {
  const subjects = new Map<string, VoteSubject>();
  const seen = new Map<string, { at: number; id: string }>();
  for (const event of events) {
    if (event.kind !== KIND_IO_PROPOSAL) continue;
    const id = tagValue(event.tags, "d");
    const title = id ? proposalTitle(event.content, event.tags) : null;
    if (!id || !title) continue;
    const current = seen.get(id);
    if (
      current &&
      (event.created_at < current.at ||
        (event.created_at === current.at && event.id <= current.id))
    ) {
      continue;
    }
    seen.set(id, { at: event.created_at, id: event.id });
    subjects.set(id, {
      title,
      itemId: tagValue(event.tags, "i"),
    });
  }
  return subjects;
}

function voteDetail(choice: string | null, content: string): string | null {
  if (choice === "agree") return "Agreed";
  if (choice === "decline") return "Declined";
  return activityDetail(content);
}

function activityDetail(content: string): string | null {
  const trimmed = content.trim();
  if (!trimmed) return null;
  try {
    const parsed: unknown = JSON.parse(trimmed);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      const record = parsed as Record<string, unknown>;
      for (const key of ["text", "body", "note", "title", "why"]) {
        const value = record[key];
        if (typeof value === "string" && value.trim().length > 0) {
          return value.trim();
        }
      }
      return null;
    }
  } catch {
    // Plain text progress notes stay as the detail line.
  }
  return trimmed.length > 180 ? `${trimmed.slice(0, 177)}…` : trimmed;
}

export function describeMemberActivity(
  event: Pick<RelayEvent, "id" | "kind" | "content" | "created_at" | "tags">,
  subjects?: ReadonlyMap<string, VoteSubject>,
): MemberActivity | null {
  if (event.kind === 50003) {
    const proposalId = tagValue(event.tags, "e");
    const subject = proposalId ? subjects?.get(proposalId) : undefined;
    const choice = tagValue(event.tags, "vote");
    return {
      id: event.id,
      createdAt: event.created_at,
      label: subject ? `Voted on ${subject.title}` : "Voted",
      detail: voteDetail(choice, event.content),
      itemId: subject?.itemId ?? tagValue(event.tags, "i"),
    };
  }
  const label =
    event.kind === KIND_IO_PROGRESS
      ? "Posted a progress note"
      : ACTIVITY_LABELS[event.kind];
  if (!label) return null;
  return {
    id: event.id,
    createdAt: event.created_at,
    label,
    detail: activityDetail(event.content),
    itemId: tagValue(event.tags, "i"),
  };
}

/** Newest signed actions, capped so "Show more" stays a bounded list. */
export function recentMemberActivity(
  events: readonly Pick<
    RelayEvent,
    "id" | "kind" | "content" | "created_at" | "tags"
  >[],
  limit = PROFILE_ACTIVITY_CAP,
  subjects?: ReadonlyMap<string, VoteSubject>,
): MemberActivity[] {
  return events
    .map((event) => describeMemberActivity(event, subjects))
    .filter((entry): entry is MemberActivity => entry !== null)
    .sort(
      (left, right) =>
        right.createdAt - left.createdAt || right.id.localeCompare(left.id),
    )
    .slice(0, limit);
}

/** The five most recent activities, or the full capped list once opened. */
export function visibleProfileActivity<T>(
  entries: readonly T[],
  expanded: boolean,
  preview = PROFILE_ACTIVITY_PREVIEW,
): T[] {
  if (expanded || entries.length <= preview) return [...entries];
  return entries.slice(0, preview);
}

export function formatProfileWhen(
  unixSeconds: number,
  nowSeconds: number,
): string {
  const delta = Math.max(0, nowSeconds - unixSeconds);
  if (delta < 60) return "just now";
  const minutes = Math.floor(delta / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 14) return `${days}d ago`;
  return new Date(unixSeconds * 1000).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}
