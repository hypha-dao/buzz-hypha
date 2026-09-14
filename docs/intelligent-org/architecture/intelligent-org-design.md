---
title: 'The Intelligent Organization — Design'
date: 2026-09-14
status: current
tags: [architecture, intelligent-org, ai, memory, buzz]
---

# The Intelligent Organization — Design

How to build the features in
[The Intelligent Organization — What it is](../product/intelligent-org-features.md) on Buzz.
That document is the what; this is the how. The exact kinds, tags, and state machines are in
the [Protocol](./intelligent-org-protocol.md); this document says why they are shaped that way
and where each piece lives in the repo.

One organization is one Buzz **community**: one relay host, one member list, one signed event
log. Everything below is scoped to a community the way every other Buzz feature is.

---

## Principles

Four rules the whole design hangs on:

1. **The model is a commodity. The memory is the asset.** An AI model has no memory; it knows
   only what we put in front of it. Intelligence is what we write down, who may write it, and
   what we load when. The model stays swappable.
2. **The AI proposes, never publishes.** Nothing the model writes becomes memory or work until
   a human confirm promotes it. Silent AI writes are how hallucination becomes institutional
   truth. Structurally impossible here: the agent has no command that changes state.
3. **Memory holds interpretation, never readings.** "We are over-exposed to one funding
   source" is memory. "We hold 43,000 USDC" is a reading — fetched live, never stored in a
   belief, or the org recites stale numbers forever.
4. **Every claim carries a receipt.** A suggestion, a done, an answer — each points at the
   events that justify it. No receipt, no trust. On Buzz a receipt is an event id, and the
   relay refuses a draft whose receipts do not resolve.

And one Buzz-specific corollary: **rules trigger, models explain.** Every proactive move
starts from a relay state change or a clock, never from the model deciding to speak.

---

## Why this memory design

The options, and why they lose:

| Design                      | Why it fails here                                                                                                                                                |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Everything in the prompt    | Months of chat is millions of tokens. Cost, noise, and gossip ranks equal to decisions.                                                                          |
| One big vector search (RAG) | Similarity has no concept of **authority**. It cannot tell what the org _believes_ from what someone once _said_. Embedded AI summaries come back later as fact. |
| Fine-tune the model         | Stale the next day, unauditable, undeletable, welded to one vendor.                                                                                              |
| Knowledge graph             | Schema maintenance eats the team. Orgs are too messy for it.                                                                                                     |
| **Layered memory** (chosen) | Different rules per layer: who writes, how big, whether it ever reaches the AI.                                                                                  |

The chosen design — four layers, from
[Organizational Intelligence — Memory Architecture](./organizational-intelligence.md):

|        | Layer     | Holds                                                                    | Written by                       | Reaches the AI                         |
| ------ | --------- | ------------------------------------------------------------------------ | -------------------------------- | -------------------------------------- |
| **L1** | Substrate | every message, transcript, file                                          | machines, automatically          | never directly — searched for evidence |
| **L2** | Ledger    | typed facts: _ticket done_, _project approved_                           | the relay, on state change       | as aggregates                          |
| **L3** | Beliefs   | mission, vision, objectives, strategy — four short texts                 | **humans confirm every version** | always, in full                        |
| **L4** | Outcomes  | suggestion → decision → what happened                                    | the relay and the agent          | selectively                            |

L3 is small enough to always sit in the prompt — no retrieval lottery for what the org
believes. L1 is searched only for receipts. That split is what RAG-only designs cannot do.

---

## The layers on Buzz

Where each layer physically lives. The short version: **L1 already exists**, which is the
single biggest difference from building this anywhere else.

**L1 — substrate.** The relay's event store _is_ L1. Every stream message (`kind:9`,
`40002`), forum post and comment (`45001`, `45003`), canvas (`40100`), file (`1063`), huddle
lifecycle event (`48100–48103`), and DM lives in Postgres with a stable id, a signed author, a
channel, and a timestamp, indexed by Postgres FTS (NIP-50 `search` over `POST /query`). Nothing
to ingest, no appservice to build, no consent decision to make beyond the one Buzz already
made: channel membership gates visibility, and the org agent sees what a member of its channels
sees.

Two L1 gaps remain, both small:

- **Transcripts.** The desktop huddle pipeline already runs speech-to-text and posts the text
  as `kind:9` into the huddle channel (`desktop/src-tauri/src/huddle/stt.rs`). Those messages
  must carry a `["transcript", "huddle"]` tag so the agent and the relay can tell spoken text
  (weak attribution — nudge only) from typed text (a signature). Until they do, the agent
  treats every message in a huddle channel as a transcript.
- **Documents.** Uploaded files are stored (Blossom) and described (`kind:1063`) but their
  text is not indexed. Extracting text for search is a later item; receipts can already point
  at the file event.

**L2 — activity ledger.** On Buzz the ledger is not a side table someone must remember to
write; it is the event log. Every state change is a stored, signed **command event**
(`50001–50018`), and every rule-driven change (an offer expiring, a root closing on its date)
writes a ledger row with `actor = relay` pointing at the relay-signed state event it produced.
The typed table `io_ledger` is a projection of those, built inside the same transaction, for
the queries reviews and health need. Completeness is enforced by construction: the command
executor's single `apply()` helper is the only path that touches an `io_*` table, and it will
not commit a projection change without its command or ledger row. (Protocol §6.1–6.2.)

**L3 — beliefs.** Four relay-signed addressable events, `kind:39100` with
`d ∈ {mission, vision, objectives, strategy}`, one head each, replaced on every confirmed
version. They replace the "org brief" of earlier drafts: Overview renders them, and every
agent call loads all four heads in full. There are no other L3 artifacts in this design;
assessments and insights, if they come, are agent reads (like health), not beliefs.

`objectives` is the one with structure: a short list (three to seven), each line an outcome
with a rough date and a **stable line id** carried across versions. The whole list is redrawn
when an objective is reached or dropped — still one artifact, one version, always small enough
to load. It is also the join between direction and work: a root item cites the line it serves
(`objective_ref = objectives@<version>#<line-id>`), and the review draft for a project asks
whether that line moved.

The write path is **direction talk → draft → Shaper command → vote → new head**, and it is a
role check in the relay, not a room check:

- The agent listens for direction talk in the private **`#shapers`** channel and in any
  Shaper's DM with it. A draft (`50100`, `kind=direction|objectives`) names which artifact it
  changes and carries the diff against the current head, `needs: shaper`.
- **One Shaper** (a new org — the founder): the draft is posted back in their DM and the
  founder's tap is `io_direction_propose`; with one eligible Shaper the `one_other` rule
  degrades to `one` and the proposal passes on the spot.
- **Several Shapers**: the draft is posted to `#shapers` wherever the talk happened. One
  Shaper's `io_direction_propose` opens the proposal; another Shaper's `io_vote` confirms it.
  The others are notified.
- Confirm is a relay transaction — proposal passes, `39100` head replaced, ledger row. The
  model never writes L3.

**Seeding.** The community's description (NIP-29 metadata / workspace profile) and the
founder's first DM with the agent become the _first drafts_ of `mission` and `vision`, offered
right after the agent joins; `objectives` and `strategy` are drafted when the founder first
talks about what to do next. Nothing is confirmed on their behalf — a new community has four
empty slots on Overview until its founder says yes.

**L4 — decision memory.** Three projections, all relay-written: `io_drafts` with outcome
columns (what the agent suggested; accept / amend / decline with the fixed reason; what it
became) — surfaced as `kind:39104`; `io_health_ratings` (Shapers' blind bands against the
agent's); and the ledger rows that show what happened after. The agent reads L4 selectively
when drafting similar suggestions and never writes it directly.

---

## Work objects

Work is **one recursive tree**, not two tables. A project is a ticket with no parent; a ticket
is a piece of work under something. Anyone who holds a piece of work can split it further and
offer the pieces — to any depth. This is what lets a DRI hand a sub-task to a third person
without going back up to the project DRI, and it is what keeps the authority rule simple.

Each item is one relay-signed addressable event, `kind:39101`, `d = item uuid`, projected into
`io_work_items` (`community_id`, `parent_id?`, `root_id`, `depth`, `path`, `title`, `brief`,
`state`, `dri?`, `offered_to?`, `offered_by?`, `due_at`, `approved_at?`, `objective_ref?`,
`created_from`, `done_receipt?`, `closed_by?`). No money column — see "Money".

States: `open → offered → accepted → (in_review) → done`, with `open` reachable again by
decline, expiry, or release. **In progress means a holder**: `accepted` and `in_review`
require `dri`. `depth` and `path` are denormalised so "everything under this project" is one
indexed read.

Rules enforced in the command executor, not in the UI (Protocol §5.1):

- **One promotion rule.** A root is created only by a passed `project` proposal (Shapers). A
  child is created only by the holder of its parent. The same rule at every depth.
- **Holding a piece means you can split it.** A DRI may create children under their own item
  and offer them. They cannot offer work under an item they do not hold.
- Only the **named person** accepts or declines an offer.
- **Offers expire.** An offer nobody answers is renotified once, then returned after a
  window (`39103.offer_window_secs`, default three days). Items stuck in `offered` forever
  are what erodes trust in My Work.
- **Completion cascades up, never down.** An item with an open child cannot be marked done;
  when the last child closes, the agent offers the holder a **done card** — a `50100`
  `kind=done`, `needs: <holder>` — and the holder's tap is the `io_done`. Marking a parent
  done does not close children. Releasing a parent returns its open children to the holder
  above.
- **Roots close on their date.** The relay's scheduler (`io_scheduler`) moves a root to `done`
  when `due_at` passes unless a Shaper moved the date. That is a date rule a Shaper set, not
  a judgment; the agent does not run it and cannot stop it.
- Every state change is one transaction: command (or rule) → projection → `39101` → ledger.

Boards read the same tree at different cuts: **Work** groups by root; **My Work** filters by
`dri = me` at any depth and shows the path above each item as a breadcrumb; an item page shows
its direct children and its trail (the commands that touched it). Depth is a display concern,
not a schema concern.

The agent routes drafts with the same rule: a heard need is drafted under the nearest open item
the speaker holds, or the item the talk was about; the draft's `needs` is that item's holder.
If nothing covers it, it becomes a root draft and goes to the Shapers.

### Shapers

The Shaper set is one relay-signed event, `kind:39103`, maintained by `io_shaper_set`. The
community owner (NIP-43 `owner` role) is the founder and the first Shaper; after that only
Shapers add Shapers. The private `#shapers` channel is created by the relay on the first
`io_shaper_set` and its membership is updated **in the same transaction** as the Shaper set —
there is no second writer, so there is no reconcile loop to own. The org agent is added to the
room at creation.

Decision thresholds live in `39103.rules` per proposal kind. Defaults: direction and project
approval need **one Shaper other than the proposer** (`one_other` — the founder alone
degrades to `one`); DRI naming, money, and join need a **majority**. A community changes them
with `io_shaper_set op=rules`.

### Money

For the MVP, **no sum lives on a work item** — no budget indication on projects, no pay on
tickets. Money exists in exactly two places: proposals (the decision) and profiles (the record
of what a person was paid). Money proposals are **outgoing only** — pay or reimburse. Incoming
money is not a proposal kind; "we do not take brand money" is an L3 strategy line, not a
treasury vote.

**Buzz has no treasury, and this design does not add one.** A passed `money` proposal is a
decision; a Shaper then pays outside Buzz and sends `io_money_settle` with the reference. The
proposal moves to `settled`, and the payee's profile shows the payment with the proposal and
the settlement as receipts. When a treasury integration exists (Hypha or another), it can
execute passed proposals and send the settlement itself; nothing in the tree or the proposals
changes.

How a person gets paid, MVP: the item is done; the payee (or the holder above, or a Shaper)
sends `io_money_propose` — usually by telling the agent in their DM, which drafts it and shows
a card whose tap is the command; the Shapers vote; a Shaper settles. Every payment is a Shaper
decision — no threshold below which a DRI approves pay for the people under them.

**Later:** "agreed in chat". Whoever holds the work and whoever holds the item above name a
sum where they already talk; the HEAR pass tags that message as a pay agreement so "…whatever
we agreed" works and a differing sum shows both. It needs HEAR and is scoped after it.

Shaper decisions are five kinds, all `kind:39102` proposals opened by commands and decided by
`io_vote`:

| Kind        | What it is                                        | Opened by                              | Decisions filter          |
| ----------- | ------------------------------------------------- | -------------------------------------- | ------------------------- |
| `money`     | Pay or reimburse — **out only**                   | payee, holder above, or a Shaper       | Money                     |
| `project`   | Approve a root project (may have no DRI yet)      | any member                             | Work                      |
| `dri`       | Name a DRI for work that has none                 | any member                             | Work (tag **project DRI**) |
| `direction` | Mission / vision / objectives / strategy version  | a Shaper                               | Direction                 |
| `join`      | A person asks to join — people only, no Recipient | a member on their behalf, or the relay from an inbound request | Join |

A passed `dri` proposal writes `dri` on the item. That is a Shaper naming vote — the one
place work lands on a person without their accept, and it takes a vote to do it. The
offer–accept path still applies when work is offered to a person on a card.

---

## The org agent

Not a chatbot. A pipeline with three passes, running as a **Buzz member with its own key**:

```
1. HEAR    events the agent is subscribed to: state (39100–39104), its own and
           others' drafts (50100), messages in its channels (9 / 40002), its DMs
              ↓ batched per channel, debounced
              ↓ deterministic pre-filter: only candidate batches go on
2. THINK   model call: the four L3 heads (always) + the trigger's context
           (a channel window, a subtree, a diff) + open work (L2) + relevant L4
              ↓ structured drafts, or nothing; a deterministic judge drops what fails
3. ROUTE   draft → the one party that can make it real
           (needs: shaper for roots and direction; the parent's holder below that;
           the DRI for done) — published as kind 50100, signed by the agent
```

### Where it runs

A new crate, **`crates/buzz-org-agent`**, a long-running binary. It connects with
`buzz-ws-client` (NIP-42 auth, subscriptions, publish), builds events with `buzz-sdk`
builders, and talks to a model through the same provider configuration `buzz-agent` uses
(`BUZZ_AGENT_PROVIDER`, `OPENAI_COMPAT_BASE_URL`, …), so Buzz Mesh works as a provider out of
the box. Identity, credentials, and lifecycle are those of any **managed agent** (NIP-AP,
`kind:30177`): the community owner creates it from the desktop Agents view with the built-in
"Org agent" persona, and `BUZZ_RELAY_URL` / `BUZZ_PRIVATE_KEY` / `BUZZ_AUTH_TAG` are injected
the same way. The relay learns which pubkey is the community's org agent from a
`["role", "org-agent"]` tag on that managed-agent event — the only thing the role unlocks is
the done-from-talk exception (Protocol §5.5).

Why a dedicated crate rather than a persona behind `buzz-acp`: the org agent needs structured
outputs validated against fixed schemas, a deterministic judge between the model and the
relay, subscriptions to state kinds rather than mentions, timers, and a cost model that scales
with candidate batches. `buzz-acp` is built for conversational agents that answer when
mentioned. The two share identity, deployment, and the relay client; they do not share a
pipeline.

### Triggers

Rules trigger; models explain. Three sources feed THINK, all deterministic:

- **State changes** (subscriptions, no scheduler needed). A new `39100` head → diff its
  objectives against live roots; each line no root cites is a candidate (move 1). A root
  entering `open` → a DRI-suggestion candidate (move 1b). An item entering `accepted` → its
  brief against its children is a candidate (move 2). A root entering `in_review` → the
  review candidate (move 3). A root entering `done` → the objectives-redraw candidate. The
  last child of an item closing → the done-card candidate.
- **Clock.** Monday: the gap scan over `io_work_items` + L3 — objectives with no live root,
  live roots with no open child and no `done` in the window, objectives near their date with
  little under them. Friday: the health read for every live root (move 4). Both run inside
  the agent; one agent per community makes a single timer sufficient.
- **Talk** (HEAR). Candidate batches only: mentions of open work or its holders, questions
  addressed to a room, commitment verbs, anything a Shaper says in `#shapers` or their DM with
  the agent (direction talk is always a candidate — the room and the role are the filter).
  THINK judges candidates; it does not read everything everyone says.

Drafts carry `origin: talk | gap` and a `gap` key. Dedupe and dismissal are enforced per key:
one open draft per key (the relay rejects a second while the first is `open`), and a declined
key is suppressed until the L3 head or the subtree it points at changes. This is what keeps
unprompted suggestions rare enough to be read.

### The judge

Between THINK and the relay sits a deterministic check the model cannot talk its way past:
every receipt resolves; `objective_ref` names a live line; `due_at` is inside the parent's
date; the suggested holder is a member with no more than the configured open items; the
payload matches the schema for its draft kind. Anything that fails is dropped and written to
the ledger as `draft_dropped` (through `buzz org ledger note`) so the evaluation tally counts
it. The relay repeats the receipt check at ingest — the judge is the agent's own gate, not the
only one.

### Done-from-talk

Only a message **authored by the item's DRI** — in a channel or their DM with the agent —
moves an item to `done`, and the message is the receipt. The agent recognises the sentence and
sends `io_done` carrying that message id; the relay verifies the message's author is the
`dri`, that it is fresh and not a transcript, and executes. That is not an AI write on
inferred speech: the author is known from the signature, the check is the same one the **Mark
done** button runs, and the agent only relays. Anyone else saying "it's done" produces nothing
(at most a nudge to the DRI). Transcripts never count; a done heard on a call is surfaced to
the DRI as a `done` draft, and the DRI's reply is the confirm. The DRI can `io_reopen` for
seven days.

### Personal Assistant

Your DM with the org agent. Buzz DMs are channels (`kind:41010` opens one), so the agent reads
and writes there exactly as in a room. It is the **publish door**: draft a direction, project,
money, or DRI proposal; create a ticket (including a child under a ticket you hold — you
confirm it yourself; the project DRI does not); mark your own ticket done; ask the org
anything. Each answer that changes something is a card whose tap is a command signed by you.

"Ask the org anything" (feature 9) is the same context recipe in reverse: answer from L3 +
live L2 aggregates, search L1 (NIP-50) for receipts, cite event ids. Live numbers are fetched
at question time — never from memory.

### Memory of its own

The agent keeps operational state (last scan time, per-key suppression, judge thresholds) in
NIP-AE engrams (`kind:30174`) under its own key — encrypted, owner-readable, not org memory.
Org memory is L2–L4 on the relay, readable by everyone.

---

## Surfaces

Five doors in the desktop, one feature folder: `desktop/src/features/org/`. Routes under
`/org`, a primary-menu group in the sidebar beside Inbox, Projects (git), Agents, and
Workflows. Every read is a REQ filter over the state kinds (Protocol §6.5); every write is a
signed command through the existing `sign_event` → `EVENT` path. No new HTTP endpoints, no
new Tauri data commands beyond signing.

| Door           | Route                 | Reads                                                                                                                        |
| -------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| **Overview**   | `/org`                | the four `39100` heads (empty slots shown as such), `39103`, root `39101`s, members. Each direction card opens `/org/direction/$slug`: full text, every confirmed version (passed `direction` proposals), and per line the **proofs** — ledger facts that bear on it, each linking to its receipt. The agent's read is shown as the agent's, never as fact. |
| **Work**       | `/org/work`, `/org/work/$itemId` | roots with their subtree, DRI or _open_, dates; an item page with children, trail, and (roots) the latest `50101` health read |
| **Decisions**  | `/org/decisions`      | `39102` open and decided, filters Work / Money / Direction / Join; vote buttons for Shapers                                  |
| **My Work**    | `/org/my-work`        | `39101` where I am `dri` or offered; `50100` where `needs` is me (or `shaper` if I am one); open `39102` for Shapers. Three columns: Needs your answer, You hold, You offered. |
| **My Profile** | existing profile view | extends `features/profile` with current work, earlier work, paid (settled `money` where I am payee), recent decisions       |

The Buzz **Inbox** (Home) also carries the cards: `needs_action` gains open offers naming me,
drafts addressed to me, and — for Shapers — open proposals (relay-side, in
`buzz-db/src/store/feed.rs`). A card in Inbox and a card on My Work are the same event.

Cards share one component set: **draft card** (title, why, receipts, primary tap = the
command with the draft tag, secondary = decline with reason chips), **offer card**
(accept / decline), **decision card** (agree / decline, who has voted), **done card** (mark
done / not yet), **review card** (brief, recommendation, open the follow-up / nothing more /
keep open until). Chips on every card: _AI is asking you_ vs a person's name; the breadcrumb;
the date.

**Naming.** The board door is **Work**, not _Projects_: `desktop/src/features/projects/` is
the git forge (NIP-34 / NIP-MP) and has the `/projects` route. Product documents written before
this design say _Projects_; the behaviour is identical.

**CLI.** `buzz org` in `buzz-cli`: `direction`, `work` (`tree`, `show`, `offer`, `accept`,
`decline`, `done`, `release`, `set-due`), `proposals` (`list`, `vote`, `settle`), `shapers`,
`drafts` (`list`, `decide`), `health`, `ledger` (`list`, `note`). Agent-facing operations go
in `buzz-cli` first (AGENTS.md); the org agent itself uses the SDK builders directly, and the
integration tests drive the relay through the CLI.

**Mobile.** Later. Kinds are mirrored into `nostr_models.dart` when the protocol lands so the
mobile app can at least render the cards it receives.

---

## Feature → design map

| Feature                        | Reads                                              | Writes                                                                                                                                          |
| ------------------------------ | -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Direction stays current     | `39100` heads                                      | `50100` (kind `direction` or `objectives`) → `io_direction_propose` → `io_vote` → new `39100` head                                             |
| 2. The org listens             | —                                                  | nothing: L1 is the relay                                                                                                                        |
| 3. Talk becomes work           | channel window + L3 + L4 + tree                    | `50100` (kind `project` or `ticket`; root → `needs: shaper`, child → the holder); from talk, the state hook, or the Monday scan                 |
| 4. Offered, never assigned     | `39101`                                            | `io_offer` / `io_accept` / `io_decline`; `dri` proposal passing writes `dri`                                                                    |
| 5. Talk moves work             | messages                                           | DRI's own message → agent `io_done` with the message as receipt (Protocol §5.5); transcripts → `50100 kind=done` nudge                          |
| 6. Homes (five doors)          | `39100–39104`, `50100`, `50101`                    | —                                                                                                                                               |
| 6a. Join                       | `39102 kind=join`                                  | `io_join_propose` → `io_vote` → NIP-43 add member                                                                                               |
| 7. Money via proposals         | done item + trail                                  | `io_money_propose` → `io_vote` → `io_money_settle`                                                                                              |
| 8. Reviews write themselves    | ledger + L4 for the root, the line it cites        | scheduler → `in_review`; `50100 kind=review` `needs: shaper`; follow-up tap = `io_project_propose`; close on `due_at` by rule                     |
| 8a. Health                     | ledger for the root                                | `50101` weekly and on change; Shapers' `io_health_rate`                                                                                        |
| 9. Ask the org anything        | L3 + ledger aggregates + NIP-50 search             | —                                                                                                                                               |
| 10. Newcomers                  | profile + open work + L3                           | profile (person confirms); the agent's pool of suggested holders                                                                                |

---

## Build order

Each step ships value without the ones after it. Steps 1–3 are
[Phase 0](../plans/intelligent-org-phase-0.md).

1. **Protocol in the relay.** Kinds in `buzz-core`; `handlers/intelligent_org/` in the
   command executor with the single `apply()` write path; `io_*` projections and migration;
   `io_scheduler`; Shapers room sync; `needs_action` sources; `buzz org` CLI; `buzz-sdk`
   builders; `buzz-test-client` integration tests for every invariant in Protocol §5. No AI,
   no UI. The spine.
2. **Three doors.** Overview, Work (with item page), My Work in `features/org/`, plus the
   card set and the Inbox sources. Decisions folded into My Work for Shapers at first.
   Visible progress; forces the filters to be right.
3. **The org agent, move by move, with the harness first.** `buzz-org-agent` skeleton
   (client, L3 loader, judge, publisher) and the evaluation harness; then moves in the Phase 0
   order — 1 (direction → projects), 4 (health), 2 (project → tickets), 3 (completion → what
   next). State-change and clock triggers only; no HEAR yet. Shadow mode before cards.
4. **Decisions and money.** The Decisions door; vote and settle flows; `join`; My Profile
   sections.
5. **HEAR.** Channel and DM listening with the pre-filter; talk-derived drafts; direction
   from `#shapers` talk; the Personal Assistant publish flows; done-from-talk (last in this
   step — it needs the role tag and the transcript tag to be in place).
6. **Later.** Transcript tagging in the huddle STT pipeline; document text extraction;
   pay-agreed-in-chat; a community directory for feature 10's "no community in mind"; mobile
   cards.

---

## Known risks

Ranked, with where each is addressed:

1. **Authority creep in the agent.** The one exception (done-from-talk) is exactly where an
   "it's basically the same" second exception would start. The role tag unlocks one command
   under five checks and nothing else; any new agent-authored command is a protocol change
   with its own section. (Protocol §5.5.)
2. **Card noise.** Buzz's Inbox is zero-notification by default; the org agent must not be
   the thing that breaks that. Per-key dedupe and suppression are relay-enforced, the judge
   drops weak drafts, and the evaluation plan's acceptance bars gate every widening from
   shadow to cards. (Agent section; AI evaluation plan.)
3. **Tag-filter coverage.** The doors rely on `#needs`, `#item`, `#parent`, `#status`
   filters. Single-letter tags are indexed natively; verify multi-letter tag filtering in
   `buzz-db` and add an index migration if needed before step 2. (Protocol §6.5.)
4. **Model cost and auditability.** Without the deterministic pre-filter, THINK runs on every
   batch of chat with unbounded cost and becomes a model-as-trigger. Cost scales with
   candidates; every draft names its trigger. (Agent section.)
5. **Depth without discipline.** A recursive tree lets work fragment into trees nobody can
   read. The cascade rule holds the one invariant that matters; keeping money off the tree
   removes the other. Work shows roots and one level; deeper is behind the item page. Watch
   median depth on the dogfood community; past three, the product has a problem the schema
   cannot fix.
6. **One community, one org.** The design assumes it. A person in several communities has
   one key and several profiles (Buzz already works this way); a single org that spans
   several communities is not modelled and should not be until someone needs it.

---

## What we do not build

- No fine-tuning, no knowledge graph, no autonomous memory writes.
- No treasury and no AI-initiated payments — money is decided by proposal and settled outside.
- No assignment. There is no code path that puts work on a person without their accept,
  except a Shaper `dri` vote, which is a decision, not an assignment.
- No new HTTP endpoints and no second event stream: commands are events, state is events,
  the ledger is a projection of both.

---

## Related

- [The Intelligent Organization — What it is](../product/intelligent-org-features.md) — the target
- [The Intelligent Organization — Protocol](./intelligent-org-protocol.md) — kinds, tags, schemas, state machines
- [The Intelligent Organization — User Journeys](../product/intelligent-org-journeys.md) — DRI, Shaper, member, and the org agent's own flows
- [The Intelligent Organization — Current State](./intelligent-org-current-state.md) — what Buzz has today and the gap
- [Intelligent Org on Buzz — Phase 0](../plans/intelligent-org-phase-0.md) — steps 1–3 as a dogfood plan
- [The Intelligent Organization — AI Evaluation Plan](../plans/intelligent-org-ai-evaluation.md) — pass/fail bars and the harness for the agent's four moves
- [Organizational Intelligence — Memory Architecture](./organizational-intelligence.md) — the four layers
- [VISION.md](../../../VISION.md), [VISION_AGENT.md](../../../VISION_AGENT.md), [VISION_ACTIVITY.md](../../../VISION_ACTIVITY.md) — the Buzz product this sits inside
