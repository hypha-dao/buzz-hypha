/**
 * Project, ticket, done, and DRI from any channel the Org. Agent answers in.
 *
 * The agent tags its kind 9 reply. The person it was talking to signs the
 * command. A holder marks done and creates children; anyone seated can open
 * a project or name a DRI. Nothing is real until the relay's rule is met.
 */

import { parseSocialsTag, type OrgSocial } from "./profile";

export type ChatActMessage = {
  id: string;
  createdAt: number;
  pubkey?: string;
  signerPubkey?: string;
  tags?: readonly (readonly string[])[];
  pending?: boolean;
};

export type ChatWorkRef = {
  id: string;
  state: string;
  dri: string | null;
  parent?: string | null;
  createdBy?: string | null;
  offeredBy?: string | null;
};

export type ChatAct =
  | {
      kind: "project";
      agentEventId: string;
      title: string;
      brief: string;
      dueAt: number;
      suggestedDri: string | null;
    }
  | { kind: "done"; agentEventId: string; itemId: string }
  | {
      kind: "ticket";
      agentEventId: string;
      parentId: string;
      title: string;
      brief: string;
      dueAt: number;
      /** Absent when the ticket is left open for a match. */
      offerTo?: string;
    }
  | { kind: "dri"; agentEventId: string; itemId: string; pubkey: string }
  | { kind: "remove"; agentEventId: string; itemId: string }
  | { kind: "removeProposal"; agentEventId: string; itemId: string }
  | {
      kind: "revise";
      agentEventId: string;
      proposalId: string;
      reviseKind: "direction";
      slug: "mission" | "vision" | "situation" | "objectives" | "strategy";
      body: string;
      base: number;
    }
  | {
      kind: "revise";
      agentEventId: string;
      proposalId: string;
      reviseKind: "project";
      title: string;
      brief: string;
      dueAt: number;
    }
  | { kind: "skip"; agentEventId: string }
  | {
      kind: "profile";
      agentEventId: string;
      about: string;
      skills: string[];
      socials: OrgSocial[];
      openLimit?: number;
    };

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const DIRECTION_SLUGS = new Set([
  "mission",
  "vision",
  "situation",
  "objectives",
  "strategy",
]);

/**
 * A proposal opened from chat does not carry the opener's agree.
 * One Shaper and many follow the same path: publish, then My work.
 */
export function voteAgreeFromChat(
  _room: "dm" | "shapers" | "channel" | undefined,
): boolean {
  return false;
}

const HEX_64 = /^[0-9a-f]{64}$/;

function signerOf(message: ChatActMessage): string | undefined {
  return (message.signerPubkey ?? message.pubkey)?.trim().toLowerCase();
}

function tagRow(
  tags: readonly (readonly string[])[] | undefined,
  name: string,
): readonly string[] | undefined {
  return tags?.find((tag) => tag[0] === name);
}

function hex(value: string | undefined): string | null {
  const text = value?.trim().toLowerCase() ?? "";
  return HEX_64.test(text) ? text : null;
}

function dueOf(
  tags: readonly (readonly string[])[] | undefined,
): number | null {
  const raw = tagRow(tags, "due")?.[1];
  if (!raw) return null;
  const value = Number(raw);
  if (!Number.isFinite(value) || value <= 0) return null;
  return Math.floor(value);
}

function holds(item: ChatWorkRef | undefined, me: string): boolean {
  if (!item) return false;
  return (
    (item.state === "accepted" || item.state === "in_review") &&
    item.dri?.trim().toLowerCase() === me
  );
}

/**
 * The earliest unhandled act tag addressed to this member.
 * `wait` while Shapers or the work tree have not loaded.
 */
export function nextChatAct(input: {
  messages: readonly ChatActMessage[];
  currentPubkey: string;
  orgAgentPubkey: string;
  shaperPubkeys: readonly string[] | null;
  items: readonly ChatWorkRef[] | null;
  handledIds: ReadonlySet<string>;
  room?: "dm" | "shapers" | "channel";
}): ChatAct | "wait" | null {
  const me = input.currentPubkey.trim().toLowerCase();
  const agent = input.orgAgentPubkey.trim().toLowerCase();
  if (input.room === "shapers" && input.shaperPubkeys === null) return "wait";
  const ordered = [...input.messages].sort(
    (left, right) => left.createdAt - right.createdAt,
  );
  for (const message of ordered) {
    if (!message || message.pending || input.handledIds.has(message.id)) {
      continue;
    }
    if (signerOf(message) !== agent) continue;
    const project = tagRow(message.tags, "project");
    const done = tagRow(message.tags, "done");
    const ticket = tagRow(message.tags, "ticket");
    const dri = tagRow(message.tags, "dri");
    const remove = tagRow(message.tags, "remove");
    const revise = tagRow(message.tags, "revise");
    const profile = tagRow(message.tags, "profile");
    if (
      !project &&
      !done &&
      !ticket &&
      !dri &&
      !remove &&
      !revise &&
      !profile
    ) {
      continue;
    }
    const from = tagRow(message.tags, "from")?.[1]?.trim().toLowerCase();
    if (from !== me) return { kind: "skip", agentEventId: message.id };
    if (profile) {
      const about = profile[1] ?? "";
      if (about.length > 1000) {
        return { kind: "skip", agentEventId: message.id };
      }
      const skills = (tagRow(message.tags, "skills") ?? [])
        .slice(1)
        .map((label) => label.trim())
        .filter((label) => label.length > 0);
      const socials = parseSocialsTag(tagRow(message.tags, "socials")?.[1]);
      const limitRaw = tagRow(message.tags, "limit")?.[1] ?? "";
      const parsedLimit = Number(limitRaw);
      const openLimit =
        limitRaw.length > 0 &&
        Number.isInteger(parsedLimit) &&
        parsedLimit >= 1 &&
        parsedLimit <= 50
          ? parsedLimit
          : undefined;
      return {
        kind: "profile",
        agentEventId: message.id,
        about,
        skills,
        socials,
        openLimit,
      };
    }
    if ((project || dri || revise) && input.shaperPubkeys === null)
      return "wait";
    if ((done || ticket || dri || remove) && input.items === null)
      return "wait";
    const items = input.items ?? [];
    const byId = new Map(items.map((item) => [item.id, item]));
    if (revise) {
      const proposalId = revise[1]?.trim().toLowerCase() ?? "";
      const reviseKind = revise[2];
      const shapers = (input.shaperPubkeys ?? []).map((pubkey) =>
        pubkey.trim().toLowerCase(),
      );
      if (!UUID.test(proposalId) || !shapers.includes(me)) {
        return { kind: "skip", agentEventId: message.id };
      }
      if (reviseKind === "direction") {
        const slug = revise[3]?.trim() ?? "";
        const body = revise[4]?.trim() ?? "";
        const base = Number(tagRow(message.tags, "base")?.[1]);
        if (
          !DIRECTION_SLUGS.has(slug) ||
          body.length < 12 ||
          !Number.isInteger(base) ||
          base < 0
        ) {
          return { kind: "skip", agentEventId: message.id };
        }
        return {
          kind: "revise",
          agentEventId: message.id,
          proposalId,
          reviseKind: "direction",
          slug: slug as
            | "mission"
            | "vision"
            | "situation"
            | "objectives"
            | "strategy",
          body,
          base,
        };
      }
      if (reviseKind === "project") {
        const title = revise[3]?.trim() ?? "";
        const brief = revise[4]?.trim() || title;
        const dueAt = dueOf(message.tags);
        if (title.length < 3 || !dueAt) {
          return { kind: "skip", agentEventId: message.id };
        }
        return {
          kind: "revise",
          agentEventId: message.id,
          proposalId,
          reviseKind: "project",
          title,
          brief,
          dueAt,
        };
      }
      return { kind: "skip", agentEventId: message.id };
    }
    if (project) {
      const title = project[1]?.trim() ?? "";
      const brief = project[2]?.trim() || title;
      const dueAt = dueOf(message.tags);
      if (title.length < 3 || !dueAt) {
        return { kind: "skip", agentEventId: message.id };
      }
      const suggested = tagRow(message.tags, "p");
      const suggestedDri =
        suggested?.[3] === "suggested" ? hex(suggested[1]) : null;
      return {
        kind: "project",
        agentEventId: message.id,
        title,
        brief,
        dueAt,
        suggestedDri,
      };
    }
    if (done) {
      const item = byId.get(done[1] ?? "");
      if (!item || !holds(item, me)) {
        return { kind: "skip", agentEventId: message.id };
      }
      return { kind: "done", agentEventId: message.id, itemId: item.id };
    }
    if (ticket) {
      const parent = byId.get(ticket[1] ?? "");
      const title = ticket[2]?.trim() ?? "";
      const brief = ticket[3]?.trim() || title;
      const dueAt = dueOf(message.tags);
      const offerTo = hex(tagRow(message.tags, "p")?.[1]) ?? undefined;
      if (!parent || !holds(parent, me) || title.length < 3 || !dueAt) {
        return { kind: "skip", agentEventId: message.id };
      }
      return {
        kind: "ticket",
        agentEventId: message.id,
        parentId: parent.id,
        title,
        brief,
        dueAt,
        offerTo,
      };
    }
    if (dri) {
      const item = byId.get(dri[1] ?? "");
      const pubkey = hex(dri[2]);
      if (item?.state !== "open" || item?.dri || !pubkey || !item) {
        return { kind: "skip", agentEventId: message.id };
      }
      return {
        kind: "dri",
        agentEventId: message.id,
        itemId: item.id,
        pubkey,
      };
    }
    if (remove) {
      const item = byId.get(remove[1] ?? "");
      if (!item || item.state === "withdrawn") {
        return { kind: "skip", agentEventId: message.id };
      }
      const proposal = remove[2] === "proposal";
      const isProject = !item.parent;
      if (isProject && input.shaperPubkeys === null) return "wait";
      const shapers = (input.shaperPubkeys ?? []).map((pubkey) =>
        pubkey.trim().toLowerCase(),
      );
      if (proposal) {
        if (!isProject || shapers.length < 2 || !shapers.includes(me)) {
          return { kind: "skip", agentEventId: message.id };
        }
        return {
          kind: "removeProposal",
          agentEventId: message.id,
          itemId: item.id,
        };
      }
      if (isProject) {
        if (shapers.length !== 1 || shapers[0] !== me) {
          return { kind: "skip", agentEventId: message.id };
        }
        return { kind: "remove", agentEventId: message.id, itemId: item.id };
      }
      if (!mayRemoveTicket(item, me)) {
        return { kind: "skip", agentEventId: message.id };
      }
      return { kind: "remove", agentEventId: message.id, itemId: item.id };
    }
  }
  return null;
}

function mayRemoveTicket(item: ChatWorkRef, me: string): boolean {
  const created = item.createdBy?.trim().toLowerCase() ?? "";
  const offered = item.offeredBy?.trim().toLowerCase() ?? "";
  if (created === me || (HEX_64.test(offered) && offered === me)) return true;
  return created.length === 0 && offered.length === 0;
}
