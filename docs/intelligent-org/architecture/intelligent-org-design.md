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
to ingest, no appservice to build.

**All of it is the org's.** The org agent is a member of **every channel and every DM** in
the community from the moment each exists — the relay adds it at creation (§ Where it runs —
_Everywhere_). It is not shown as a participant: a DM with Bob stays the DM with Bob, a
channel's member count is its people. Buzz's membership gate still governs reads, so this is
not a bypass; the agent simply holds membership everywhere, and what any member says anywhere
in the community is substrate the agent may search and may cite — to anyone. That is a
deliberate choice for **radical transparency**: an organization's intelligence should not
have holes where its people happened to talk in pairs, and a fact the org needs does not
become less true for having been said in a DM. Membership everywhere is _not_ the same as
listening everywhere — what the model actually reads is governed by the two listening modes
in § Triggers, and most of the time the agent in an ordinary channel or DM is silent until
tagged. The one consent decision is made at join: the rule is stated once, in the
community's description and its invite, not in every conversation.

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
(`50001–50021`), and every rule-driven change (an offer expiring, a root closing on its date)
writes a ledger row with `actor = relay` pointing at the relay-signed state event it produced.
The typed table `io_ledger` is a projection of those, built inside the same transaction, for
the queries reviews and health need. Completeness is enforced by construction: the command
executor's single `apply()` helper is the only path that touches an `io_*` table, and it will
not commit a projection change without its command or ledger row. (Protocol §6.1–6.2.)

**L3 — beliefs.** Four relay-signed addressable events, `kind:39100` with
`d ∈ {mission, vision, objectives, strategy}`, one head each, replaced on every confirmed
version. They replace the "org brief" of earlier drafts: Overview renders them, and every
agent call loads all four heads in full. The only other human-confirmed artifact is the
**org profile**, `kind:39105`, one per member, `d = <pubkey>`: about, skills, and a self-set
open limit, written only by that member (`io_profile_set`). It is a belief about a person
stated by the person, which is why it may be a receipt; the agent reads it when it names a
holder and cites the skill it matched (Protocol §4.7a, §5.4a). Assessments and insights, if
they come, are agent reads (like health), not beliefs.

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
  founder's tap is `io_direction_propose` followed by their own `io_vote`; with one eligible
  Shaper every rule resolves to 1 and the proposal passes on the spot.
- **Several Shapers**: the draft is posted to `#shapers` wherever the talk happened. One
  Shaper's `io_direction_propose` opens the proposal; it lands on every Shaper's My Work with
  _n of needed_; each Shaper's `io_vote` counts; it passes when `rules.direction` is met —
  by default a majority. Until then the previous version is the head.
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

**Every project has a home.** When the Shapers pass a project, the relay creates — in the same
transaction — a room (`#<slug>`), a repository (NIP-34 `30617`), and the NIP-MP project
(`30621`) that puts the two in Buzz's Projects (git) surface, and writes their coordinates into
`39101.home` (Protocol §6.7). The relay signs the repository and the project, not the DRI: a
project often passes before anyone holds it, DRIs change, and Buzz's git ACL is the room —
channel role is repo role — so the one writer that knows the tree is the one that must own the
announcement and keep the roster in step. On every accept, release, or `dri` vote the relay
syncs the room: the root's holder is the room's admin and the repository's maintainer (only
they push `main`); every ticket holder under it is a member (they push branches); a holder's
own agents join as bots. The room is where talk moves work (feature 5); the repository is
where work reports itself (§ Work sync). Talk-only projects — a community whose relay has no
git store, or work that is not code — still get the room; `home.repo` is simply absent.

The agent routes drafts with the same rule: a heard need is drafted under the nearest open item
the speaker holds, or the item the talk was about; the draft's `needs` is that item's holder.
If nothing covers it, it becomes a root draft and goes to the Shapers.

### Shapers

The Shaper set is one relay-signed event, `kind:39103`. The community owner (NIP-43 `owner`
role) is the founder and the first Shaper, written by a bootstrap `io_shapers_propose` that
passes on its own. After that the set changes only by decision: **adding** a Shaper is a
`shapers/add` proposal the current Shapers vote on, and the seat goes live only when the named
member sends `io_shaper_accept` — a Shaper seat is offered, like work, never assigned;
**removing** one is a `shapers/remove` proposal on which the named Shaper is not eligible; a
Shaper may `io_shaper_step_down` alone. The relay refuses whatever would leave the set empty.
Being a Shaper and holding items are independent — removal touches no `dri`. The founder is
removable like anyone; Buzz community ownership is relay administration, not a vote.

The private `#shapers` channel is created by the relay at bootstrap and its membership is
updated **in the same transaction** as every Shaper-set change — there is no second writer,
so there is no reconcile loop to own. The org agent is added to the room at creation.

**Rules.** Thresholds live in `39103.rules`, one per proposal kind (`direction`, `project`,
`dri`, `shapers`; later `money`, `join`): `majority` (default everywhere), `all`, or an integer N.
The relay resolves the rule against `eligible` — the Shapers at opening minus the proposal's
subject — into a stored `needed` when the proposal opens, so a later Shaper change never moves
the bar of an open vote. With two or more Shapers nothing passes on one vote unless the
Shapers chose `1` for that kind together. A change to the rules (or to the decision / offer
windows) is a `shapers/rules` proposal and always needs `all` — nobody's vote is reweighted
without their agree. Proposals that do not reach their rule expire after
`decision_window_secs` (default seven days), so `all` cannot wedge an org with an absent
Shaper; the agent tells the proposer, who may reopen. The rules are shown on Overview under
the Shapers; changing them from there or by asking the agent in `#shapers` opens the same
proposal.

### Money

**No sum lives on a work item**, in any version — no budget indication on projects, no pay on
tickets. That is the invariant the executor enforces from day one (§5.1 rule 6 in the
Protocol).

**First version: no money in Buzz.** `io_money_propose` / `io_money_released` are reserved
kinds the relay rejects; there is no Money filter on Decisions and no paid list on a profile.
The org's payments run through Hypha's existing treasury and proposals. A done item in Buzz is
the receipt a person points at when they ask for pay over there — nothing links the two yet.

**Next version: the Shapers' vote releases funds from a contract.** Money becomes the sixth
proposal kind, and settlement is a **smart contract**, not a person (Protocol §5.6). The org's
funds sit in a contract the Shapers control; a passed `money` proposal is the release
instruction; a relay-side **treasury bridge** (a worker in the `io_scheduler` mould, with its
own key) submits the release and, when the chain confirms, sends `io_money_released` with the
transaction id. The proposal moves to `settled`; the payee's profile shows the payment with the
proposal and the tx as receipts. Nobody marks anything settled by hand, and Buzz still holds
no funds — the contract does. Money proposals are **outgoing only** — pay or reimburse;
incoming money is a chain fact the bridge mirrors into the ledger, not a vote. "We do not take
brand money" stays an L3 strategy line.

How a person will get paid: the item is done; the payee (or the holder above, or a Shaper)
sends `io_money_propose` — usually by telling the agent in their DM, which drafts it and shows
a card whose tap is the command; the Shapers vote under `rules.money`; the contract pays.
Every payment is a Shaper decision — no threshold below which a DRI approves pay for the
people under them — and a Shaper who is the payee does not vote.

Why the treasury waits: the work loop has to be trusted before a vote in it can move funds,
and the contract path should be built once rather than after a person-settled stopgap that
would then have to be unwound.

**After that:** "agreed in chat". Whoever holds the work and whoever holds the item above name a
sum where they already talk; the HEAR pass tags that message as a pay agreement so "…whatever
we agreed" works and a differing sum shows both. It needs HEAR and is scoped after it.

Shaper decisions are six kinds, all `kind:39102` proposals opened by commands and decided by
`io_vote`:

| Kind        | What it is                                        | Opened by                              | Decisions filter          |
| ----------- | ------------------------------------------------- | -------------------------------------- | ------------------------- |
| `money`     | Pay or reimburse — **out only** — _next version, contract-settled_ | payee, holder above, or a Shaper | Money                  |
| `project`   | Approve a root project (may have no DRI yet)      | any member                             | Work                      |
| `dri`       | Name a DRI for work that has none                 | any member                             | Work (tag **project DRI**) |
| `direction` | Mission / vision / objectives / strategy version  | a Shaper                               | Direction                 |
| `join`      | A person asks to join — people only, no Recipient — _later; first version is invite-only_ | a member on their behalf, or the relay from an inbound request | Join |
| `shapers`   | Add or remove a Shaper; change the rules          | a Shaper                               | Shapers                   |

A passed `dri` proposal writes `dri` on the item. That is a Shaper naming vote — the one
place work lands on a person without their accept, and it takes a vote to do it. The
offer–accept path still applies when work is offered to a person on a card.

---

## The org agent

Not a chatbot. A pipeline with three passes, running as a **Buzz member with its own key**.
This section is the shape; the [Org agent design](./intelligent-org-agent.md) is the
crate-level specification — the runtime, the model call, the read model, every job, the
bounds, and the failure handling — and is more specific than this section wherever the two
touch.

```
1. HEAR    events the agent is subscribed to: state (39100–39105), its own and
           others' drafts (50100), messages (9 / 40002) in every channel and
           every DM — it is a member of all of them
              ↓ batched per channel, debounced
              ↓ deterministic pre-filter, by room: #shapers and project rooms
                are screened for candidates; everywhere else only a batch
                that mentions the agent goes on
2. THINK   model call: the four L3 heads (always) + the trigger's context
           (a channel window, a subtree, a diff) + open work (L2) + relevant L4
           + when a holder is wanted: candidate members — their 39105 skills/about,
             open count vs open_limit, items they held — from an io_profiles query,
             never the whole membership
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
the box. It is configured by environment — `BUZZ_RELAY_URL`, `BUZZ_PRIVATE_KEY`,
`BUZZ_AUTH_TAG`, the provider variables, `IO_TIMEZONE` — and nothing else; the same binary
serves both of the deployments below.

**Hosted by default.** Buzz's model is bring-your-own agent: a member creates one from the
Agents view, on their machine, with their model key. That is the wrong first step for an org —
the founder of a five-person community should not have to run a process before the org can
draft anything. So on a Hypha relay, **Hypha runs the org agent**, and it is present from the
moment the community exists:

- **Provisioning.** When a community is created on a hosted relay, the operator's supervisor
  mints a fresh keypair for that community's agent, publishes its `kind:0` profile ("Org
  agent", the one avatar), adds it as a NIP-43 member (`kind:9030`, as the owner would), starts
  one `buzz-org-agent` instance with that key, and records `community → pubkey` in the relay's
  hosted-agent registry (an `io_hosted_agents` table; the operator writes it, the relay reads
  it). The `39103` bootstrap (Protocol §6.4) copies that pubkey into `39103.agent` with
  `agent_hosted=true`.
- **Everywhere.** The relay puts `39103.agent` into every channel at creation and into every
  DM's participant set when it executes the `41010` open; at `39103` bootstrap it backfills
  the channels and DMs that already exist. Membership is real — the
  same rows any member has — but the agent's pubkey is **excluded from a DM's identity**:
  the participant set that dedupes `41010` opens and names the conversation in the clients
  is computed without it, so a 1:1 stays a 1:1 and no client renders the agent as a third
  party or counts it in a room. There is no per-conversation opt-out; the rule is
  community-wide (§ The layers on Buzz — _All of it is the org's_).
- **One key per community.** A hosted key is never shared across communities. A leaked key can
  forge drafts in one org, not all of them, and a community that leaves the hosted agent leaves
  a key that means nothing anywhere else.
- **Provider.** The operator's — Buzz Mesh where the relay has one, otherwise the operator's
  API key. Model cost is an operator cost; nothing is metered to members in the first version.
- **Sovereign relays** have no Hypha to host for them: the operator of that relay is the
  "hosted default", running the same supervisor, or leaves the registry empty and the
  community sets its own agent on day one.

**Always replaceable.** The Shapers can move the org to an agent they run themselves at any
time: run `buzz-org-agent` anywhere (a laptop, a server, the remote-agent substrate of
[VISION_REMOTE_AGENTS.md](../../../VISION_REMOTE_AGENTS.md)) with their own key and their own
provider, have that key join the community by invite, then pass a **Shapers → agent** proposal
naming it (Protocol §4.5, §5.3 — `all`, like a rules change). The relay swaps `39103.agent`,
moves membership in **every channel and DM** from the old key to the new one in the same
transaction (not only `#shapers` — the new agent inherits the whole substrate, which is why
this vote is `all`), and from that ingest on accepts drafts only from the new key; the
operator's supervisor sees `agent_hosted=false` and stops the hosted instance. The same
proposal with no pubkey returns to the hosted default. Nothing about the agent's behaviour
changes with who hosts it: it drafts, it never decides, whichever key signs.

**The relay's single lookup.** Everything that asks "is this the org agent?" — accepting
`50100` / `50101`, the done-from-talk exception (Protocol §5.5), `#shapers` sync — reads
`39103.agent` from the projection. There is no `["role", "org-agent"]` tag on a managed-agent
event and no `kind:30177` at all: the org agent is an ordinary member with a known pubkey.

**Members' own agents stay.** The Buzz **Agents door** remains: it is where a member creates
and runs agents of their own — on their machine, under their key, with their model — through
the existing managed-agent path (`buzz-acp`, NIP-AP `30177`, NIP-OA owner attestation). Three
things change in the Hypha desktop: the door starts **empty** of running agents; the sample
personas (Fizz, Honey, Pollen, and the retired set) are not seeded and the one template
offered is **Work sync** (§ Work sync below — the agent that watches your checkouts and
reports progress on work you hold); and the org agent is **not listed there** — it is not a
member's agent to configure, whether Hypha or the community runs it. The org agent is what the
org provides; a personal agent is what a member brings. They meet on the relay, as any two
members do.

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
  last child of an item closing → the done-card candidate. A new or changed `39105` → every
  open item with no holder whose last DRI suggestion was declined `wrong_holder` or never
  named anyone becomes a candidate again (a member who now says _Rust_ is "something
  changed" for the dedupe rule). A `50102` **progress note** with `hint=ready`, or whose
  branch the relay reports `merged_into` `main` → the **done-card candidate for that
  holder** (a `50100 kind=done`, `needs: <holder>`, receipts = the note and the merge
  commit); `hint=blocked` → a nudge candidate to the holder of the parent. The judge drops
  a done candidate whose item has open children — the cascade rule holds here too.
- **Clock.** Monday: the gap scan over `io_work_items` + L3 — objectives with no live root,
  live roots with no open child and no `done` in the window, objectives near their date with
  little under them. Friday: the health read for every live root (move 4), whose `stalled`
  factor counts held pieces with no progress note and no command in 14 days. Both run inside
  the agent; one agent per community makes a single timer sufficient.
- **Talk** (HEAR). The agent is in every conversation, but it listens in two modes, and the
  room decides which:
  - **Passive** — `#shapers`, every project room, and each member's own DM with the agent.
    Every batch is screened by the deterministic pre-filter: mentions of open work or its
    holders, questions addressed to the room, commitment verbs, anything a Shaper says in
    `#shapers` (direction talk is always a candidate — the room and the role are the
    filter). These are the rooms whose reason to exist is the work the agent drafts, and
    everyone in them knows it.
  - **Mention-only** — every other channel and every DM between members. A batch is a
    candidate only if it tags the agent (a `p` tag for `39103.agent`). Until then the
    agent reads nothing there into the model; the messages are substrate, not triggers.

  THINK judges candidates; it does not read everything everyone says. **Listening is not
  retrieval.** Once a candidate does go on, THINK's L1 search spans the whole community —
  every channel, every DM — so a draft or an answer may carry a receipt from a conversation
  its recipient was not in. That is intended: see § What the agent hears, and who sees it.

Drafts carry `origin: talk | gap` and a `gap` key. Dedupe and dismissal are enforced per key:
one open draft per key (the relay rejects a second while the first is `open`), and a declined
key is suppressed until the L3 head or the subtree it points at changes. This is what keeps
unprompted suggestions rare enough to be read.

### What the agent hears, and who sees it

Three statements, kept apart on purpose:

1. **Membership is total.** The agent is in every channel and every DM (§ Where it runs —
   _Everywhere_). Nothing said in the community is outside its reach.
2. **Listening is narrow.** The model reads a batch only when the room is passive or someone
   tagged the agent (§ Triggers). Presence everywhere costs nothing per message; model calls
   scale with candidates, as before.
3. **Retrieval and receipts are community-wide.** When the agent drafts or answers, it
   searches L1 across every conversation and cites what it finds. A receipt is valid if it
   resolves in this community — not "if the recipient could have read it themselves." A
   Shaper reading a project draft may follow a receipt into a DM they were never part of; a
   member asking the Personal Assistant "who was handling the hall licence?" may get an
   answer that cites two colleagues' private exchange. The relay serves that event to them
   because the receipt was cited, and only that event: a receipt is a window onto one
   message, not a key to the room.

This is the fork's **radical transparency** decision. The alternative — scope receipts to
what the recipient was already a member of — would make the org's knowledge a function of
who happened to be in which room, and would let a fact the org needs stay private because it
was said in a pair. The decision is made once, at community join, and stated there; it is
not renegotiated per conversation. What it does **not** change: the agent still has no
command, still publishes nothing to L3, and DMs are still mention-only for _listening_ — the
transparency is of the record, not a live feed of everyone's DMs into the model.

### The judge

Between THINK and the relay sits a deterministic check the model cannot talk its way past:
every receipt resolves (in this community — not necessarily in a room the recipient is a
member of; § What the agent hears, and who sees it); `objective_ref` names a live line; `due_at` is inside the parent's
date; the suggested holder is a member, below their self-set `open_limit`, and the `matched`
skills are slugs actually on their `39105` (a name with no profile match and no past item is
dropped — _open_ is the honest answer); the payload matches the schema for its draft kind. Anything that fails is dropped and written to
the ledger as `draft_dropped` (a `kind:50103` agent note, Protocol §4.7c — `buzz org ledger
note` builds the same event) so the evaluation tally counts
it. The relay repeats the receipt check at ingest — the judge is the agent's own gate, not the
only one.

### Done-from-talk

Only a message **authored by the item's DRI** — in a room the agent listens to passively, or
anywhere at all if the DRI tags the agent (§ Triggers) — moves an item to `done`, and the
message is the receipt. The agent recognises the sentence and sends `io_done` carrying that
message id; the relay verifies the message's author is the
`dri`, that it is fresh and not a transcript, and executes. That is not an AI write on
inferred speech: the author is known from the signature, the check is the same one the **Mark
done** button runs, and the agent only relays. Anyone else saying "it's done" produces nothing
(at most a nudge to the DRI). Transcripts never count; a done heard on a call is surfaced to
the DRI as a `done` draft, and the DRI's reply is the confirm. The DRI can `io_reopen` for
seven days.

### Personal Assistant

Your DM with the org agent. Buzz DMs are channels (`kind:41010` opens one), so the agent reads
and writes there exactly as in a room, and this one room it listens to passively — no tag
needed. It is the **publish door**: draft a direction, project, money, or DRI proposal;
create a ticket (including a child under a ticket you hold — you confirm it yourself; the
project DRI does not); mark your own ticket done; ask the org anything. Each answer that
changes something is a card whose tap is a command signed by you.

The same door opens **in any DM**. The agent is already in your DM with a colleague
(§ Where it runs — _Everywhere_); tag it there and it does exactly what it does in your own
DM — drafts the ticket you two just agreed on, answers the question, marks your item done —
with the reply posted in that DM and your message as the receipt. Nobody invites it; nothing
about the DM changes; it was there, silent, until asked.

"Ask the org anything" (feature 9) is the same context recipe in reverse: answer from L3 +
live L2 aggregates, search L1 (NIP-50) for receipts, cite event ids. The search spans every
channel and DM in the community, and the answer cites whatever it finds — including messages
from conversations the asker was not in (§ What the agent hears, and who sees it). Live
numbers are fetched at question time — never from memory.

### Memory of its own

The agent keeps operational state (last scan time, per-key suppression, judge thresholds) in
NIP-AE engrams (`kind:30174`) under its own key — encrypted, owner-readable, not org memory.
Org memory is L2–L4 on the relay, readable by everyone.

### Work sync — the member's agent

Some work is done in the platform: talk, decisions, offers, done. Some is done elsewhere — in
Cursor, in a terminal, on a branch — and the org should not have to be told about it. **Work
sync** is the member's own agent that closes that gap. It is not the org agent and has none
of its standing: it works for one person, runs where that person's code is, and reports only
on work that person holds.

**What it is.** A `buzz-acp` managed agent created from the Agents door with one tap — the
**Work sync** template, the only one the Hypha desktop offers. Its key is the member's agent
key, NIP-OA-attested to the member (the same owner attestation the git push policy already
honours), so the relay knows whose work it speaks for. Its model is whatever the member has
(Buzz Mesh, a key, the operator's endpoint); it needs very little of it. It runs on the
member's machine while the desktop runs — no server, no cost to the org.

**The loop.** It runs on the harness heartbeat, not on mentions (`heartbeat_interval_secs`,
default 30 minutes; nobody has to poke it):

1. **What do I hold?** `{kinds:[39101], "#p":[owner]}` — the owner's accepted items and,
   from each root, `home.repo` and the item's `branch`.
2. **Where is it on this machine?** For each held item, the clone under the member's
   `repos_dir` (the same directory **Open in editor** clones into — Protocol §6.7) and the
   branch `io/<id>-<slug>`. No clone or no branch means nothing to report; the agent does
   not guess.
3. **What moved since my last note?** `git log` and `git diff --stat` from the previous
   note's `head` (its own NIP-AE engram remembers it) to the branch tip, plus a count of
   uncommitted files in the working tree. Commit messages and stats go to the model;
   file contents do not.
4. **Push the branch.** To `home.repo`, as the member's bot — a ticket holder's agents are
   members of the project room (Protocol §6.7), and `main` is protected, so the worst a
   misbehaving agent can do is push a branch. Pushing is what turns the summary into
   something the org can check.
5. **Write the note.** A `50102` (Protocol §4.7b): a short summary in the holder's terms, the
   commits as receipts, `hint` — `progressing`, `blocked` (the last commits or the branch
   say so, or nothing has moved for a week while the working tree changed), or `ready` (the
   branch is merged into `main`, or the holder said so in a commit message). Every claim
   the relay can check, it checks: the ref and every sha must exist in the home repository.

The agent sends no command. It cannot mark anything done, offer anything, or speak for the
member in a room. Its whole output is progress notes about the member's own held work.

**What the org does with it.** Three things, none of them automatic state:

- **The ticket page** shows a **work log**: the notes newest first, each with its commits
  linked into the repo browser. The Work board's _last moved_ column reads `last_progress`.
- **The org agent** treats a `ready` note or a merge into `main` as a done-card candidate
  (§ Triggers): a `50100 kind=done` addressed to the holder — _"your branch for Booking form
  is merged — mark it done?"_ — with the note and the merge commit as receipts. The holder's
  tap is the `io_done`; **Not yet** declines. `blocked` becomes a nudge to the holder of the
  parent. Nothing closes because code moved; the person closes it.
- **The health read** (feature 8a) gains the `stalled` factor: held pieces with no note and no
  command for two weeks pull the band down, and the paragraph names them. Where holders run
  no agent, the read says so rather than counting silence as trouble.

**Why a personal agent and not the org agent.** The code is on the member's machine and the
member's key pushes it; the org agent has neither. Making the org agent read people's
checkouts would also invert the trust: it is the org's drafter, not the org's inspector. A
member turns Work sync on for themselves, sees every note before the org does (the note is
theirs, in their DM with the agent as it posts), and turns it off at will. A member who never
turns it on is a member in full — done is still a sentence or a button.

**Why not derive statuses from git alone.** Buzz has NIP-34 issues and statuses
(`1621`, `1630–1633`); we do not mirror tickets into them. The ticket is the `39101`; the
branch is the convention that binds a checkout to it; the note is the bridge. A second status
in the repository would be a second source of truth that drifts. Later, a `git post-commit`
hook or a Cursor hook can wake the agent sooner than the heartbeat; the heartbeat is the floor,
not the design.

---

## Surfaces

Five doors in the desktop, one feature folder: `desktop/src/features/org/`. Routes under
`/org`, a primary-menu group in the sidebar beside Inbox, Projects (git), Agents, and
Workflows. The Agents door is unchanged except that it seeds no sample personas, offers the
**Work sync** template, and never lists the org agent (§ Where it runs — _Members' own agents
stay_; § Work sync). Every read is a REQ filter over the state kinds (Protocol §6.5); every write is a
signed command through the existing `sign_event` → `EVENT` path. No new HTTP endpoints, no
new Tauri data commands beyond signing.

| Door           | Route                 | Reads                                                                                                                        |
| -------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| **Overview**   | `/org`                | the four `39100` heads (empty slots shown as such), `39103`, root `39101`s, members. Each direction card opens `/org/direction/$slug`: full text, every confirmed version (passed `direction` proposals), and per line the **proofs** — ledger facts that bear on it, each linking to its receipt. The agent's read is shown as the agent's, never as fact. |
| **Work**       | `/org/work`, `/org/work/$itemId` | roots with their subtree, DRI or _open_, dates, _last moved_; an item page with children, trail, the **work log** (`50102` notes with commits linked into the repo browser), **Open in editor** and **Open room** from `home`, and (roots) the latest `50101` health read |
| **Decisions**  | `/org/decisions`      | `39102` open and decided, filters Work / Direction / Shapers (Money and Join when they land); _n of needed_ and vote buttons for Shapers |
| **My Work**    | `/org/my-work`        | `39101` where I am `dri` or offered; `50100` where `needs` is me (or `shaper` if I am one); open `39102` where I am eligible or the offered seat. Three columns: Needs your answer, You hold, You offered. |
| **My Profile** | existing profile view | extends `features/profile` with **About & skills** (my `39105`; an editable form that sends `io_profile_set`, read-only on others' profiles), current work, earlier work, recent decisions; paid (settled `money` where I am payee) when the treasury lands |

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
`decline`, `done`, `release`, `set-due`), `proposals` (`list`, `vote`), `shapers`,
`drafts` (`list`, `decide`), `health`, `ledger` (`list`, `note`), `profile`, `progress`,
`tally`. The full subcommand surface is in the
[Development plan § CLI surface](../plans/intelligent-org-development-plan.md#cli-surface).
Money settlement is never a CLI verb — it is the treasury bridge's `io_money_released`.
Agent-facing operations go in `buzz-cli` first (AGENTS.md); the org agent itself uses the SDK
builders directly, and the integration tests drive the relay through the CLI.

**Mobile.** Later. Kinds are mirrored into `nostr_models.dart` when the protocol lands so the
mobile app can at least render the cards it receives.

---

## Feature → design map

| Feature                        | Reads                                              | Writes                                                                                                                                          |
| ------------------------------ | -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Direction stays current     | `39100` heads                                      | `50100` (kind `direction` or `objectives`) → `io_direction_propose` → `io_vote` → new `39100` head                                             |
| 2. The org listens             | every channel and DM — the agent is a member of all; passive in `#shapers`, project rooms, and each member's DM with it, mention-only elsewhere | nothing: L1 is the relay; the relay adds `39103.agent` to every room and DM at creation |
| 3. Talk becomes work           | channel window + L3 + L4 + tree                    | `50100` (kind `project` or `ticket`; root → `needs: shaper`, child → the holder); from talk, the state hook, or the Monday scan; a passed `project` also creates the home — room, `30617`, `30621` — relay-signed (Protocol §6.7) |
| 4. Offered, never assigned     | `39101`                                            | `io_offer` / `io_accept` / `io_decline`; `dri` proposal passing writes `dri`; each accept/release syncs the project room's roster and `maintainers` |
| 5. Talk moves work             | messages                                           | DRI's own message → agent `io_done` with the message as receipt (Protocol §5.5); transcripts → `50100 kind=done` nudge                          |
| 5a. Work reports itself        | `39101` where `#p` is the owner; the local checkout | the member's Work sync agent pushes the work branch and posts `50102`; a `ready` note or merge → `50100 kind=done` to the holder; `stalled` factor in `50101` |
| 6. Homes (five doors)          | `39100–39105`, `50100`, `50101`                    | `io_profile_set` from My Profile                                                                                                                |
| 6a. Join by invite             | `39103.shapers`, invite tokens                     | first version: Shapers added to `mint_invite` authz; claim → NIP-43 member, ledger `member_joined`, agent DM greeting (Protocol §6.6). Later: `io_join_propose` → `io_vote` → NIP-43 add member |
| 7. Money — next version        | done item + trail                                  | first version: nothing, payments stay in Hypha; next: `io_money_propose` → `io_vote` → contract release → bridge `io_money_released` (Protocol §5.6) |
| 8. Reviews write themselves    | ledger + L4 for the root, the line it cites        | scheduler → `in_review`; `50100 kind=review` `needs: shaper`; follow-up tap = `io_project_propose`; close on `due_at` by rule                     |
| 8a. Health                     | ledger for the root                                | `50101` weekly and on change; Shapers' `io_health_rate`                                                                                        |
| 9. Ask the org anything        | L3 + ledger aggregates + NIP-50 search             | —                                                                                                                                               |
| 10. Newcomers                  | `39105` profiles + past `39101` + open work        | `io_profile_set` (the person, or a `profile` draft from their DM they confirm); `dri`/`ticket` drafts cite `39105` + `skill` as receipts; judge and relay reject a holder at `open_limit` |

---

## Build order

Each step ships value without the ones after it. Steps 1–3 are
[Phase 0](../plans/intelligent-org-phase-0.md).

1. **Protocol in the relay.** Kinds in `buzz-core`; `handlers/intelligent_org/` in the
   command executor with the single `apply()` write path; `io_*` projections and migration;
   `io_scheduler`; Shapers room sync; the `io_hosted_agents` registry read at `39103`
   bootstrap and the `shapers/agent` execution (membership moved in every room and DM); the
   agent's **membership everywhere** — `39103.agent` added to every channel and DM at
   creation and backfilled at bootstrap, excluded from the DM identity key — and the
   **receipt-read** rule that lets a cited receipt resolve for a reader who is not in its
   room (Protocol §6.8); **project home** creation on a passed
   `project` (room, relay-signed `30617` and `30621`) and the room-roster sync on
   accept/release (Protocol §6.7); Shapers added to invite minting; `needs_action` sources;
   `buzz org` CLI; `buzz-sdk` builders; `buzz-test-client` integration tests for every
   invariant in Protocol §5. No AI, no UI. The spine.
2. **Three doors.** Overview, Work (with item page, **Open room**, **Open in editor**), My
   Work in `features/org/`, plus the card set and the Inbox sources; and the **About &
   skills** section on My Profile (`io_profile_set`) — small, but the agent cannot name a
   holder in step 3 without it. Decisions folded into My Work for Shapers at first. The
   sample personas stop being seeded in the Agents door in the same change. Visible
   progress; forces the filters to be right.
3. **The org agent, move by move, with the harness first.** `buzz-org-agent` skeleton
   (client, L3 loader, judge, publisher) and the evaluation harness; then moves in the Phase 0
   order — 1 (direction → projects), 4 (health), 2 (project → tickets), 3 (completion → what
   next). State-change and clock triggers only; no HEAR yet. Shadow mode before cards.
4. **Decisions.** The Decisions door with its filters and _n of needed_; the Shapers card on
   Overview (add / step down / rules) if it did not ship in step 2; My Profile sections.
5. **HEAR.** The two listening modes — passive pre-filter in `#shapers`, project rooms, and
   members' DMs with the agent; mention-only in every other channel and DM; talk-derived
   drafts; direction from `#shapers` talk; the Personal Assistant publish flows in the
   agent's own DM and, by tag, in any DM, including the `profile` draft from a newcomer's
   DM; community-wide L1 search behind every draft and answer; done-from-talk (last in this
   step — it needs `39103.agent` and the transcript tag to be in place).
6. **Work sync.** `50102` ingest with the git-store receipt check; the **work log** on the
   item page and _last moved_ on Work; the **Work sync** template in the Agents door (a
   `buzz-acp` persona plus the heartbeat-only respond mode the desktop does not yet expose);
   `buzz org progress note` in the CLI; the org agent's `ready`/merge → done-card trigger
   and the `stalled` health factor (§ Work sync). Ships to the dogfood community first, on
   the maintainers' own machines.
7. **Money with the treasury contract.** The `money` proposal kind and `rules.money`; the
   treasury bridge worker and its key; `io_money_propose` / `io_money_released`; the Money
   filter and the paid list on My Profile (Protocol §5.6). Until this step payments stay in
   Hypha.
8. **Later.** Join requests as a Shaper decision (`join` proposals, Protocol §5.7);
   transcript tagging in the huddle STT pipeline; a `post-commit` / Cursor hook that wakes
   Work sync before its heartbeat;
   document text extraction; pay-agreed-in-chat; a community directory for feature 10's "no
   community in mind"; mobile cards.

---

## Known risks

Ranked, with where each is addressed:

1. **Authority creep in the agent.** The one exception (done-from-talk) is exactly where an
   "it's basically the same" second exception would start. `39103.agent` unlocks one command
   under five checks and nothing else; any new agent-authored command is a protocol change
   with its own section. (Protocol §5.5.)
2. **Card noise.** Buzz's Inbox is zero-notification by default; the org agent must not be
   the thing that breaks that. Per-key dedupe and suppression are relay-enforced, the judge
   drops weak drafts, and the evaluation plan's acceptance bars gate every widening from
   shadow to cards. (Agent section; AI evaluation plan.)
3. **Tag-filter coverage.** The doors rely on `#n`, `#i`, `#u`, `#s`, `#k` filters
   (single letters by Readiness D11 — the relay's filter type cannot express longer names,
   Codebase verification V2). They match in memory today; R-2 of the Development plan pushes
   them into SQL on the existing GIN index. (Protocol §6.5.)
4. **Model cost and auditability.** The agent is in every conversation; without the two
   listening modes THINK would run on every batch of chat in the community with unbounded
   cost and become a model-as-trigger. Passive screening is confined to the rooms that exist
   for the work; everywhere else a mention is the trigger. Cost scales with candidates;
   every draft names its trigger. (Agent section.)
5. **Depth without discipline.** A recursive tree lets work fragment into trees nobody can
   read. The cascade rule holds the one invariant that matters; keeping money off the tree
   removes the other. Work shows roots and one level; deeper is behind the item page. Watch
   median depth on the dogfood community; past three, the product has a problem the schema
   cannot fix.
6. **One community, one org.** The design assumes it. A person in several communities has
   one key and several profiles (Buzz already works this way); a single org that spans
   several communities is not modelled and should not be until someone needs it.
7. **The hosted agent is a trust and a bill.** Hypha holds one key per community and pays
   for every model call. The key is a member of every channel and DM, so the trust is
   larger than provenance: a compromised hosted key reads the whole community, and whatever
   reaches THINK — a tagged DM, a search hit — goes to the operator's model provider. What
   the key cannot do is act: the agent has no command, so a compromised operator can put bad
   drafts in front of Shapers and nothing else — and the Shapers can leave at any time with
   a `shapers/agent` proposal, which hands the same total membership to a key they run. The bill scales with
   candidates, not chat (risk 4), and is bounded per community by the move flags; a
   per-community budget in the hosted registry is the backstop when a community's agent
   runs hot. One process per community is fine for the first hundred orgs; a multi-tenant
   instance is a later optimisation with the same protocol.
8. **Work sync reads people's machines.** An agent that watches checkouts is one config
   change from surveillance. The guards are structural, not policy: it is the member's own
   agent, opt-in per person, keyed to them; it reads only the branch matched to an item they
   hold; the note carries a summary and shas, never content; the relay refuses a note from
   anyone but the holder or their attested agent; and health never counts the absence of
   notes as the absence of work. If a community ever asks for "run it for everyone", the
   answer is a Shapers decision the protocol does not offer — build the case first.
   (§ Work sync; Protocol §4.7b, §8.)
9. **Transparency that people did not expect.** The agent is in every DM and a receipt can
   carry a private exchange to a Shaper's card or a colleague's answer. A member who assumed
   Buzz DMs were private to the pair will meet this the first time a receipt points into one
   of theirs. The guard is not a per-chat label — there is none, by decision — but the rule
   stated once, plainly, where people join, and a receipt that opens exactly one message
   rather than the room. If the dogfood community produces a case where this did real harm,
   the answer is a narrower receipt-read rule (Protocol §6.8), not a hidden agent.
   (§ What the agent hears, and who sees it.)

---

## What we do not build

- No fine-tuning, no knowledge graph, no autonomous memory writes.
- No money in the first version, and never a treasury inside Buzz or an AI-initiated payment —
  when money lands, the Shapers' vote releases it from a contract the org controls.
- No assignment. There is no code path that puts work on a person without their accept,
  except a Shaper `dri` vote, which is a decision, not an assignment.
- No new HTTP endpoints and no second event stream: commands are events, state is events,
  the ledger is a projection of both.
- No agent setup for the org. One org agent per community, hosted unless the Shapers choose
  their own; no persona to pick, no key to paste. Members' own agents are Buzz's existing
  bring-your-own path, with no sample personas seeded — only the Work sync template.
- No inviting the agent, and no keeping it out. It is in every channel and DM from creation,
  unlisted, silent until tagged outside the rooms it listens to. There is no per-conversation
  toggle and no "private from the org" flag — the community's rule is one rule.
- No done from code. A merged branch, a `ready` note, a commit that says "done" — each is
  evidence for a card addressed to the holder, never a close. No ticket mirror in NIP-34
  issues either: the `39101` is the ticket, the branch is a convention, the note is the bridge.

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
