/**
 * What a proposal card expands to, and which #shapers message it hangs on.
 *
 * The 39102 is not a channel message. The agent's kind 9 carries the
 * direction, project, or revise tag, and the card sits under that message.
 */

import { announcementMentions, directionBodyLines } from "./chatDraft";

const SLUGS = new Set([
  "mission",
  "vision",
  "situation",
  "objectives",
  "strategy",
]);

const DIRECTION_NAME: Record<string, string> = {
  mission: "Mission",
  vision: "Vision",
  situation: "Situation",
  objectives: "Objectives",
  strategy: "Strategy",
};

/** Kickers that name the column or a generic bucket, not a person. */
const QUIET_KICKERS = new Set([
  "",
  "Needs your answer",
  "Direction",
  "Project",
  "Up for a vote",
  "Proposal",
]);

const PROPOSAL_KIND_LABEL: Record<string, string> = {
  project: "Project proposal",
  dri: "DRI proposal",
  money: "Money proposal",
  join: "Join proposal",
  shapers: "Shapers proposal",
  withdraw: "Withdraw proposal",
};

export type DirectionSlugName =
  | "mission"
  | "vision"
  | "situation"
  | "objectives"
  | "strategy";

export type ProposalDetail = {
  proposalId: string;
  kind: "direction" | "project";
  title: string;
  body: string;
  slug: DirectionSlugName | null;
  dueAt: number | null;
};

/** What the proposal page shows. A direction proposal is not a work item. */
export type ProposalPageModel = {
  eyebrow: string;
  title: string;
  brief: string;
  lines: { id: string; text: string }[];
  doorTitle: string;
  parentLabel: string;
  parentTo: "/org/my-work" | "/org/work";
  crumb: string;
  status: string;
  dueAt: number | null;
  suggestedDri: string | null;
  directionSlug: DirectionSlugName | null;
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
      slug: directionSlug(slug),
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

/**
 * Objectives and strategy proposals, one entry per line, for the card face.
 * Empty when the proposal is one block of text.
 */
export function proposalClaimLines(event: ProposalDetailEvent): string[] {
  const detail = proposalDetail(event);
  if (
    detail?.kind !== "direction" ||
    (detail.slug !== "objectives" && detail.slug !== "strategy")
  ) {
    return [];
  }
  const content = objectOf(event.content);
  const stored = lineTexts(content ? payloadOf(content)?.lines : undefined).map(
    (line) => line.text,
  );
  const lines =
    stored.length > 0 ? stored : directionBodyLines(detail.slug, detail.body);
  return lines.length > 1 ? lines : [];
}

function directionSlug(slug: string): DirectionSlugName | null {
  return SLUGS.has(slug) ? (slug as DirectionSlugName) : null;
}

function directionTitle(slug: string): string {
  return DIRECTION_NAME[slug] ?? "Direction";
}

/** The body often stores the lines again as one paragraph. */
function briefRepeatsLines(
  brief: string,
  lines: readonly { text: string }[],
): boolean {
  if (lines.length === 0 || brief.trim().length === 0) return false;
  const plain = (value: string) =>
    value
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, " ")
      .replace(/\s+/g, " ")
      .trim();
  return plain(brief) === plain(lines.map((line) => line.text).join(" "));
}

function lineTexts(value: unknown): { id: string; text: string }[] {
  if (!Array.isArray(value)) return [];
  const lines: { id: string; text: string }[] = [];
  for (const line of value) {
    if (!line || typeof line !== "object" || Array.isArray(line)) continue;
    const row = line as Record<string, unknown>;
    const body = text(row.text);
    if (!body) continue;
    const id = text(row.id) || `line-${lines.length + 1}`;
    lines.push({ id: `${id}-${lines.length + 1}`, text: body });
  }
  return lines;
}

function statusOf(
  event: ProposalDetailEvent,
  content: Record<string, unknown>,
) {
  const tagged = event.tags.find((tag) => tag[0] === "s")?.[1];
  if (tagged) return tagged;
  return typeof content.status === "string" ? content.status : "open";
}

/**
 * The label on a proposal card. Mission, vision, objectives, and strategy
 * each name themselves; a project proposal does not read as a direction.
 */
export function proposalKindLabel(event: ProposalDetailEvent): string | null {
  if (event.kind !== 39102) return null;
  const content = objectOf(event.content);
  if (!content || typeof content.kind !== "string") return "Proposal";
  if (content.kind === "direction") {
    const slug = text(payloadOf(content)?.slug);
    const name = DIRECTION_NAME[slug];
    return name ? `${name} proposal` : "Direction proposal";
  }
  return PROPOSAL_KIND_LABEL[content.kind] ?? "Proposal";
}

/**
 * Kind first, then who asked when that is a person rather than the column.
 */
export function proposalCardFace(
  event: ProposalDetailEvent,
  kicker: string,
): { label: string; asker: string | null } {
  const kind = proposalKindLabel(event);
  if (!kind) return { label: kicker, asker: null };
  const asker =
    kicker && !QUIET_KICKERS.has(kicker) && kicker !== kind ? kicker : null;
  return { label: kind, asker };
}

/** Copy for the proposal page. Direction opens here, not on an empty artifact. */
export function proposalPage(
  event: ProposalDetailEvent,
): ProposalPageModel | null {
  if (event.kind !== 39102) return null;
  const content = objectOf(event.content);
  if (!content) return null;
  const payload = payloadOf(content);
  const status = statusOf(event, content);
  const kind = typeof content.kind === "string" ? content.kind : "proposal";
  if (kind === "direction") {
    const slug = text(payload?.slug);
    const name = directionTitle(slug);
    const body = text(payload?.body);
    const lines = lineTexts(payload?.lines);
    return {
      eyebrow: directionSlug(slug) ? `${name} proposal` : "Direction proposal",
      title: lines.length > 0 ? name : body || name,
      brief: lines.length > 0 && !briefRepeatsLines(body, lines) ? body : "",
      lines,
      doorTitle: name,
      parentLabel: "My Work",
      parentTo: "/org/my-work",
      crumb: name,
      status,
      dueAt: null,
      suggestedDri: null,
      directionSlug: directionSlug(slug),
    };
  }
  const due = payload?.due_at;
  const suggested = payload?.suggested_dri;
  const title = text(payload?.title) || text(payload?.body) || "Proposal";
  const briefRaw = text(payload?.brief);
  return {
    eyebrow: proposalKindLabel(event) ?? "Proposal",
    title,
    brief: briefRaw === title ? "" : briefRaw,
    lines: [],
    doorTitle: "Work",
    parentLabel: "Work",
    parentTo: "/org/work",
    crumb: title,
    status,
    dueAt: typeof due === "number" && Number.isFinite(due) ? due : null,
    suggestedDri: typeof suggested === "string" ? suggested : null,
    directionSlug: null,
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
