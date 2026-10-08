/**
 * Where a proposal card opens.
 *
 * An open proposal — direction or project — opens the proposal page.
 * A passed project opens the work item the relay created. A passed
 * direction stays on the proposal page, which links to the artifact
 * once that page has the agreed text.
 */

const SLUGS = new Set([
  "mission",
  "vision",
  "situation",
  "objectives",
  "strategy",
]);
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export type ProposalChatEvent = {
  id: string;
  kind: number;
  content: string;
  created_at: number;
  tags: string[][];
};

export type ProposalChatLink =
  | {
      kind: "direction";
      proposalId: string;
      slug: "mission" | "vision" | "situation" | "objectives" | "strategy";
      label: string;
    }
  | {
      kind: "project";
      proposalId: string;
      itemId: string;
      label: string;
    };

function contentOf(event: ProposalChatEvent): Record<string, unknown> | null {
  try {
    const value: unknown = JSON.parse(event.content);
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      return null;
    }
    return value as Record<string, unknown>;
  } catch {
    return null;
  }
}

function statusOf(
  event: ProposalChatEvent,
  content: Record<string, unknown>,
): string | null {
  const tagged = event.tags.find((tag) => tag[0] === "s")?.[1];
  if (tagged) return tagged;
  return typeof content.status === "string" ? content.status : null;
}

export type ProposalDestination =
  | { to: "/org/work/$itemId"; params: { itemId: string } }
  | { to: "/org/proposal/$proposalId"; params: { proposalId: string } };

/** Where a click on this proposal opens. A passed project opens the work item. */
export function proposalDestination(
  event: ProposalChatEvent,
): ProposalDestination | null {
  const link = proposalChatLink(event);
  if (link?.kind === "project") {
    return { to: "/org/work/$itemId", params: { itemId: link.itemId } };
  }
  if (event.kind !== 39102) return null;
  const proposalId =
    event.tags.find((tag) => tag[0] === "d")?.[1]?.trim() ?? "";
  if (!UUID.test(proposalId)) return null;
  return {
    to: "/org/proposal/$proposalId",
    params: { proposalId },
  };
}

/** A link for this proposal, when the chat can open something behind it. */
export function proposalChatLink(
  event: ProposalChatEvent,
): ProposalChatLink | null {
  if (event.kind !== 39102) return null;
  const content = contentOf(event);
  if (
    !content ||
    (content.kind !== "direction" && content.kind !== "project")
  ) {
    return null;
  }
  const proposalId = event.tags.find((tag) => tag[0] === "d")?.[1]?.trim();
  if (!proposalId || !UUID.test(proposalId)) return null;
  const payload =
    content.payload &&
    typeof content.payload === "object" &&
    !Array.isArray(content.payload)
      ? (content.payload as Record<string, unknown>)
      : null;
  if (content.kind === "direction") {
    const slug = typeof payload?.slug === "string" ? payload.slug : "";
    if (!SLUGS.has(slug)) return null;
    return {
      kind: "direction",
      proposalId,
      slug: slug as
        | "mission"
        | "vision"
        | "situation"
        | "objectives"
        | "strategy",
      label: `Open ${slug}`,
    };
  }
  if (statusOf(event, content) !== "passed") return null;
  const executed =
    content.executed &&
    typeof content.executed === "object" &&
    !Array.isArray(content.executed)
      ? (content.executed as Record<string, unknown>)
      : null;
  const itemId = typeof executed?.id === "string" ? executed.id.trim() : "";
  if (!UUID.test(itemId)) return null;
  const title =
    typeof payload?.title === "string" && payload.title.trim()
      ? payload.title.trim()
      : "project";
  return {
    kind: "project",
    proposalId,
    itemId,
    label: `Open ${title}`,
  };
}

/** Newest passed proposals the chat can open, capped so the strip stays short. */
export function passedProposalLinks(
  events: readonly ProposalChatEvent[],
  limit = 8,
): ProposalChatLink[] {
  const newest = new Map<string, ProposalChatEvent>();
  for (const event of events) {
    const id = event.tags.find((tag) => tag[0] === "d")?.[1];
    if (!id) continue;
    const current = newest.get(id);
    if (!current || event.created_at >= current.created_at) {
      newest.set(id, event);
    }
  }
  return [...newest.values()]
    .sort((left, right) => right.created_at - left.created_at)
    .flatMap((event) => {
      const link = proposalChatLink(event);
      return link ? [link] : [];
    })
    .slice(0, limit);
}
