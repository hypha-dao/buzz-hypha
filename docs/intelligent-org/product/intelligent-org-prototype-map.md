---
title: The Intelligent Organization — Prototype map
status: current
date: 2026-09-15
source: prototypes/org-preview @ 2026-09-15 (14.3k lines TypeScript, two sample orgs)
---

# The Intelligent Organization — Prototype map

[`prototypes/org-preview`](../../../prototypes/org-preview/README.md) is a
static Next.js walk-through of the designed product: five doors, chats, a
Personal Assistant, and two scripted orgs (**River Commons**, **Hypha
Energy**). It is the only visual specification the project has, and the
[AI evaluation](../plans/intelligent-org-ai-evaluation.md) names its
`src/lib/data.ts` as the first source for the test fixtures.

This document is the bridge from the prototype to the code:

1. **What is in it** — screens, components, data model, scripted flows.
2. **What the desktop `org` feature grows from** — screen by screen, with
   the Phase 0 decision (build now / later / never) and what changes when
   the screen becomes data-driven over relay subscriptions.
3. **What the fixtures extract** — the mapping from `data.ts` types to
   Protocol events, with the rules for the parts that do not carry over
   (money, "assigned" work, scripted futures).
4. **What the prototype gets wrong** against the current documents, so
   nobody copies it.

It does not restate the product. For what a screen _means_, read
[Features](./intelligent-org-features.md) and
[Journeys](./intelligent-org-journeys.md); for what a screen _reads_, the
[Protocol § 6.5 door table](../architecture/intelligent-org-protocol.md).

---

## 1. Inventory

### Screens (`src/screens/`)

| File                 | Lines | What it shows                                                                                                                                                                             |
| -------------------- | ----: | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `org.tsx`            |   818 | **Overview** — four direction cards (mission, vision, objectives, strategy) with version and confirmer; _Highlights_ timeline; _At a glance_ stats; Shapers list; _Who holds a project_; Treasury; _Org health — the agent's read_ |
| `direction.tsx`      |   353 | **Direction page** — one artifact's body, its lines with the agent's `read` and `proofs`, and _Versions — every one confirmed by a Shaper_                                              |
| `work.tsx`           | 1 754 | **Projects** (`AllWork`, _Who is working on what_), **Project page** (`ProjectDetail`), **My Work** (`MyWork`: _What needs me, what I hold, what I offered_), and the **Shaper ask** screen (`ShaperAskScreen`, _Needs your answer_) |
| `work-bits.tsx`      |   769 | The shared work components — `WorkBoard`, `MyWorkBoard` (columns _Needs your answer / You hold / You offered / Finished_), `WorkItemCard`, `HolderFact`, `StateChip`, `HealthCard`, `HealthPill`, `ChildList` (_Under this ticket_), `OpenProjectCard` (_needs a DRI_), `OfferCard`, `HeldCard`, `Waiting` |
| `ticket-view.tsx`    |   190 | **Ticket page** read-only — _Holds it_, _Offered by_, _Due_, breadcrumb via `parent`                                                                                                     |
| `project-static.tsx` |   120 | Project page for stories that do not move; `ProjectHealth` card                                                                                                                          |
| `proposals.tsx`      |   373 | **Decisions** — list with _Voting until_; `ProposalDetail` with _Shaper approval_, _Recipient_, _Opened by_, _Ends_, _Decides_                                                            |
| `profile.tsx`        | 1 024 | **My Profile / a member** — _What you hold_, _Recent decisions_, _Earlier work_, and pay history                                                                                          |
| `chats.tsx`          | 1 759 | DMs, group rooms, `#shapers`, the **Personal Assistant** thread; agent cards with _Open as a proposal_, _Confirm — done_, _Not done yet_, _Agree_, _Decline_, _Offer to …_, amend-in-place toasts; receipts under replies |
| `energy.tsx`         |   848 | Hypha Energy variants of My Work, Projects, project and ticket pages; `CarbonDetail`, `JoinDetail`, _Draft — already written, yours to correct_                                          |
| `onboarding.tsx`     |   295 | First-login flow — Purpose / People / 90 days / Money cards                                                                                                                              |
| `public-space.tsx`   |   154 | The door from outside — _What this space has actually done_, _Open work nobody holds_, request to join                                                                                   |
| "Confirms itself in 48h if nobody objects" on `done` drafts (`store.tsx`, `chats.tsx`); the agent posts "Marked … done" (`data.ts` 2218, `store.tsx` 872) | Done is always the holder's tap (Features § two rules; Design § Work sync). No draft auto-settles; a `done` draft expires unsettled. The agent never marks anything — fixtures rewrite these lines as the holder's `io_done` with the draft as receipt |
| The door is labelled **Projects** (`workspace.tsx` 70, `energy.tsx`) | The door is **Work** (`/org/work`); "project" is the root item kind, not the surface |
| A persona switcher in the shell (`workspace.tsx` "persona switcher") | A demo device only. Desktop has one signed-in member; the E2E fixtures cover other viewpoints |
| `about.tsx`          |   134 | The prototype's own notes — two rules, money, old Hypha → new mapping, the five-minute demo path                                                                                          |

### Shell and primitives (`src/components/`)

`workspace.tsx` — one nav for everyone (Overview, Projects, Decisions, My
Work, My Profile), a chat list (Personal Assistant first, then rooms and
DMs), badges for open proposals and _needs me_, a mobile bottom bar.
`primitives.tsx` — `Button`, `Card`, `Kicker`, `Chip`, `Avatar`,
`AgentMark`, `EmptyState`, `Hairbar`, `Row`. `chat.tsx` — thread and
composer. `person.tsx` — `PersonLink`. `direction-mark.tsx` — the four
artifact glyphs.

### Data (`src/lib/data.ts`, 2 640 lines)

| Type / const                                      | What it is                                                                                                                         |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `space`, `energyOrg.space`                        | name, founder, `shapers[]`, `members[]`, purpose lines                                                                             |
| `Direction` (`mission`, `vision`, `objectives`, `strategy`) | each an `Artifact` — `version`, `confirmedBy`, `confirmedOn`, `text`, `body[]`, `history[]`; objectives and strategy have `items[]`/`lines[]` of `DirectionLine { text, read?, proofs[] }` |
| `Proof`                                           | `{ when, what, go?: 'thread'\|'proposal'\|'project', id? }` — an L2 fact behind an L3 line                                            |
| `StaticProject`                                   | `{ id, title, dri, review, approved, brief, from, tickets[], trail[], health? }`                                                    |
| `WorkTicketRow`                                   | `{ title, who, state: done\|doing\|waiting\|open, due?, children?, offeredBy?, ticketKey? }` — the tree, any depth                    |
| `TrailRow`                                        | `{ when, what, receipt? }`                                                                                                          |
| `Health`                                          | `{ pct 0–100, label, text }`                                                                                                        |
| `Proposal`                                        | `{ id, kind: money\|project\|dri\|direction\|join, artifact?, title, sub, amount?, state, yes, no, needed, openedBy?, agreedBy?, to?, description?, ends? }` |
| `Msg`, `MsgCardType`, `Receipt`                   | chat lines; eleven card types; `Receipt { label, go, id }`                                                                          |
| `personas`, `HOLDERS`, `TICKET_SUGGESTED`         | who you can be; who holds what; the agent's first suggestion per ticket                                                             |
| `agreedPay`, `treasury`, `USD`/`EUR`              | pay agreed in chat; balances                                                                                                        |
| `threads`, `seedMessages`, `jobs`, `founding`     | the rooms and their scripted lines                                                                                                  |
| `uiMapping`                                       | old Hypha surface → new door, with the reason (rendered on About)                                                                   |

Counts: River has 5 projects (`stall`, `weekday`, `growers`, `currency`,
`harvest`), 3 live tickets with children (`covers`, `setup`, `prices`), 14
seed proposals, 5 threads, 2 Shapers, 7 members. Energy has 6 projects
(`iberia`, `ems`, `islands`, `carbon`, `playbook`, `hardware`), a tree up to
six levels, 3 Shapers. Across both: 116 titled rows; ticket states 23 done,
21 doing, 11 open, 3 waiting.

### State and flows (`src/lib/store.tsx`, `assist-flows.ts`, `entries.ts`)

`store.tsx` (1 553 lines) is the in-browser world: `Route` (five doors plus
`thread`, `project`, `ticket`, `ticket-view`, `offer`, `proposal`,
`direction`, `shaper`, `about`, and the pre-space `onboarding`, `public`,
`request`), the persona, and a state machine per story (`SetupState`,
`CoversState`, `WeekdayState`, `ReviewState`, `CarbonState`, `OfferState`,
…). `assist-flows.ts` scripts the Personal Assistant: seven help entries
(direction, project, money, done, ticket, dri, ask), `openWorkWithoutDri`,
`driPeople`. `entries.ts` says where `/` and `/onboarding` drop you.

---

## 2. Screen → desktop

The desktop feature is `desktop/src/features/org/` behind the `org`
feature id (Development plan D-0). The prototype's components are the
starting material; they are **moved and rewritten**, not imported: the
prototype is Next.js with its own Tailwind theme and px text sizes, the
desktop is Tauri + React 19 with the rem-only text contract (AGENTS.md
§ Text sizing) and Radix primitives. What carries over is layout, copy,
kickers, empty states, and the card grammar.

| Prototype screen                              | Phase 0        | Desktop route / place                        | Reads (Protocol § 6.5)                                                   | What changes                                                                                                                                                                                                                     |
| --------------------------------------------- | -------------- | -------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Overview — direction cards                    | **build** D-1  | `/org`                                       | `{kinds:[39100]}`                                                        | Cards render from `39100` content; `confirmedBy` is the proposal's last agreeing Shaper; the write-in animation goes; empty slot copy _Not set yet_ with the form for a Shaper                                                     |
| Overview — Shapers list                       | **build** D-1  | `/org`                                       | `{kinds:[39103]}`                                                        | Adds rules and the agent host line; **Add a Shaper**, **Step down**, **Change the rules**                                                                                                                                          |
| Overview — Who holds a project                | **build** D-1  | `/org`                                       | `{kinds:[39101], "#t":["project"]}`                                      | `DriRow` / `OpenDriRow` as designed; _open_ links to the item page, not to a scripted offer                                                                                                                                       |
| Overview — Highlights timeline                | later          | —                                            | ledger `50103` + `39104`                                                 | Needs the ledger read; Phase 0 § Overview says "no timeline"                                                                                                                                                                       |
| Overview — At a glance stats                  | later          | —                                            | counts over `39101`/`39102`                                              | Phase 0 says "no glance numbers"                                                                                                                                                                                                  |
| Overview — Treasury                           | **never** in this form | —                                    | —                                                                        | Money is out of the Phase 0 protocol (Readiness F-series); when it returns it is a proposal kind, not a balance card                                                                                                              |
| Overview — Org health                         | later          | `/org` bottom                                | `{kinds:[50101], "#i":[root]}` aggregated                                | Project health is Phase 0 (item page); org-level roll-up is not                                                                                                                                                                  |
| Direction page                                | **build** D-1 (read) | `/org/direction/$slug`                 | `{kinds:[39100], "#d":[slug]}`, `{kinds:[39102], "#t":["direction"], "#s":["passed"]}` | Lines with stable ids as Protocol § 4.1; `read` and `proofs` per line come with move 3/4 (later); versions list from passed proposals                                                                                       |
| Projects (`AllWork`)                          | **build** D-3  | `/org/work`                                  | `{kinds:[39101]}`                                                        | Tree assembled client-side from `root`/`parent`; roots + one level; `WorkBoard` columns collapse to one list with state chips                                                                                                    |
| Project page (`ProjectDetail`, `StaticProjectDetail`) | **build** D-3 | `/org/work/$itemId`                   | `#d`, `#u` (children), `{kinds:[50001–50021], "#i":[id]}` (trail), `50101` (health), `50102` (work log) | `TrailRow` is the commands newest first; `HealthCard` from the latest `50101` with rows on hover; **Open room** from `home.channel`; **Mark done / Release / Set due** buttons; _Review_ becomes `due_at`; _Approved_ is `approved_at` |
| Ticket page (`TicketViewScreen`)              | **build** D-3  | same route                                   | same                                                                     | One page for project and ticket — the `t` tag decides the header; `parent` breadcrumb from `39101.parent`                                                                                                                       |
| My Work (`MyWork`, `MyWorkBoard`)             | **build** D-2  | `/org/my-work`                               | `39101 #p me`, `50100 #n me` (or `shaper`), `39102 #p me #s open`        | Three columns (Phase 0 drops _Finished_); `ShaperAskCard` becomes the draft card with kickers _AI is asking you_ / _AI is suggesting for (name)_ / _Drafted by the agent_; three taps Agree / Edit then agree / Decline with reason chips |
| Shaper ask screen                             | **build** D-2  | card detail (sheet), not a route             | one `50100` + its receipts                                               | Becomes the expanded draft card; the fixed decline-reason list replaces free text                                                                                                                                                 |
| Decisions (`Proposals`, `ProposalDetail`)     | later (wave 4) | `/org/decisions`                             | `{kinds:[39102]}` with `#t`/`#s`                                         | Phase 0 puts votes on My Work cards; the door comes when there are enough proposals to list. `Recipient` and `amount` go with money; `join` goes with membership proposals (Protocol § 6.6)                                        |
| My Profile (`ProfileScreen`)                  | **build** D-4 (section only) | existing profile view              | `{kinds:[39105], "#d":[pubkey]}`, `39101 #p`, `39102 #p`                 | Only the About & skills section is new; _What you hold_ and _Recent decisions_ come from `39101`/`39102` and are cheap once D-3 exists; pay history never                                                                          |
| Chats — rooms, DMs, `#shapers`                | **exists**     | Buzz channels and DMs                        | —                                                                        | Buzz's own. The agent's cards land in Buzz messages as `50100` renderings (D-2's card set reused in the timeline), not as a separate chat surface                                                                                 |
| Personal Assistant thread                     | **exists** as a DM | the member's DM with the org agent       | —                                                                        | Protocol § 6.8: the agent is in every DM; a member's 1:1 with it is the Personal Assistant. The seven scripted help entries become the agent's THINK menu (Org agent § 8), not UI                                                 |
| Onboarding, Public space, Request to join     | later          | —                                            | `39100`, `39101` open roots                                              | Membership proposals are out of Phase 0; the public door needs an unauthenticated read the relay does not offer                                                                                                                  |
| About                                         | **never**      | —                                            | —                                                                        | Prototype-only                                                                                                                                                                                                                    |

### Cards — the grammar to keep

The prototype settled the card grammar the desktop must reproduce
(Phase 0 § Cards, by move). From `work-bits.tsx` and `chats.tsx`:

- A card is _kicker + one-sentence claim + facts row + receipts + taps_.
- Kickers name who is speaking to whom: **AI is asking you**, **AI is
  suggesting for (name)**, **Drafted by the agent**, **Needs your answer**.
- Facts are label/value pairs (`Fact`, `HolderFact`): _Holds it_, _Offered
  by_, _Due_, _Suggested_, _Review_, _Needs_, _n of needed_.
- State chips (`StateChip`): **done**, **in progress** (always with a
  holder), **waiting on a yes** (with who), **open** / **needs a DRI**.
- Receipts are a row of chips that open the cited place; _Receipts below_ is
  the agent's phrase.
- Taps are verbs, at most three: **Agree**, **Edit then agree**, **Decline**;
  **Accept**, **Not now**; **Mark done**, **Not yet**; **Open as a
  proposal**; **Offer to …**. The prototype's amend-in-place toasts (_Amended
  in place — wording. Still a draft._) become the **Edit then agree** path
  that emits the command with the draft's `e` tag and a changed payload
  (Protocol § 3.2 settlement, `amended`).
- Empty states are one sentence: _Nothing needs you._ / _Nothing here._ /
  _Not set yet._

Health (`HealthCard`, `HealthPill`, `healthColor`): a position on a
red→yellow→green bar, a label, and a paragraph — Protocol § 4.7 `50101`
maps `band` to the position and `text` to the paragraph; the rows behind
each sentence are new.

### Accessibility and input, before any card ships

The prototype has no keyboard path, no focus management, and decorative
`div` buttons in places. AGENTS.md rules 7 and 8 apply to every card: one
owner per actionable label, a keyboard path for every tap, no duplicate
screen-reader stops on the receipt chips. D-2's Playwright spec asserts the
keyboard path.

---

## 3. Fixtures from `data.ts`

Development plan E-1 generates `tests/eval/fixtures/orgs/{river,energy,cold}/seed.json`
from `data.ts` with a checked-in script. The seed is a list of Protocol
events in the order the relay would have stored them, applied through the
agent's `OrgState::apply` (Org agent § 5). This table is the spec.

| `data.ts`                                   | Events                                                                                                                                                                   | Rule                                                                                                                                                                                                                              |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `space.shapers`, `space.founder`            | `39103` with `shapers`, `rules` = defaults, `agent` = a fixture pubkey, `agent_hosted = true`                                                                             | founder is `owner`; a private `#shapers` channel id is part of the seed                                                                                                                                                           |
| `space.members`                             | NIP-43 membership rows (fixture pubkeys, deterministic from the name)                                                                                                    | one pubkey per name across both orgs; `personas.you` maps to the harness's reader                                                                                                                                                 |
| `direction.*`                               | one `39100` per artifact at its `version`; each `history[]` entry becomes a passed `39102 t=direction` and the `50002` that opened it                                     | `body[]` → `text` lines; objectives `items[]` and strategy `lines[]` become numbered lines with stable ids `l_<hash>`; `read` and `proofs` are **not** seeded — they are agent output the harness must produce                        |
| `projectsData.*`, `energyOrg.projects.*`    | `39101 t=project` per project — `state` from `dri`/`approved`: `approved && dri` → `in_progress`, `approved && !dri` → `open`, `!approved` → an open `39102 t=project` and no `39101`; `due_at` from `review`; `approved_at` from `approved` | `from` becomes the draft's `origin` on a `50100` + `39104 accepted` when it says the agent drafted it, otherwise nothing; `brief` is `brief`                                                                                       |
| `tickets[]` / `children[]` (any depth)      | `39101 t=ticket` with `parent`, `root`; state map: `done` → `done` with `done_at`; `doing` → `in_progress` (holder = `who`); `waiting` → `offered` (`offered_to = who`); `open` → `open` | **"Assigned" is re-expressed as offered-then-accepted**: a `doing` row seeds `50006 io_offer` + `50007 io_accept` so the ledger shows the holder said yes; `offeredBy` = the agent seeds a `50100 kind=ticket` + `39104 accepted`  |
| `trail[]`                                   | ignored                                                                                                                                                                  | the trail is derived from the seeded commands; a `trail` row with no command behind it is a story, not a fact                                                                                                                       |
| `health`                                    | one `50101` per project that has it, `band` from `pct` (0–33 struggling, 34–66 watch, 67–100 healthy), `text` from `text`, `rows` empty                                    | used as the **gold** for move 4's judge, not as state                                                                                                                                                                             |
| `seedProposals[]` `kind=project`/`dri`/`direction` | `39102` with `votes` from `agreedBy`/`rejectedBy` (or the first `yes` Shapers in order), `needed`, `state`; `t` from `kind`                                     | `direction` proposals also produce the `39100` version they confirmed                                                                                                                                                             |
| `seedProposals[]` `kind=money`/`join`, `agreedPay`, `treasury`, `amount`, `to` (money) | **dropped**                                                                                                                                                   | Out of the Phase 0 protocol. A comment in the generator lists what was dropped so the story stays traceable                                                                                                                       |
| `threads`, `seedMessages`                   | `kind:9` messages in the matching rooms with fixture timestamps; `Msg.card` lines become the `50100` they represent where one exists, else plain text                    | This is the L1 the HEAR cases read; the Personal Assistant thread is the reader's DM with the agent                                                                                                                              |
| `HOLDERS`, `TICKET_SUGGESTED`               | `39105` profiles: `skills` inferred from held work titles for the named few (Rafi, Priya skills-only; Lea history-only per AI evaluation § Test data)                    | `open_limit` default; Rowan at `open_limit` in Energy                                                                                                                                                                              |
| `jobs`, `founding`, `uiMapping`, `about`    | ignored                                                                                                                                                                  | prototype narrative                                                                                                                                                                                                                 |
| `store.tsx` state machines                  | ignored                                                                                                                                                                  | they are the demo's futures; fixtures seed the present. The AI evaluation's cases describe the futures as expected agent output                                                                                                    |

**Cold start** is not in `data.ts`: one founder, four `39100` at version 1,
one `39103`, empty tree. The generator writes it from constants.

**Locales.** `pt` for River and `es` for Energy translate `text`, `brief`,
`title`, and messages; ids, pubkeys, timestamps, and tags are shared with
`en` so a case can run in both and compare.

**Determinism.** Pubkeys derive from `sha256("io-fixture:" + name)` as a
private key; event ids follow. Timestamps are `2026-03-01T09:00Z` plus the
row's order in `data.ts`, one minute apart, except where `when`/`decided`
gives a date. The generator is idempotent; its output is checked in, and a
test regenerates and diffs.

---

## 4. Where the prototype disagrees with the documents

Copying the prototype literally would reintroduce things the documents have
since removed or changed. The list, so the desktop and the fixtures follow
the documents:

| Prototype                                                                 | Documents                                                                                                                       |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Five doors including **Decisions** and **My Profile**                     | Phase 0 has three doors; Decisions is wave 4; profile is a section on the existing Buzz profile                                  |
| Money everywhere — `money` proposals, `amount`, `agreedPay`, Treasury, pay history on profiles | Money is out of the Phase 0 protocol; `io_money_propose` (`50013`) is rejected with a fixed reason. When it returns it follows Features 1a, not the prototype's chat-agreed pay |
| `join` proposals and the public space                                     | Membership is invite links minted by Shapers (Protocol § 6.6); join proposals are a later feature                                |
| The agent "assigns" work in copy (`who`) and `TICKET_SUGGESTED` reads as assignment | Work is offered, never assigned (Features § two rules); the fixtures re-express every held row as offer + accept              |
| `Proposal.yes/no` counts without voters in places                         | `39102.votes` is a list of `(pubkey, vote)`; the fixture derives voters from `agreedBy` or the Shaper order                        |
| `confirmedBy` is one name                                                 | A direction version is confirmed by a proposal meeting the rule; the card shows the rule met and the last agreeing Shaper          |
| Direction `read` and `proofs` are hand-written                            | They are move 3/4 agent output with receipts; never seeded, always produced                                                        |
| `Health.pct` is a number                                                  | `50101` carries a `band` and rows; the bar position is derived                                                                    |
| Personal Assistant is a separate chat surface                             | It is the member's DM with the org agent, which Buzz already renders                                                              |
| Proposals show _Voting until_ per proposal                                | `expires_at` exists on `39102`; the copy stays                                                                                    |
| Overview shows Treasury, glance stats, timeline                           | Phase 0: none of the three                                                                                                        |
| `about.tsx` "old Hypha → this prototype" mapping                          | Historical; the Exploration document covers the decision                                                                          |

---

## Related

- [Prototype README](../../../prototypes/org-preview/README.md) — how to run it
- [Phase 0 § The app](../plans/intelligent-org-phase-0.md) — the three doors this maps to
- [Protocol § 6.5](../architecture/intelligent-org-protocol.md) — the door filters
- [AI evaluation § Test data](../plans/intelligent-org-ai-evaluation.md) — the fixtures this feeds
- [Development plan](../plans/intelligent-org-development-plan.md) — D-0…D-4, E-1
