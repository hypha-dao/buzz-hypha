import { normalizePubkey } from "@/shared/lib/pubkey";

/**
 * What the org agent is for, shown the moment its DM opens.
 * `crates/buzz-org-agent/src/dm_chat.rs` `WELCOME_LINES` is this same text.
 */
export const ORG_AGENT_OPENING = {
  lead: "I draft for this organization — what it is for, the work, who decides, and your profile. You decide what becomes real. Nothing I write changes the org until the right person agrees.",
  items: [
    {
      title: "Direction",
      detail:
        "Mission, vision, where you stand, objectives, and strategy. I draft each one and say what's weak.",
    },
    {
      title: "Work",
      detail:
        "Projects, tickets, and who should hold them. Only the named person accepts.",
    },
    {
      title: "Shapers",
      detail:
        "Who decides, and how many of them must agree before something passes.",
    },
    {
      title: "Profile",
      detail:
        "What you do, the work you want, and your links, so offers go to the right person.",
    },
    {
      title: "Questions",
      detail: "Ask about anything the organization has already written down.",
    },
  ],
} as const;

export type OrgAgentOpening = {
  lead: string;
  items: readonly { title: string; detail: string }[];
};

/** Older openings. Still not a reply to the person. */
const LEGACY_ORG_AGENT_OPENINGS = [
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
  return `${opening.lead}\n\n${items}`;
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

/** The canned opening the agent used to post. The DM intro says it now. */
export function isCannedOrgAgentOpening(
  message: { body: string; pubkey?: string; signerPubkey?: string },
  orgAgentPubkey: string | null,
) {
  if (!orgAgentPubkey || !isOrgAgentAuthor(message, orgAgentPubkey)) {
    return false;
  }
  const body = message.body.trim();
  return (
    body === orgAgentWelcomeText() || LEGACY_ORG_AGENT_OPENINGS.includes(body)
  );
}
