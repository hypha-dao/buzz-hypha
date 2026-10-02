/**
 * What a proposal card expands to, and which #shapers message it hangs on.
 *
 * The 39102 is not a channel message. The agent's kind 9 carries the
 * direction, project, or revise tag, and the card sits under that message.
 */

import { announcementMentions } from "./chatDraft";

const SLUGS = new Set(["mission", "vision", "objectives", "strategy"]);

export type ProposalDetail = {
  proposalId: string;
  kind: "direction" | "project";
  title: string;
  body: string;
  slug: "mission" | "vision" | "objectives" | "strategy" | null;
  dueAt: number | null;
};

export type ProposalDetailEvent = {
  kind: number;
  content: string;
  tags: readonly (readonly string[])[];
};

export type AnchorMessage = {
  id: string;
  createdAt: number;
  body?: string;
  tags?: readonly (readonly string[])[];
};

export type ChatProposalPreview = {
  messageId: string;
  kind: "direction" | "project";
  title: string;
  body: string;
  proposalId: string | null;
  dueAt: number | null;
};

function objectOf(content: string): Record<string, unknown> | null {
  try {
    const value: unknown = JSON.parse(content);
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      return null;
    }
    return value as Record<string, unknown>;
  } catch {
    return null;
  }
}

function text(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function payloadOf(
  content: Record<string, unknown>,
): Record<string, unknown> | null {
  const payload = content.payload;
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    return null;
  }
  return payload as Record<string, unknown>;
}

function same(left: string | undefined, right: string): boolean {
  const normalize = (value: string) =>
    value.trim().toLowerCase().replace(/\s+/g, " ");
  return normalize(left ?? "") === normalize(right);
}

/** Direction body or project brief, when this event is that kind of proposal. */
export function proposalDetail(
  event: ProposalDetailEvent,
): ProposalDetail | null {
  if (event.kind !== 39102) return null;
  const content = objectOf(event.content);
  if (!content) return null;
  const kind = content.kind;
  if (kind !== "direction" && kind !== "project") return null;
  const proposalId =
    event.tags.find((tag) => tag[0] === "d")?.[1]?.trim() ?? "";
  if (!proposalId) return null;
  const payload = payloadOf(content);
  if (kind === "direction") {
    const slug = text(payload?.slug);
    const body = text(payload?.body);
    return {
      proposalId,
      kind,
      title: body || slug || "Direction",
      body,
      slug: SLUGS.has(slug) ? (slug as ProposalDetail["slug"]) : null,
      dueAt: null,
    };
  }
  const due = payload?.due_at;
  const dueAt = typeof due === "number" && Number.isFinite(due) ? due : null;
  return {
    proposalId,
    kind,
    title: text(payload?.title) || "Project",
    body: text(payload?.brief),
    slug: null,
    dueAt,
  };
}

function tagRow(
  tags: readonly (readonly string[])[] | undefined,
  name: string,
): readonly string[] | undefined {
  return tags?.find((tag) => tag[0] === name);
}

/**
 * The chat line this card belongs under.
 *
 * Prefer the message that carries this version: the same direction body,
 * the same project title, or a revise tag for this proposal. A later draft
 * of the same slug is a different version, so earlier ones stay on their
 * own lines. Otherwise the line closest to when the proposal opened, so
 * the card stays in the thread instead of a strip above it.
 */
export function proposalMessageId(
  messages: readonly AnchorMessage[],
  detail: ProposalDetail,
  at?: number,
): string | null {
  let best: AnchorMessage | null = null;
  for (const message of messages) {
    if (!messageMentions(message, detail)) continue;
    if (!best || message.createdAt >= best.createdAt) best = message;
  }
  if (best) return best.id;
  if (at === undefined || messages.length === 0) return null;
  let nearest = messages[0];
  if (!nearest) return null;
  let distance = Math.abs(nearest.createdAt - at);
  for (const message of messages) {
    const next = Math.abs(message.createdAt - at);
    if (next < distance) {
      nearest = message;
      distance = next;
    }
  }
  if (differentDirectionVersion(nearest, detail)) return null;
  return nearest.id;
}

/**
 * Passed direction cards in #shapers: the current version of each artifact.
 * Earlier versions stay on the direction page.
 */
export function currentDirectionProposals<
  T extends ProposalDetailEvent & { created_at: number },
>(events: readonly T[]): T[] {
  const heads = new Map<string, T>();
  const rest: T[] = [];
  for (const event of events) {
    const detail = proposalDetail(event);
    if (detail?.kind !== "direction" || !detail.slug) {
      rest.push(event);
      continue;
    }
    const current = heads.get(detail.slug);
    if (!current || event.created_at >= current.created_at) {
      heads.set(detail.slug, event);
    }
  }
  return [...rest, ...heads.values()];
}

function messageMentions(
  message: AnchorMessage,
  detail: ProposalDetail,
): boolean {
  const tags = message.tags ?? [];
  if (
    announcementMentions(message.body, {
      kind: detail.kind,
      title: detail.title,
      slug: detail.slug,
    })
  ) {
    return true;
  }
  const revise = tagRow(tags, "revise");
  if (revise?.[1]?.trim() === detail.proposalId) return true;
  if (detail.kind === "project") {
    const project = tagRow(tags, "project");
    return project ? same(project[1], detail.title) : false;
  }
  const direction = tagRow(tags, "direction");
  if (!direction) return false;
  const slug = text(direction[1]);
  const body = text(direction[2]);
  if (detail.slug && slug && !same(slug, detail.slug)) return false;
  if (!detail.body || !body) return false;
  return same(body, detail.body);
}

/** A newer draft of the same artifact. It is not a home for an older version. */
function differentDirectionVersion(
  message: AnchorMessage,
  detail: ProposalDetail,
): boolean {
  if (detail.kind !== "direction" || !detail.slug) return false;
  const direction = tagRow(message.tags, "direction");
  if (!direction) return false;
  const slug = text(direction[1]);
  const body = text(direction[2]);
  if (!slug || !same(slug, detail.slug) || !body) return false;
  return !same(body, detail.body);
}

/**
 * The project or direction tag on an agent message.
 *
 * That message is a draft. The chat shows the draft card for it, and a
 * vote card only after the member publishes the proposal.
 */
export function chatProposalPreview(
  message: AnchorMessage,
): ChatProposalPreview | null {
  const tags = message.tags ?? [];
  const revise = tagRow(tags, "revise");
  if (revise?.[2] === "direction" || revise?.[2] === "project") {
    const kind = revise[2];
    const due = Number(tagRow(tags, "due")?.[1]);
    return {
      messageId: message.id,
      kind,
      title:
        kind === "direction"
          ? text(revise[4]) || text(revise[3]) || "Direction"
          : text(revise[3]) || "Project",
      body: text(revise[4]),
      proposalId: text(revise[1]) || null,
      dueAt: kind === "project" && Number.isFinite(due) && due > 0 ? due : null,
    };
  }
  const project = tagRow(tags, "project");
  if (project?.[1]?.trim()) {
    const due = Number(tagRow(tags, "due")?.[1]);
    return {
      messageId: message.id,
      kind: "project",
      title: project[1].trim(),
      body: text(project[2]),
      proposalId: null,
      dueAt: Number.isFinite(due) && due > 0 ? due : null,
    };
  }
  const direction = tagRow(tags, "direction");
  const slug = text(direction?.[1]);
  if (!SLUGS.has(slug)) return null;
  const body = text(direction?.[2]);
  return {
    messageId: message.id,
    kind: "direction",
    title: body || slug,
    body,
    proposalId: null,
    dueAt: null,
  };
}
