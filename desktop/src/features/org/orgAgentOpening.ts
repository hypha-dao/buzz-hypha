import { normalizePubkey } from "@/shared/lib/pubkey";

/**
 * Shown under the org agent's name when its DM opens.
 * The first chat line is `ORG_AGENT_FIRST_MESSAGE`, posted by
 * `crates/buzz-org-agent/src/dm_chat.rs` `WELCOME_LINES`.
 */
export const ORG_AGENT_OPENING = {
  lead: "I'm super intelligence of your organization.",
  items: [],
} as const;

/** Posted once into an empty agent DM. Same words as `WELCOME_LINES`. */
export const ORG_AGENT_FIRST_MESSAGE = [
  "Hey, I'm glad to connect! I'm a powerful AI agent, that can take your organization to the next level. View me as Elon Musk on steroids at your service.",
  "",
  "Do you have time to set up your organization now?",
].join("\n");

export type OrgAgentOpening = {
  lead: string;
  items: readonly { title: string; detail: string }[];
};

/** Older openings. Still not a reply to the person, and still hidden. */
const LEGACY_ORG_AGENT_OPENINGS = [
  [
    "I draft for this organization — what it is for, the work, who decides, and your profile. You decide what becomes real. Nothing I write changes the org until the right person agrees.",
    "",
    "Direction — Mission, vision, where you stand, objectives, and strategy. I draft each one and say what's weak.",
    "Work — Projects, tickets, and who should hold them. Only the named person accepts.",
    "Shapers — Who decides, and how many of them must agree before something passes.",
    "Profile — What you do, the work you want, and your links, so offers go to the right person.",
    "Questions — Ask about anything the organization has already written down.",
  ].join("\n"),
  "Hey. I'm Org. Agent.",
  "I draft, you decide. Are you shaping this alone, or with other people?",
  [
    "I draft. You decide. Nothing is real until the right person agrees.",
    "",
    "Direction — Mission, vision, objectives, and strategy.",
    "Work — Projects, tickets, and who holds them.",
    "Shapers — Who decides, and how many must agree.",
    "Profile — About you, the work you want, and your links.",
    "Questions — Answers from what's already written down.",
    "",
    "Are you shaping this alone, or with other people?",
  ].join("\n"),
  [
    "I draft. You decide. Nothing is real until the right person agrees.",
    "",
    "Direction — Mission, vision, where you stand, objectives, and strategy, with an honest read on each.",
    "Work — Projects, tickets, and who holds them.",
    "Shapers — Who decides, and how many must agree.",
    "Profile — About you, the work you want, and your links.",
    "Questions — Answers from what's already written down.",
    "",
    "Are you shaping this alone, or with other people?",
  ].join("\n"),
];

export function orgAgentWelcomeText(
  opening: OrgAgentOpening = ORG_AGENT_OPENING,
) {
  const items = opening.items
    .map((item) => `${item.title} — ${item.detail}`)
    .join("\n");
  return items ? `${opening.lead}\n\n${items}` : opening.lead;
}

function isOrgAgentAuthor(
  message: { pubkey?: string; signerPubkey?: string },
  orgAgentPubkey: string,
) {
  const agent = normalizePubkey(orgAgentPubkey);
  return [message.pubkey, message.signerPubkey].some(
    (pubkey) => pubkey !== undefined && normalizePubkey(pubkey) === agent,
  );
}

/**
 * Older canned openings. The intro shows the description now, and the
 * current first DM stays in the timeline.
 */
export function isCannedOrgAgentOpening(
  message: { body: string; pubkey?: string; signerPubkey?: string },
  orgAgentPubkey: string | null,
) {
  if (!orgAgentPubkey || !isOrgAgentAuthor(message, orgAgentPubkey)) {
    return false;
  }
  const body = message.body.trim();
  return LEGACY_ORG_AGENT_OPENINGS.includes(body);
}
