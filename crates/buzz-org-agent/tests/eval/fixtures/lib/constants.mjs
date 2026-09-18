// What the fixtures need that `data.ts` does not say: who is a member, in
// which order the Shapers arrived, which room each prototype thread is,
// which objective line a root serves, the `39105` profiles (AI evaluation
// § Test data tunes them so `requires` is met by one, two, or nobody), and
// the cold-start org. Everything here is an explicit fixture decision;
// everything else comes from `prototypes/org-preview/src/lib/data.ts`.
//
// Dropped from `data.ts`, per Prototype map §3 and §4 — out of the Phase 0
// protocol, kept here so the story stays traceable:
//   - `seedProposals[]` kind=money: reimburse-sam, lea-saturdays, sam-stipend,
//     pay-priya, van; e-eecf, e-marcus-travel, e-pedro-stipend,
//     e-rowan-oncall, e-pay-rowan, e-coopernico, e-conference.
//   - `seedProposals[]` kind=join: join-rafi, join-priya (membership is an
//     invite link, §6.6 — Rafi and Priya are simply members).
//   - `agreedPay`, `treasury`, `energyOrg.treasury`, `Proposal.amount`,
//     `Proposal.to`, `USD`/`EUR`; the chat lines that agree a sum stay as
//     plain `kind:9` talk.
//   - `energyOrg.health` (org-level; a `50101` is per project),
//     `energyOrg.timeline`, `jobs`, `founding`, `uiMapping`, `trail[]`,
//     `DirectionLine.read`, `Proof[]`, `HOLDERS` beyond the profiles below.
//   - `Msg.card` lines whose draft has no object to attach to: the `dri`
//     drafts for Weekday hall (`sh-d3`) and Carbon credits (`esh-d4`) name
//     projects that are still open proposals, not `39101` items, so they
//     stay plain text; the Personal Assistant help card is plain text.

import { DAY, NOW } from "./dates.mjs";

function at(y, m, d) {
  return Date.UTC(y, m - 1, d, 9, 0, 0) / 1000;
}

export const RIVER = {
  id: "river",
  founder: "Maya",
  // Everyone `data.ts` names in River: `space.members`, plus Rafi (HOLDERS,
  // a member by invite), You (the reader), and Eli (the investor persona).
  members: ["Maya", "Sam", "Lea", "Jun", "Noor", "Tom", "Priya", "Rafi", "You", "Eli"],
  // Maya founds alone; Sam's seat comes after mission and vision v1 — the
  // two proposals `data.ts` records with `needed: 1`.
  shaperAdds: [{ name: "Sam", after: "vision@1" }],
  foundedAt: at(2026, 3, 1),
  objectiveOf: { stall: 0, weekday: 1, growers: 2, harvest: 2 },
  // Target dates carried inside the objective sentences.
  objectiveDates: [null, at(2026, 7, 31), at(2026, 11, 30)],
  proposalOf: { stall: "fund-stall", growers: "approve-growers", currency: "approve-currency" },
  // Threads → rooms. A project's home room is the relay's (§6.7): the
  // prototype's "Saturday stall" thread is the stall's home. "Growers" is a
  // community room — the Growers project was drafted *from* it, so it
  // predates the project and cannot be its home.
  rooms: {
    shapers: { shapers: true },
    saturday: { home: "stall", anchor: NOW - 2 * DAY },
    growers: { community: { name: "growers", topic: "Growers" }, members: ["Jun", "Maya"], anchor: NOW - 6 * DAY },
    "sam-dm": { dm: ["Sam", "You"], anchor: NOW - DAY },
    agent: { dm: ["You", "agent"], anchor: NOW - 2 * DAY },
  },
  // Shapers-room talk behind each confirmed version, and the card line
  // that is the agent's `50100`.
  directionTalk: {
    "mission@1": { room: "shapers", said: ["sh-m1", "sh-m2", "sh-m3"], card: "sh-m4" },
    "vision@1": { room: "shapers", said: ["sh-v1", "sh-v2", "sh-v3"], card: "sh-v4" },
    "objectives@3": { room: "shapers", said: ["sh-o1", "sh-o2", "sh-o3"], card: "sh-o4" },
  },
  // The one direction draft still open: strategy v5 from Tuesday's call.
  openDirectionDraft: {
    room: "shapers",
    ingest: "sh1",
    said: ["sh-s1", "sh-s2", "sh-s3"],
    card: "sh-s4",
    slug: "strategy",
    anchor: NOW - 5 * DAY,
  },
  // Plain talk after the drafts, in order, with a date anchor.
  laterTalk: [{ room: "shapers", ids: ["sh-d1", "sh-d2", "sh-d3"], anchor: NOW - 3600 }],
  // The message a talk-drafted project cites; else the room's `39000`.
  projectReceipt: {
    weekday: ["saturday", "s3"],
    growers: ["growers"],
    harvest: ["growers"],
    currency: ["shapers"],
  },
  // `from` says which room the draft was heard in.
  projectDraftRoom: { weekday: "saturday", growers: "growers", harvest: "growers", currency: "shapers" },
  // The live tickets from `ticketsData`, folded into their project's rows.
  liveTickets: {
    stall: ["covers", "setup", "prices"],
    islands: [],
    iberia: [],
  },
  // `TICKET_SUGGESTED`: the agent's first suggested holder per ticket.
  suggested: { covers: "Lea", setup: "You", weekday: "Lea" },
  // The stall enters review on the §5.1 date rule; the message the agent
  // relayed as a done is none in River.
  doneFromTalk: {},
  // §4.7a profiles. Rafi and Priya are skills-only newcomers, Lea is
  // history-only (no profile), Noor/Eli/You have neither. Requirements:
  // `council-licensing` → Rafi alone; `hosting-events` → Sam (at
  // `open_limit`, he holds the stall) and Tom; `electrical-certification`
  // → nobody in River (Tomas has it in Energy).
  profiles: [
    {
      name: "Rafi",
      about: "I ran the market office for six years. I can deal with the council, and I know which licence a hall needs before the deposit goes down.",
      skills: ["council licensing", "market management", "permits and insurance"],
      openLimit: 2,
    },
    {
      name: "Priya",
      about: "I design — print, signage, a one-page rule sheet. Spanish at home, so I can do the growers' Spanish flyer too.",
      skills: ["graphic design", "print and signage", "spanish"],
      openLimit: 3,
    },
    {
      name: "Sam",
      about: "I hold the Saturday stall. I can host, run a cash box, and drive the crates in.",
      skills: ["hosting events", "cash handling", "driving"],
      openLimit: 1,
    },
    {
      name: "Tom",
      about: "Handy — tables, fridges, laminating. Happy to host an afternoon when someone else does the money.",
      skills: ["hosting events", "practical repairs", "laminating"],
      openLimit: 4,
    },
    {
      name: "Jun",
      about: "I know the three growers and the orchard prices. I can print anything the stall needs at work.",
      skills: ["grower relations", "pricing", "printing"],
      openLimit: null,
    },
    {
      name: "Maya",
      about: "Founder. I write the direction down and keep the Shapers honest about it.",
      skills: ["writing", "facilitation"],
      openLimit: null,
    },
  ],
  whoIsNeeded: {
    "council-licensing": "one",
    "hosting-events": "two",
    "electrical-certification": "nobody",
  },
  locale: "pt",
};

export const ENERGY = {
  id: "energy",
  founder: "Alex",
  members: [
    "Alex", "Edgar", "Zekeriya", "Pedro", "Rogerio", "Suzana", "Rowan", "Marcus", "Inês", "Surya",
    "Kai", "Tomas", "Jelle", "Diego", "Marta", "Nina", "You",
  ],
  // "Written at founding, with Edgar and Zekeriya": both seats before vision v1.
  shaperAdds: [
    { name: "Edgar", after: "bootstrap" },
    { name: "Zekeriya", after: "bootstrap" },
  ],
  foundedAt: at(2022, 1, 1),
  objectiveOf: { iberia: 0, ems: 1, playbook: 2, islands: 3 },
  objectiveDates: [at(2026, 5, 31), at(2026, 9, 30), at(2026, 12, 31), at(2026, 12, 31)],
  proposalOf: { iberia: "e-approve-iberia", playbook: "e-approve-playbook", islands: "e-ameland" },
  rooms: {
    "e-shapers": { shapers: true },
    "e-pilots": {
      community: { name: "pilots", topic: "Pilots" },
      members: ["Alex", "Pedro", "Rogerio", "Marcus", "Suzana", "Inês", "Diego", "Marta", "You"],
      anchor: NOW - 3600 - 20 * 60,
    },
    "e-tech": {
      community: { name: "tech-team", topic: "Tech team" },
      members: ["Surya", "Rowan", "Kai", "Tomas", "Jelle", "Edgar"],
      anchor: NOW - 5 * DAY,
    },
    "e-marcus-dm": { dm: ["Marcus", "You"], anchor: NOW - DAY },
    "e-agent": { dm: ["You", "agent"], anchor: NOW - 2 * DAY },
  },
  directionTalk: {
    "mission@2": { room: "e-shapers", said: ["esh-m1", "esh-m2", "esh-m3"], card: "esh-m4" },
    "vision@1": { room: "e-shapers", said: ["esh-v1", "esh-v2", "esh-v3"], card: "esh-v4" },
    "objectives@6": { room: "e-shapers", said: ["esh-o1", "esh-o2", "esh-o3"], card: "esh-o4" },
  },
  openDirectionDraft: {
    room: "e-shapers",
    ingest: "esh1",
    said: ["esh-s1", "esh-s2", "esh-s3"],
    card: "esh-s4",
    slug: "strategy",
    anchor: NOW - 5 * DAY,
  },
  laterTalk: [{ room: "e-shapers", ids: ["esh-d1", "esh-d2", "esh-d3", "esh-d4"], anchor: NOW - 3600 }],
  projectReceipt: {
    iberia: ["e-pilots"],
    islands: ["e-pilots"],
    playbook: ["e-pilots"],
    carbon: ["e-tech"],
    hardware: ["e-tech"],
  },
  projectDraftRoom: { iberia: "e-pilots", islands: "e-pilots", playbook: "e-pilots", carbon: "e-tech", hardware: "e-tech" },
  liveTickets: { islands: ["e-summary"], iberia: ["e-muni"] },
  suggested: { "e-summary": "You", "e-muni": "Rogerio", carbon: "Rowan" },
  // Marcus's line in Pilots closed the Ameland ticket (§5.5): the agent's
  // `io_done` cites the message.
  doneFromTalk: { "islands/ameland-tokenised-credits-pilot": ["e-pilots", "ep1"] },
  // Requirements: `electrical-certification` → Tomas alone;
  // `on-call-operations` → Rowan (at `open_limit`: he holds the Ameland
  // on-call) and Surya; `dutch-cooperative-law` → nobody (the Netherlands
  // template is open for exactly that reason). Rowan is the only member at
  // `open_limit` in the seed (AI evaluation § Test data): Diego's and
  // Pedro's limits sit one above what they hold, so the sequence fixtures
  // can hand each of them one more item.
  profiles: [
    {
      name: "Tomas",
      about: "Grid protocols and the boring correctness work — DST, timezones, certification paperwork. I hold an electrical certification from the Prague years.",
      skills: ["electrical certification", "grid protocols", "timezone handling"],
      openLimit: 3,
    },
    {
      name: "Rowan",
      about: "EMS on-call and the battery algorithms. I also keep the carbon accounting spreadsheet nobody else wants.",
      skills: ["on-call operations", "battery optimisation", "carbon accounting", "video"],
      openLimit: 1,
    },
    {
      name: "Surya",
      about: "I run the EMS: markets, batteries, and the on-call rota when Rowan is out.",
      skills: ["on-call operations", "energy management systems", "market connectors"],
      openLimit: 3,
    },
    {
      name: "Inês",
      about: "Portuguese energy law. I draft the CER statutes and read the Coopérnico contracts.",
      skills: ["portuguese energy law", "legal templates"],
      openLimit: 2,
    },
    {
      name: "Diego",
      about: "Spanish energy law — comunidades energéticas, the regional variants, the Junta.",
      skills: ["spanish energy law", "legal templates"],
      openLimit: 3,
    },
    {
      name: "Marta",
      about: "Translation and terminology. Spanish grid vocabulary is my corner.",
      skills: ["spanish", "translation", "grid terminology"],
      openLimit: null,
    },
    {
      name: "Kai",
      about: "Data pipelines and parsers. Python. Spanish from a year in Seville.",
      skills: ["python", "data pipelines", "spanish"],
      openLimit: null,
    },
    {
      name: "Pedro",
      about: "I hold Iberia pilots. Municipalities, mayors, and the EECF paperwork.",
      skills: ["municipal relations", "grant writing"],
      openLimit: 4,
    },
    {
      name: "Rogerio",
      about: "Portuguese municipalities and the Coopérnico relationship.",
      skills: ["portuguese municipalities", "partner liaison"],
      openLimit: null,
    },
    {
      name: "Suzana",
      about: "Onboarding and grants. I wrote the round-1 EECF application.",
      skills: ["grant writing", "community onboarding"],
      openLimit: null,
    },
    {
      name: "Marcus",
      about: "Island grids and the Ameland sandbox. Tokenised credits, end to end.",
      skills: ["island grids", "tokenised credits"],
      openLimit: 2,
    },
    {
      name: "Alex",
      about: "Founder. Direction, projects, money.",
      skills: ["direction", "fundraising"],
      openLimit: null,
    },
  ],
  whoIsNeeded: {
    "electrical-certification": "one",
    "on-call-operations": "two",
    "dutch-cooperative-law": "nobody",
  },
  locale: "es",
};

// Not in `data.ts`: one founder, four version-1 artifacts, one `39103`,
// an empty tree, no profiles — the first-week experience.
export const COLD = {
  id: "cold",
  founder: "Ada",
  members: ["Ada"],
  foundedAt: NOW - DAY,
  direction: {
    mission: {
      body: "A repair café for the estate: fix what people already own, together, on the first Saturday of the month.",
    },
    vision: {
      body: "In a year, a room that opens every month, three regulars who can teach a repair, and nothing on the estate thrown away that could have been mended.",
    },
    objectives: {
      body: "What we mean to have done soon. One sentence each, with the timing inside it.",
      lines: [
        { text: "The first repair café opens before July.", date: at(2026, 6, 30) },
        { text: "Three volunteers can each run a bench by September.", date: at(2026, 9, 30) },
      ],
    },
    strategy: {
      body: "How we get there — the bets, and what we say no to.",
      lines: [
        { text: "Borrow before we buy: tools and the room come from people who have them.", date: null },
        { text: "No charging. A donation tin, nothing else.", date: null },
      ],
    },
  },
};

export const ORGS = [RIVER, ENERGY];
