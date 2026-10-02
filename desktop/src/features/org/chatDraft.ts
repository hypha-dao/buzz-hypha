/**
 * A proposal the Org. Agent put in the chat, before anyone publishes it.
 *
 * The same card is used in the agent's DM and in #shapers, and for every
 * proposal kind the agent can draft. Publishing does not attach an agree:
 * the proposal then sits on My work for every Shaper, one or many.
 */

import {
  buildIoDirectionPropose,
  buildIoDriPropose,
  buildIoProjectPropose,
  buildIoShapersPropose,
  buildIoWithdrawPropose,
  type DirectionSlug,
  type UnsignedOrgCommand,
} from "./commands";

const SLUGS = new Set<DirectionSlug>([
  "mission",
  "vision",
  "objectives",
  "strategy",
]);
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const HEX_64 = /^[0-9a-f]{64}$/;

export type ChatDraftMessage = {
  id: string;
  createdAt: number;
  pubkey?: string;
  signerPubkey?: string;
  body?: string;
  tags?: readonly (readonly string[])[];
  pending?: boolean;
};

export type ChatDraft =
  | {
      kind: "project";
      messageId: string;
      createdAt: number;
      from: string;
      title: string;
      brief: string;
      dueAt: number;
      suggestedDri: string | null;
    }
  | {
      kind: "direction";
      messageId: string;
      createdAt: number;
      from: string;
      slug: DirectionSlug;
      body: string;
    }
  | {
      kind: "dri";
      messageId: string;
      createdAt: number;
      from: string;
      itemId: string;
      pubkey: string;
    }
  | {
      kind: "revise-direction";
      messageId: string;
      createdAt: number;
      from: string;
      proposalId: string;
      slug: DirectionSlug;
      body: string;
      base: number;
    }
  | {
      kind: "revise-project";
      messageId: string;
      createdAt: number;
      from: string;
      proposalId: string;
      title: string;
      brief: string;
      dueAt: number;
    }
  | {
      kind: "remove-project";
      messageId: string;
      createdAt: number;
      from: string;
      itemId: string;
    }
  | {
      kind: "shapers-add" | "shapers-remove";
      messageId: string;
      createdAt: number;
      from: string;
      pubkey: string;
      why: string;
    }
  | {
      kind: "shapers-rules";
      messageId: string;
      createdAt: number;
      from: string;
      /** Kinds the agent named. The dialog fills the rest from the live rules. */
      rules: Partial<Record<RuleKind, string>>;
      decisionWindowSecs: number | null;
      offerWindowSecs: number | null;
    }
  | {
      kind: "shapers-agent";
      messageId: string;
      createdAt: number;
      from: string;
      /** Absent means return to the hosted agent. */
      pubkey: string | null;
      why: string;
    };

export const RULE_KINDS = [
  "direction",
  "project",
  "dri",
  "shapers",
  "money",
  "join",
] as const;

export type RuleKind = (typeof RULE_KINDS)[number];

export function emptyRules(): Record<RuleKind, string> {
  return {
    direction: "majority",
    project: "majority",
    dri: "majority",
    shapers: "majority",
    money: "majority",
    join: "majority",
  };
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

function signerOf(message: ChatDraftMessage): string | undefined {
  return (message.signerPubkey ?? message.pubkey)?.trim().toLowerCase();
}

/** One draft on an agent message, when the tags name a proposal. */
export function chatDraftFromMessage(
  message: ChatDraftMessage,
  orgAgentPubkey: string,
): ChatDraft | null {
  if (message.pending) return null;
  if (signerOf(message) !== orgAgentPubkey.trim().toLowerCase()) return null;
  const from = hex(tagRow(message.tags, "from")?.[1]);
  if (!from) return null;
  const tags = message.tags;
  const revise = tagRow(tags, "revise");
  if (revise) {
    const proposalId = revise[1]?.trim().toLowerCase() ?? "";
    if (!UUID.test(proposalId)) return null;
    if (revise[2] === "direction") {
      const slug = revise[3]?.trim() ?? "";
      const body = revise[4]?.trim() ?? "";
      const base = Number(tagRow(tags, "base")?.[1]);
      if (!SLUGS.has(slug as DirectionSlug) || body.length < 12) return null;
      if (!Number.isInteger(base) || base < 0) return null;
      return {
        kind: "revise-direction",
        messageId: message.id,
        createdAt: message.createdAt,
        from,
        proposalId,
        slug: slug as DirectionSlug,
        body,
        base,
      };
    }
    if (revise[2] === "project") {
      const title = revise[3]?.trim() ?? "";
      const brief = revise[4]?.trim() || title;
      const dueAt = dueOf(tags);
      if (title.length < 3 || !dueAt) return null;
      return {
        kind: "revise-project",
        messageId: message.id,
        createdAt: message.createdAt,
        from,
        proposalId,
        title,
        brief,
        dueAt,
      };
    }
    return null;
  }
  const project = tagRow(tags, "project");
  if (project?.[1]?.trim()) {
    const title = project[1].trim();
    const brief = project[2]?.trim() || title;
    const dueAt = dueOf(tags);
    if (title.length < 3 || !dueAt) return null;
    const suggested = tagRow(tags, "p");
    const suggestedDri =
      suggested?.[3] === "suggested" ? hex(suggested[1]) : null;
    return {
      kind: "project",
      messageId: message.id,
      createdAt: message.createdAt,
      from,
      title,
      brief,
      dueAt,
      suggestedDri,
    };
  }
  const direction = tagRow(tags, "direction");
  const slug = direction?.[1]?.trim() ?? "";
  const body = direction?.[2]?.trim() ?? "";
  if (SLUGS.has(slug as DirectionSlug) && body.length >= 12) {
    return {
      kind: "direction",
      messageId: message.id,
      createdAt: message.createdAt,
      from,
      slug: slug as DirectionSlug,
      body,
    };
  }
  const dri = tagRow(tags, "dri");
  if (dri) {
    const itemId = dri[1]?.trim() ?? "";
    const pubkey = hex(dri[2]);
    if (!UUID.test(itemId) || !pubkey) return null;
    return {
      kind: "dri",
      messageId: message.id,
      createdAt: message.createdAt,
      from,
      itemId,
      pubkey,
    };
  }
  const remove = tagRow(tags, "remove");
  if (remove?.[2] === "proposal") {
    const itemId = remove[1]?.trim() ?? "";
    if (!UUID.test(itemId)) return null;
    return {
      kind: "remove-project",
      messageId: message.id,
      createdAt: message.createdAt,
      from,
      itemId,
    };
  }
  const shapers = tagRow(tags, "shapers");
  if (shapers?.[1] === "add" || shapers?.[1] === "remove") {
    const pubkey = hex(shapers[2]);
    if (!pubkey) return null;
    return {
      kind: shapers[1] === "add" ? "shapers-add" : "shapers-remove",
      messageId: message.id,
      createdAt: message.createdAt,
      from,
      pubkey,
      why: shapers[3]?.trim() || "Asked in chat.",
    };
  }
  if (shapers?.[1] === "rules") {
    const rules = parseRuleOverlay(shapers[2]);
    if (!rules) return null;
    return {
      kind: "shapers-rules",
      messageId: message.id,
      createdAt: message.createdAt,
      from,
      rules,
      decisionWindowSecs: positiveSecs(shapers[3]),
      offerWindowSecs: positiveSecs(shapers[4]),
    };
  }
  if (shapers?.[1] === "agent") {
    const raw = shapers[2]?.trim() ?? "";
    const pubkey = raw ? hex(raw) : null;
    if (raw && !pubkey) return null;
    return {
      kind: "shapers-agent",
      messageId: message.id,
      createdAt: message.createdAt,
      from,
      pubkey,
      why: shapers[3]?.trim() || "Asked in chat.",
    };
  }
  return null;
}

function parseRuleOverlay(
  raw: string | undefined,
): Partial<Record<RuleKind, string>> | null {
  if (!raw?.trim()) return {};
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!parsed || typeof parsed !== "object") return null;
  const overlay: Partial<Record<RuleKind, string>> = {};
  for (const key of RULE_KINDS) {
    const value = (parsed as Record<string, unknown>)[key];
    if (value === undefined) continue;
    const token = ruleToken(value);
    if (!token) return null;
    overlay[key] = token;
  }
  return overlay;
}

function ruleToken(value: unknown): string | null {
  if (typeof value === "number" && Number.isInteger(value) && value >= 1) {
    return String(value);
  }
  if (typeof value !== "string") return null;
  const text = value.trim().toLowerCase();
  if (text === "majority" || text === "all") return text;
  if (/^[1-9]\d*$/.test(text)) return String(Number(text));
  return null;
}

function positiveSecs(raw: string | undefined): number | null {
  if (!raw?.trim()) return null;
  const value = Number(raw);
  if (!Number.isFinite(value) || value <= 0) return null;
  return Math.floor(value);
}

export type OpenWorkRef = {
  id: string;
  title: string;
  state: string;
  dri: string | null;
};

const DRI_MARKERS = [
  " as the dri ",
  " as dri ",
  " as the holder ",
  " as holder ",
  " the dri for ",
  " the dri of ",
  " dri for ",
  " dri of ",
  " holder of ",
  " holder for ",
];

const WHO_SKIP = new Set([
  "as",
  "the",
  "a",
  "an",
  "to",
  "for",
  "of",
  "set",
  "name",
  "make",
  "proposal",
  "draft",
  "and",
]);

function asksToNameDri(text: string): boolean {
  const lower = text.toLowerCase();
  if (!lower.includes("dri") && !lower.includes("holder")) return false;
  return ["set ", "name ", "make ", "assign ", "proposal", "draft"].some(
    (word) => lower.includes(word),
  );
}

function whoWord(before: string): string | null {
  const last = before.trim().split(/\s+/).at(-1);
  if (!last) return null;
  const word = last.replace(/^[,.!?:“”"]+|[,.!?:“”"]+$/g, "");
  if (!word || WHO_SKIP.has(word.toLowerCase())) return null;
  return word;
}

function itemWords(after: string): string | null {
  let rest = after.trim();
  for (const prefix of ["for ", "of ", "on "]) {
    if (rest.toLowerCase().startsWith(prefix)) {
      rest = rest.slice(prefix.length).trim();
      break;
    }
  }
  rest = rest.replace(/^[,.!?:“”"]+|[,.!?:“”"]+$/g, "");
  const sentence = rest.split(/[.!?]/)[0]?.trim() ?? "";
  if (sentence.length < 3) return null;
  return sentence;
}

/** Who and which project a DRI request names. */
export function driRequest(text: string): { who: string; item: string } | null {
  if (!asksToNameDri(text)) return null;
  const lower = text.toLowerCase();
  let found: { who: string; item: string } | null = null;
  for (const marker of DRI_MARKERS) {
    const at = lower.indexOf(marker);
    if (at < 0) continue;
    const who = whoWord(text.slice(0, at));
    const item = itemWords(text.slice(at + marker.length));
    if (!who || !item) continue;
    const itemLower = item.toLowerCase();
    if (
      itemLower.includes("dri") ||
      itemLower.includes("holder") ||
      itemLower.includes("proposal")
    ) {
      continue;
    }
    found = { who, item };
  }
  return found;
}

function claimsDriDraft(body: string): boolean {
  const lower = body.toLowerCase();
  return (
    lower.includes("dri proposal") ||
    lower.includes("drafting a dri") ||
    lower.includes("as the holder of") ||
    lower.includes("name a holder for")
  );
}

function selfWord(who: string): boolean {
  const word = who.trim().toLowerCase();
  return word === "me" || word === "myself" || word === "i" || word === "you";
}

function matchOpenUnheld(
  query: string,
  items: readonly OpenWorkRef[],
): OpenWorkRef | null {
  const wanted = query.trim().toLowerCase().replace(/\s+/g, " ");
  if (!wanted) return null;
  const open = items.filter((item) => item.state === "open" && !item.dri);
  const exact = open.filter(
    (item) => item.title.trim().toLowerCase().replace(/\s+/g, " ") === wanted,
  );
  if (exact.length === 1) return exact[0] ?? null;
  if (exact.length > 1 || wanted.length < 4) return null;
  const prefixed = open.filter((item) => {
    const title = item.title.trim().toLowerCase().replace(/\s+/g, " ");
    return title.startsWith(wanted) || wanted.startsWith(title);
  });
  return prefixed.length === 1 ? (prefixed[0] ?? null) : null;
}

/**
 * A DRI draft the agent described without tags. The card still opens on
 * that message, from the request in the line before it.
 */
export function recoverHollowDriDrafts(
  messages: readonly ChatDraftMessage[],
  orgAgentPubkey: string,
  items: readonly OpenWorkRef[],
): ChatDraft[] {
  const agent = orgAgentPubkey.trim().toLowerCase();
  const ordered = [...messages].sort(
    (left, right) => left.createdAt - right.createdAt,
  );
  const found: ChatDraft[] = [];
  let previousHuman: ChatDraftMessage | null = null;
  for (const message of ordered) {
    const author = signerOf(message);
    if (author !== agent) {
      if (!message.pending) previousHuman = message;
      continue;
    }
    if (message.pending || tagRow(message.tags, "dri")) continue;
    if (!claimsDriDraft(message.body ?? "")) continue;
    const human = previousHuman;
    const speaker = human ? hex(human.pubkey ?? human.signerPubkey) : null;
    if (!human || !speaker) continue;
    const request =
      driRequest(human.body ?? "") ?? driRequest(message.body ?? "");
    if (!request || !selfWord(request.who)) continue;
    const item = matchOpenUnheld(request.item, items);
    if (!item) continue;
    found.push({
      kind: "dri",
      messageId: message.id,
      createdAt: message.createdAt,
      from: speaker,
      itemId: item.id,
      pubkey: speaker,
    });
  }
  return found;
}

/** Drafts that still need a publish click. The newest of each subject wins. */
export function openChatDrafts(
  messages: readonly ChatDraftMessage[],
  orgAgentPubkey: string,
  publishedIds: ReadonlySet<string>,
  items: readonly OpenWorkRef[] = [],
): ChatDraft[] {
  const parsed: ChatDraft[] = [];
  const ordered = [...messages].sort(
    (left, right) => left.createdAt - right.createdAt,
  );
  for (const message of ordered) {
    const draft = chatDraftFromMessage(message, orgAgentPubkey);
    if (draft) parsed.push(draft);
  }
  const taggedSeries = new Set(parsed.map((draft) => draftSeries(draft)));
  for (const draft of recoverHollowDriDrafts(ordered, orgAgentPubkey, items)) {
    if (taggedSeries.has(draftSeries(draft))) continue;
    if (parsed.some((existing) => existing.messageId === draft.messageId)) {
      continue;
    }
    parsed.push(draft);
  }
  const publishedAt = new Map<string, number>();
  for (const draft of parsed) {
    if (!publishedIds.has(draft.messageId)) continue;
    const series = draftSeries(draft);
    const at = publishedAt.get(series) ?? 0;
    if (draft.createdAt >= at) publishedAt.set(series, draft.createdAt);
  }
  const newest = new Map<string, ChatDraft>();
  for (const draft of parsed) {
    const published = publishedAt.get(draftSeries(draft));
    if (published !== undefined && draft.createdAt <= published) continue;
    newest.set(draftSeries(draft), draft);
  }
  return [...newest.values()].sort(
    (left, right) => left.createdAt - right.createdAt,
  );
}

export function draftSeries(draft: ChatDraft): string {
  switch (draft.kind) {
    case "project":
      return "project";
    case "direction":
      return `direction:${draft.slug}`;
    case "dri":
      return `dri:${draft.itemId}`;
    case "revise-direction":
    case "revise-project":
      return `revise:${draft.proposalId}`;
    case "remove-project":
      return `remove:${draft.itemId}`;
    case "shapers-add":
      return `shapers-add:${draft.pubkey}`;
    case "shapers-remove":
      return `shapers-remove:${draft.pubkey}`;
    case "shapers-rules":
      return "shapers-rules";
    case "shapers-agent":
      return "shapers-agent";
  }
}

export function draftKindLabel(draft: ChatDraft): string {
  switch (draft.kind) {
    case "project":
    case "revise-project":
      return "Project";
    case "direction":
    case "revise-direction":
      return draft.slug.charAt(0).toUpperCase() + draft.slug.slice(1);
    case "dri":
      return "DRI";
    case "remove-project":
      return "Remove project";
    case "shapers-add":
      return "Add a Shaper";
    case "shapers-remove":
      return "Remove a Shaper";
    case "shapers-rules":
      return "Decision rules";
    case "shapers-agent":
      return "Org agent";
  }
}

export function draftTitle(
  draft: ChatDraft,
  itemTitle?: string | null,
): string {
  switch (draft.kind) {
    case "project":
    case "revise-project":
      return draft.title;
    case "direction":
    case "revise-direction":
      return draft.body;
    case "dri":
      return itemTitle?.trim() || "Name a holder";
    case "remove-project":
      return itemTitle?.trim() || "Remove a project";
    case "shapers-add":
      return "Add a Shaper";
    case "shapers-remove":
      return "Remove a Shaper";
    case "shapers-rules":
      return "Change the decision rules";
    case "shapers-agent":
      return draft.pubkey ? "Choose an org agent" : "Use the hosted org agent";
  }
}

function markdownLiteral(text: string): string {
  return text.replace(/[*_`[\]]/g, (mark) => `\\${mark}`);
}

/**
 * The chat line for a DRI draft. Names the project, and the person when we
 * know their name. A missing name stays out of the sentence.
 */
export function driDraftSentence(
  projectTitle: string,
  holderName: string | null,
): string {
  const project = markdownLiteral(projectTitle.trim() || "this project");
  const holder = holderName?.trim() ?? "";
  if (holder) {
    return `A draft to name ${markdownLiteral(holder)} as the holder of “${project}”. Open it, then publish.`;
  }
  return `A draft to name a holder for “${project}”. Open it, then publish.`;
}

type ChatNameProfile = {
  displayName?: string | null;
  name?: string | null;
  nip05Handle?: string | null;
};

/** People a draft or proposal names: suggested holder, DRI, Shaper, org agent. */
export function holderPubkeysFromTags(
  tags: readonly (readonly string[])[] | undefined,
): string[] {
  const found = new Set<string>();
  for (const tag of tags ?? []) {
    let raw: string | undefined;
    if (tag[0] === "p") raw = tag[1];
    else if (tag[0] === "dri") raw = tag[2];
    else if (
      tag[0] === "shapers" &&
      (tag[1] === "add" || tag[1] === "remove" || tag[1] === "agent")
    ) {
      raw = tag[2];
    }
    const key = raw?.trim().toLowerCase() ?? "";
    if (HEX_64.test(key)) found.add(key);
  }
  return [...found];
}

/**
 * The name to show for a suggested holder. A missing profile stays unnamed
 * so the chat does not fall back to an npub or a pubkey prefix.
 */
export function chatHolderLabel(
  pubkey: string,
  profiles: Record<string, ChatNameProfile> | undefined,
): string | null {
  const key = pubkey.trim().toLowerCase();
  const profile = profiles?.[key];
  const label =
    profile?.displayName?.trim() ||
    profile?.name?.trim() ||
    profile?.nip05Handle?.trim() ||
    "";
  if (!label || label.toLowerCase().startsWith("npub")) return null;
  if (label.toLowerCase() === key || label.toLowerCase() === key.slice(0, 8)) {
    return null;
  }
  return label;
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function replaceHolderToken(body: string, token: string, name: string): string {
  const pattern = new RegExp(
    `(?<![0-9a-fA-F])${escapeRegExp(token)}(?![0-9a-fA-F])`,
    "gi",
  );
  return body.replace(pattern, name);
}

const HOLDER_PREFIX = /(?<![0-9a-fA-F])[0-9a-fA-F]{8}(?![0-9a-fA-F])/g;

/** The line is opening an offer, not asking whether to offer the ticket. */
export function ticketAlreadyOffered(body: string): boolean {
  const lower = body.toLowerCase();
  if (lower.includes("want me to offer") || lower.includes("want me to open")) {
    return false;
  }
  return (
    lower.includes("offered to ") ||
    lower.startsWith("opening the ticket") ||
    lower.startsWith("opening that ticket") ||
    lower.startsWith("offering ") ||
    lower.startsWith("offering “") ||
    lower.startsWith('offering "')
  );
}

/** Drop "Suggested holder" once the ticket is already offered. */
export function omitSuggestedHolderWhenOffered(body: string): string {
  if (!ticketAlreadyOffered(body)) return body;
  return body
    .replace(/\s*Suggested holder:[^.]*\.?/gi, " ")
    .replace(/[ \t]{2,}/g, " ")
    .replace(/\s+\./g, ".")
    .trim();
}

/**
 * Chat lines that still say a pubkey prefix name the person instead.
 * A draft waiting for yes often has the prefix in the sentence and no
 * holder tag yet, so a unique match among known profiles is enough.
 */
export function nameHoldersInChat(
  body: string,
  tags: readonly (readonly string[])[] | undefined,
  profiles: Record<string, ChatNameProfile> | undefined,
): string {
  const labels = new Map<string, string>();
  let next = omitSuggestedHolderWhenOffered(body);
  const remember = (pubkey: string) => {
    const key = pubkey.trim().toLowerCase();
    if (!HEX_64.test(key) || labels.has(key)) return;
    const label = chatHolderLabel(key, profiles);
    if (label) labels.set(key, label);
  };
  for (const pubkey of holderPubkeysFromTags(tags)) remember(pubkey);
  for (const pubkey of Object.keys(profiles ?? {})) remember(pubkey);
  const prefixOwners = new Map<string, string[]>();
  for (const pubkey of labels.keys()) {
    const prefix = pubkey.slice(0, 8);
    const owners = prefixOwners.get(prefix) ?? [];
    owners.push(pubkey);
    prefixOwners.set(prefix, owners);
  }
  for (const [pubkey, label] of labels) {
    next = replaceHolderToken(next, pubkey, label);
    const owners = prefixOwners.get(pubkey.slice(0, 8)) ?? [];
    if (owners.length === 1) {
      next = replaceHolderToken(next, pubkey.slice(0, 8), label);
    }
  }
  return next.replace(HOLDER_PREFIX, (token) => {
    const owners = prefixOwners.get(token.toLowerCase()) ?? [];
    if (owners.length !== 1) return token;
    return labels.get(owners[0]) ?? token;
  });
}

/** The #shapers message posted after publish. The card under it opens the proposal. */
export function proposalAnnouncement(label: string, title: string): string {
  return `Opened a ${label} proposal: ${title}.`;
}

const PROPOSAL_OPEN_LINK =
  /\n*\[Open it(?: in My work)?\]\(\/org\/(?:my-work|proposal\/[0-9a-f-]{36}|work\/[0-9a-f-]{36})\)/g;

/** Older announcements carried a link. The card is the way in now. */
export function stripProposalOpenLink(body: string): string {
  if (!body.includes("[Open it")) return body;
  return body
    .replace(PROPOSAL_OPEN_LINK, "")
    .replace(/\n{3,}/g, "\n\n")
    .trimEnd();
}

export function announcementLabel(draft: ChatDraft): string {
  switch (draft.kind) {
    case "project":
    case "revise-project":
      return "project";
    case "direction":
    case "revise-direction":
      return draft.slug;
    case "dri":
      return "DRI";
    case "remove-project":
      return "project removal";
    case "shapers-add":
      return "Shapers add";
    case "shapers-remove":
      return "Shapers remove";
    case "shapers-rules":
      return "decision rules";
    case "shapers-agent":
      return "org agent";
  }
}

/** True when this chat line is the publish announcement for that proposal. */
export function announcementMentions(
  body: string | undefined,
  detail: { kind: string; title: string; slug: string | null },
): boolean {
  if (!body?.includes("Opened a ") || !body.includes("proposal:")) return false;
  if (detail.kind === "project") {
    return (
      body.includes(`Opened a project proposal: ${detail.title}.`) ||
      body.includes(`Opened a project removal proposal: ${detail.title}.`)
    );
  }
  if (detail.kind === "direction" && detail.slug) {
    return (
      body.includes(`Opened a ${detail.slug} proposal:`) &&
      body.includes(detail.title)
    );
  }
  return false;
}

export type DraftPublishInput = {
  draft: ChatDraft;
  title: string;
  brief: string;
  body: string;
  dueAt: number;
  why: string;
  /** Live direction version. A new direction proposes `version` as base. */
  directionBase: number;
  lines?: { text: string }[];
  rules?: Record<string, string>;
  decisionWindowSecs?: number | null;
  offerWindowSecs?: number | null;
  agentPubkey?: string | null;
  /** Project holder. `null` publishes with nobody. Omitted keeps the chat tag. */
  suggestedDri?: string | null;
  /** DRI holder chosen in the draft. Omitted keeps the chat tag. */
  holder?: string;
};

function publishedHolder(
  chosen: string | null | undefined,
  fromChat: string | null,
): string | undefined {
  if (chosen === null || chosen === "") return undefined;
  if (typeof chosen === "string") return chosen;
  return fromChat ?? undefined;
}

/** The command Publish signs. No agree tag — My work is where Shapers vote. */
export function commandForDraft(input: DraftPublishInput): UnsignedOrgCommand {
  const { draft } = input;
  switch (draft.kind) {
    case "project":
      return buildIoProjectPropose({
        title: input.title,
        brief: input.brief,
        dueAt: input.dueAt,
        suggestedDri: publishedHolder(input.suggestedDri, draft.suggestedDri),
        voteAgree: false,
      });
    case "revise-project":
      return buildIoProjectPropose({
        title: input.title,
        brief: input.brief,
        dueAt: input.dueAt,
        revises: draft.proposalId,
        voteAgree: false,
      });
    case "direction":
      return buildIoDirectionPropose({
        slug: draft.slug,
        base: input.directionBase,
        body: input.body,
        lines: input.lines,
        why: input.why.trim() || "Drafted in chat.",
        voteAgree: false,
      });
    case "revise-direction":
      return buildIoDirectionPropose({
        slug: draft.slug,
        base: draft.base,
        body: input.body,
        lines: input.lines,
        why: input.why.trim() || "Revised in chat.",
        revises: draft.proposalId,
        voteAgree: false,
      });
    case "dri":
      return buildIoDriPropose({
        item: draft.itemId,
        pubkey: input.holder?.trim() || draft.pubkey,
        why: input.why.trim() || "Named in chat.",
        voteAgree: false,
      });
    case "remove-project":
      return buildIoWithdrawPropose({
        item: draft.itemId,
        why: input.why.trim() || "Removed in chat.",
        voteAgree: false,
      });
    case "shapers-add":
      return buildIoShapersPropose(
        {
          op: "add",
          pubkey: draft.pubkey,
          why: input.why.trim() || draft.why,
        },
        false,
      );
    case "shapers-remove":
      return buildIoShapersPropose(
        {
          op: "remove",
          pubkey: draft.pubkey,
          why: input.why.trim() || draft.why,
        },
        false,
      );
    case "shapers-rules": {
      const decision = positiveOrOmit(input.decisionWindowSecs);
      const offer = positiveOrOmit(input.offerWindowSecs);
      return buildIoShapersPropose(
        {
          op: "rules",
          rules: input.rules ?? { ...emptyRules(), ...draft.rules },
          ...(decision === undefined ? {} : { decision_window_secs: decision }),
          ...(offer === undefined ? {} : { offer_window_secs: offer }),
        },
        false,
      );
    }
    case "shapers-agent": {
      const pubkey = (input.agentPubkey ?? draft.pubkey)?.trim();
      return buildIoShapersPropose(
        {
          op: "agent",
          ...(pubkey ? { pubkey } : {}),
          why: input.why.trim() || draft.why,
        },
        false,
      );
    }
  }
}

function positiveOrOmit(value: number | null | undefined): number | undefined {
  if (value == null || !Number.isFinite(value) || value <= 0) return undefined;
  return Math.floor(value);
}

export function dateInputValue(unix: number): string {
  return new Date(unix * 1000).toISOString().slice(0, 10);
}

/** Keep the original timestamp when the calendar day did not change. */
export function dateInputToUnix(value: string, previous: number): number {
  if (dateInputValue(previous) === value) return previous;
  const [year, month, day] = value.split("-").map(Number);
  if (!year || !month || !day) return previous;
  return Math.floor(Date.UTC(year, month - 1, day, 12) / 1000);
}
