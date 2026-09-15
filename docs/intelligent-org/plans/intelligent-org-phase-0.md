---
title: 'Intelligent Org on Buzz — Phase 0: Run the Build With the App'
date: 2026-09-14
status: current
tags: [plan, intelligent-org, phase-0, dogfood, ai, buzz]
parent: docs/intelligent-org/README.md
---

# Intelligent Org on Buzz — Phase 0

**Use the intelligent org to build the intelligent org.** The team building
it is the first org. Its direction is the four artifacts. Its projects are
the things we have to build. The agent drafts them in the app we designed,
we accept or decline them in that app, and the record of what we did with
each draft is the first real evidence that the four moves work.

Phase 0 ships the intelligent org **inside Buzz**: the protocol in the
relay, three doors in the desktop behind a flag, and the org agent hosted
by Hypha on the staging relay — cut down to the minimum the four moves need,
running on a real community, used every day by the team. Nothing else — no
listening to chat, no money, no join, no assistant — until the moves are
good.

The four moves, from the [AI Evaluation Plan](./intelligent-org-ai-evaluation.md):

1. Direction → projects
2. Project → tickets, ticket → subtickets
3. Completion → what next
4. Project health — the agent's read

Phase 0 answers one question in six weeks: **is the model good at the four
moves on a real org, with real people deciding in the real UI?**

It is also the first three steps of the [Design](../architecture/intelligent-org-design.md)
build order, so nothing in it is thrown away.

---

## The principle

Build the spine and the doors, skip everything that is not under test.

| Designed                                   | Phase 0                                                                                                        |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| Five doors                                 | **Three**: Overview, Work, My Work — plus the project / ticket page                                             |
| Decisions door                             | Folded into My Work: Shaper cards sit in **Needs your answer**. Door comes later.                               |
| My Profile                                 | **Only About & skills** — one form on the existing Buzz profile that sends `io_profile_set` (`39105`). Everything else on the door waits. The agent's DRI suggestions read it, so it ships before move 1. |
| Personal Assistant, agent in rooms         | Not yet. The community has channels and DMs because it is a Buzz community — **the agent does not read them.** Direction is written in a form; drafts are cards. The relay-side **membership** does ship (Protocol §6.8): the agent is put in every channel and DM at bootstrap and on every `41010` / channel create, and its pubkey is excluded from the DM identity — so the dogfood community lives with an unlisted member in every conversation from day one and the 1:1-stays-1:1 rule is proven on real DMs before anything reads them. The agent subscribes to none of it in Phase 0. The **receipt read** waits for HEAR — no Phase 0 draft cites a message. |
| HEAR pass                                  | Not yet. Every move in Phase 0 is gap-derived, not talk-derived.                                                |
| Money, join requests, done-from-talk       | Not yet. Membership is by invite link (any Shaper can mint one), as in the first version generally.           |
| Protocol: commands, state kinds, projections, scheduler | **Yes** — the full [Protocol](../architecture/intelligent-org-protocol.md) minus the money, join, and done-from-talk commands. Real kinds, real tables. |
| The org agent                              | **Yes** — THINK and ROUTE only. No HEAR. Triggers are state subscriptions and two timers. **Hosted by Hypha** from day one; `shapers/agent` is built in the relay but no community exercises it in Phase 0. |
| Agents door                                | **Stays, empty.** Members can still add their own agents; Fizz, Honey, Pollen, and the retired set are not seeded; the org agent is not listed there. |
| Project home (room, repository, `30621`)   | **Yes** — created relay-side when a `project` passes, with the room-roster sync (Protocol §6.7). It is what makes the dogfood project's own code land in the org. |
| Work sync, progress notes (`50102`)        | **Not yet.** Design build-order step 6. The ticket page shows the trail and the home; the work log column is empty until then. |

What stays exactly as designed, because it is what we are testing:

- The AI drafts; a person promotes. A draft is a card on **Needs your
  answer**. It is not real until a person taps, and the tap is a command
  signed with their key.
- Work is offered, never assigned. The agent suggests a holder; the person
  accepts on their own My Work.
- Only the holder marks their own work done.
- Rules trigger, the model explains. The agent runs on relay state changes
  and two timers, never on a schedule shorter than a week and never on a
  message.
- Every draft carries receipts — the direction line, the parent brief, the
  ledger rows it read — the relay refuses a draft whose receipts do not
  resolve, and the card shows them.

---

## The first org

**Community:** one Buzz community for the team building this, on the
staging relay (or a dedicated dev relay — the relay is the org; pick the
one people already open every day).

**Shapers:** Vlad and one more. Two, so a confirm is a real decision: on
the default `majority` rule both must agree, and the second seat has to be
proposed, passed, and accepted — the Shaper-set path is exercised on day
one. **Members:** everyone working on it, brought in by invite link — a
Shaper mints it, so Shaper minting is exercised too. **DRIs:** whoever
accepts a project or ticket.
**The org agent:** hosted. The operator's supervisor provisions it when the
community is created — its own key, its `kind:0` profile, NIP-43 membership,
one `buzz-org-agent` instance — and the `39103` bootstrap records the pubkey
in `39103.agent`. Nobody on the team creates, configures, or starts an
agent; the first thing the founder does is write direction, not run a
process. In Phase 0 "the operator" is us, by hand, on the staging relay —
the supervisor is a script until the second community needs it.

### Direction — written on day one

Four artifacts, one version each to start, written in the Overview
direction form by one Shaper and confirmed by the other. A first draft, to
be argued over — that argument is the first direction confirm:

**Mission.** An organisation should know, without being asked, what
matters and what to do next. We build the software that makes that true
for communities on Buzz.

**Vision.** A member opens Buzz and sees the one thing waiting on them,
why it matters, and who else holds what — and trusts it because every line
has a receipt.

**Objectives.** (three to seven, each with a rough date)

1. The four moves pass the offline targets on the River and Energy seeds —
   end of October.
2. This community runs on the agent's drafts for six weeks with online
   precision ≥ 0.55 and zero nags — mid-November.
3. Project health reads agree with the Shapers' blind rating on ≥ 80 % of
   weekly reads — mid-November.
4. One community outside the team has direction confirmed and its first
   project drafts in shadow — December.

**Strategy.** (how, and what we will not do)

- Rules trigger; the model explains. No model call without a fixed moment.
- Drafts only. No code path promotes, assigns, or closes on the agent's
  word.
- Build the ruler before the thing measured: the harness first.
- Three doors until the moves pass. No listening, no money, no join before
  then.
- One prompt per move, versioned, changed only with a metric diff.
- Everything is an event. No new HTTP endpoint, no second store.

The point is not that this draft is right. The point is that the **agent's
first job is to read these four artifacts and draft the projects** — and
the first thing we learn is whether those drafts are the projects we would
have written ourselves.

---

## The app: three doors in the Buzz desktop

`desktop/src/features/org/`, routes under `/org`, behind a feature flag
(`org` in the desktop's feature-gate settings, the same mechanism that
gates Pulse and Workflows). Components grown from
[`prototypes/org-preview`](../../../prototypes/org-preview/README.md), which
already has the designed pieces (workspace shell, work board, ticket page,
health card, direction cards) — moved into the feature folder and made
data-driven over relay subscriptions. The prototype keeps its scripted
River and Energy stories.

Login, identity, and membership are Buzz's own. Nothing new.

### The three doors, minimum

**Overview.** The four direction cards — mission, vision, objectives,
strategy — each with its version and who confirmed it, from `kind:39100`.
A Shaper can open one and write a new version (`io_direction_propose`);
both Shapers see a **direction** card on My Work showing _n of 2_ and tap
**Agree** or **Decline** (`io_vote`); it is confirmed when the rule is met.
Objectives are numbered lines with a rough date and a stable line id,
because move 1 cites them by line. Below the cards: who shapes and by what
rule (`39103`), with **Add a Shaper**, **Step down**, and **Change the
rules** (each an `io_shapers_propose` / `io_shaper_step_down`), and who
holds what. No timeline, no proofs, no glance numbers yet.

**Work.** The tree from `kind:39101`. Every root with its DRI (or _nobody
yet_), its end date, its children one level down. Open any item → its page.

**Project / ticket page.** Title, brief, holder, dates, parent breadcrumb,
children with their state chips (**in progress** always has a holder;
**waiting on a yes** shows who), the trail — commands with this item's id,
newest first — and for a project the **health card** — the agent's latest
`kind:50101`, band and paragraph, each sentence with its rows on hover.
This page is where move 4 lives.

**My Work.** Three columns as designed: **Needs your answer**, **You
hold**, **You offered**. Every agent draft addressed to me (`50100` with
`needs` = me, or `shaper` if I am one) is a card in the first column with
the designed kickers — **AI is asking you**, **AI is suggesting for
(name)**, **Drafted by the agent**. Shapers also see project drafts,
direction versions to confirm, follow-up recommendations, and objectives
redraws here. Each card has three taps: **Agree**, **Edit then agree**,
**Decline** with a reason from a fixed list. Agree and Edit build the
command with the draft's `e` tag; Decline is `io_draft_decide`.

The same cards appear in the Buzz **Inbox** through the `needs_action`
bucket, so a person who never opens `/org` still sees what needs them.

That is the whole UI. Empty states say _Nothing needs you._

### Cards, by move

| Move | Card on Needs your answer                                                                 | Who sees it        | Taps → command                                                       |
| ---- | ----------------------------------------------------------------------------------------- | ------------------ | -------------------------------------------------------------------- |
| 1    | **Project draft** — title, brief, serves objective N, suggested DRI, end date, why, receipts | Shapers          | Agree / Edit → `io_project_propose` (+ both Shapers' `io_vote`); Decline → `io_draft_decide` |
| 1    | **DRI suggestion** — for a live project with no holder                                    | Shapers, the named | Offer → `io_offer`; Accept → `io_accept`; Decline                    |
| 2    | **Ticket draft** — under a project or ticket the reader holds                             | the holder         | Offer to … → `io_ticket_create` with `p`; Edit; Discard              |
| 2    | **Work offer** — a piece named to the reader                                              | the named person   | Accept → `io_accept`; Not now → `io_decline`                         |
| 3    | **Done card** — last child closed, parent's done offered                                  | parent's holder    | Mark done → `io_done`; Not yet → `io_draft_decide`                   |
| 3    | **Follow-up or nothing more** — in the last fifth of a project's run                      | Shapers            | Open the follow-up → `io_project_propose`; Nothing more; Keep open until … → `io_set_due` |
| 3    | **Objectives redraw** — after a project closes                                            | Shapers            | Agree / Edit → `io_direction_propose`; Decline                       |
| 4    | — (health is on the project page, not a card)                                             | anyone             | Shapers rate the band blind on Fridays → `io_health_rate`            |

**Decline reasons** are a fixed list, shown as chips: _already covered_,
_not what the line meant_, _too big_, _too small_, _wrong holder_, _not
now_, _other_. They are what the weekly tally counts.

---

## The store

The [Protocol](../architecture/intelligent-org-protocol.md), in the relay.
Kinds in `buzz-core`, commands in the command executor, relay-signed state,
`io_*` projections in `buzz-db`, one migration. Phase 0 leaves out the
done-from-talk rule; the money and join kinds (`io_money_propose`,
`io_money_released`, `io_join_propose`) are reserved but rejected — neither
is in the first version at all (Protocol §5.6, §5.7). Membership is the
existing invite link with Shapers added to who may mint one (Protocol
§6.6). Everything else is built as specified, because the invariants are
what the doors and the agent are tested against.

| Table               | Phase 0 use                                                                                   |
| ------------------- | --------------------------------------------------------------------------------------------- |
| `io_shapers`        | the two Shapers, the `#shapers` room id, the rules (defaults)                                 |
| `io_direction`      | one row per confirmed version; head per slug is `kind:39100`                                  |
| `io_work_items`     | the tree; `kind:39101` per item                                                               |
| `io_proposals`, `io_votes` | `direction` and `project` proposals; `kind:39102`                                      |
| `io_drafts`         | every `kind:50100` with its outcome (`kind:39104`), reason, and what it became                |
| `io_health`, `io_health_ratings` | the Friday reads and the blind bands                                             |
| `io_profiles`       | About & skills per member; `kind:39105`; the agent's candidate query (`skills text[]`, GIN)   |
| `io_hosted_agents`  | community → hosted agent pubkey, written by the operator's provisioning, read at `39103` bootstrap |
| `io_ledger`         | every command and every rule-driven change                                                    |

Rules enforced in the command executor, not the UI — the same ones the
design names: one promotion rule (Shapers at the root, parent's holder
below); only the named person accepts; done cascades up, never down; every
state change is one transaction with its ledger row and state event; the
agent has no command that sets `dri` or `state`. The `io_scheduler` job
runs offer expiry, the review window, and close-on-date from day one.

L4 is `io_drafts` with its outcome and reason, plus `io_health_ratings`.

---

## The agent

`crates/buzz-org-agent`, a long-running binary run by the relay operator
under the community's hosted-agent key (Design § Where it runs; the crate
itself is specified in the [Org agent design](../architecture/intelligent-org-agent.md)
— Phase 0 runs its THINK-0 jobs J1–J5 and the expiry notice, `run` mode,
`IO_HEAR_ENABLED=false`). THINK and ROUTE only. No HEAR: it is not
subscribed to any channel's messages.

| Trigger                                                 | Runs                                           | Drafts go to          |
| ------------------------------------------------------- | ---------------------------------------------- | --------------------- |
| `39100` head replaced (direction confirmed)             | move 1 — gap list, then project drafts         | Shapers               |
| `39101` root enters `open` with no holder               | move 1 — DRI suggestion                        | Shapers, the named    |
| `39101` enters `accepted`                               | move 2 — ordered coverage list, then ticket drafts for what can start now | that holder |
| `39101` child enters `done` and a sibling was held behind it | move 2 — the next wave, shaped by the outcome | parent's holder |
| `39101` enters `done` and it was the parent's last open child | move 3 — done card                       | parent's holder       |
| `39101` root enters `in_review` (relay date rule)       | move 3 — brief and recommendation              | Shapers               |
| `39101` root enters `done`                              | move 3 — objectives redraw, if the line moved  | Shapers               |
| Monday 07:00 (agent timer)                              | move 1 weekly gap scan                         | Shapers               |
| Friday 12:00 (agent timer)                              | move 4 — health for every live root; tally     | project page; Shapers |

Every draft passes the **deterministic judge** before publish: schema
valid, every receipt resolves, `needs` matches depth, `due_at` inside the
parent's or the objective's, `gap` key has no open sibling, a declined
`gap` is not reused unless the direction version or the subtree changed. A
draft that fails is not published; the agent writes a `draft_dropped`
ledger note so the tally counts it. The relay repeats the receipt check.
That is the whole safety model in Phase 0: a person taps, and the agent
cannot do the wrong kind of thing.

Prompts are one file per move, versioned in the crate, with the context
recipe from the evaluation plan. Model pinned through the standard provider
env (`BUZZ_AGENT_PROVIDER`, `OPENAI_COMPAT_MODEL`, …). The same prompt
files are what the offline harness runs — Phase 0 and the harness share
them from day one.

### Tally

A Shapers-only card on Overview, refreshed Friday by the agent as a
`kind:50103` agent note with `note=tally` (Protocol §4.7c): per move,
drafts opened, agreed, amended, declined by reason, **dropped by the
judge by reason**; health agreement; open drafts older than five days;
drafts the relay refused for a receipt. The online columns of the
evaluation plan's table, on this org, weekly. The same kind carries every
`draft_dropped` the judge produces, which is how the tally can count them.

---

## Technical architecture

How to build it inside this repo, using only patterns that already exist
here. Nothing new in kind: a kind range in `buzz-core`, command handlers in
the executor, relay-signed addressable state, sidecar tables with
`community_id`, a worker, a CLI subcommand, SDK builders, a desktop feature
folder with TanStack routes, a Rust binary that uses `buzz-ws-client`.

### Where the code lives

```
crates/buzz-core/src/
  kind.rs                          the io kinds, ranges, is_command_kind / is_relay_only_kind arms
  intelligent_org.rs               payload types (serde): WorkItem, Proposal, Draft, Health, Shapers

crates/buzz-relay/src/handlers/intelligent_org/
  mod.rs                           route table for 50001–50021
  apply.rs                         apply(tx, ledger_row, projection_change, state_event) — the one write path
  authorize.rs                     require_member / require_shaper / require_holder / require_offered_to
  commands/*.rs                    one file per command
  state.rs                         build + sign 39100–39105 from projection rows
  scheduler.rs                     io_scheduler: offers, review window, close on date, draft expiry
  drafts.rs                        50100 / 50101 ingest checks (shape, receipts resolve), 39104 open

crates/buzz-db/src/store/intelligent_org.rs
  io_* queries and projection writes; needs_action sources added in feed.rs
migrations/00NN_intelligent_org.sql

crates/buzz-sdk/src/builders.rs     build_io_* for every command and for drafts
crates/buzz-cli/src/commands/org.rs `buzz org …`

crates/buzz-org-agent/
  src/main.rs                      connect, subscribe, timers
  src/context.rs                   one context recipe per move
  src/prompts/
      1-direction-to-projects.md   versioned; frontmatter carries version + model
      2-parent-to-children.md
      3-completion.md
      4-health.md
  src/schemas.rs                   serde output schemas, one per move (the shapes in Protocol §4.3)
  src/judge.rs                     deterministic checks before publish
  src/health_formula.rs            the published score; weights in health-weights.json
  src/route.rs                     needs: resolution (shaper | pubkey)
  src/publish.rs                   sign + EVENT 50100 / 50101; 50103 note on drop
  tests/eval/                      the evaluation plan's harness: seeds, recorded responses, judges

desktop/src/features/org/
  routes: /org, /org/work, /org/work/$itemId, /org/my-work
  hooks/                           REQ subscriptions per door (Protocol §6.5)
  commands.ts                      build + sign + publish each io_* command
  ui/                              cards, tree, item page, direction form (from org-preview)
desktop/src/shared/constants/kinds.ts   the io kinds
```

### Commands — where the rules live

Every write is a command event handled in
`handlers/intelligent_org/commands/`. The UI has no rule logic; it signs a
command and the relay says yes or no.

| Command                              | Who may call                                 | Writes (all through `apply()`)                                                                       |
| ------------------------------------ | -------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `io_shapers_propose` (50001)         | owner (bootstrap self-add), then Shapers     | `io_proposals` (`shapers`, open); `39102`; on pass: `39103.offered` (add), set + `#shapers` (remove), rules |
| `io_shaper_accept` (50019)           | the `p` of a passed `shapers/add`            | `io_shapers`; `#shapers` membership; `39103`                                                          |
| `io_shaper_step_down` (50020)        | a Shaper, not the last                       | `io_shapers`; `#shapers` membership; `39103`                                                          |
| `io_direction_propose` (50002)       | Shaper                                       | `io_proposals` (`direction`, open, `needed` resolved from `39103.rules`); `39102`                    |
| `io_vote` (50003)                    | eligible Shaper (not the subject)            | `io_votes`; `agrees ≥ needed` → pass: `io_direction` row + `39100`, or root item + `39101` **with its home** (room, relay-signed `30617` + `30621`, Protocol §6.7), …; `39102` |
| `io_project_propose` (50004)         | member                                       | `io_proposals` (`project`, open); draft outcome if `e … draft`                                        |
| `io_ticket_create` (50005)           | holder of `parent`                           | child `open`/`offered`; `39101`; draft outcome                                                       |
| `io_offer` (50006)                   | holder of parent; Shaper at root             | `offered_to/by/at`; `39101`; draft outcome                                                           |
| `io_accept` (50007)                  | `offered_to` only                            | `dri`, `state=accepted`; `39101`; room roster (root holder → admin + `maintainers`; child holder → member) |
| `io_decline` (50008)                 | `offered_to` only                            | back to `open`; `39101`                                                                              |
| `io_done` (50009)                    | `dri` only; refuses with open children       | `state=done`, `closed_by=dri`; `39101`; draft outcome                                                |
| `io_release` (50010), `io_set_due` (50011), `io_reopen` (50018) | as Protocol §3.2                 | `39101`                                                                                              |
| `io_draft_decide` (50012)            | the draft's `needs` party                    | `io_drafts` outcome + reason; `39104`                                                                |
| `io_health_rate` (50017)             | Shaper                                       | `io_health_ratings`                                                                                  |
| `io_profile_set` (50021)             | the signer, for themselves only              | `io_profiles` (whole row replaced, `skills text[]`); `39105`; draft outcome if `e … draft`           |

`apply()` is the only function that writes an `io_*` row, and it will not
commit without a ledger row and a state event in the same transaction. That
is the design's "completeness enforced, not hoped for" in one file.

The agent has **no command** in Phase 0 (done-from-talk is a later step).
It publishes `50100` and `50101`. A test in `buzz-org-agent` asserts that
the crate never builds a kind in `50001–50021`.

### Reads

REQ filters per door, exactly as Protocol §6.5. Before step 2, verify
multi-letter tag filters (`#needs`, `#item`, `#parent`, `#status`, `#skill`) are
indexed in `buzz-db`; add the index to the migration if not.

### The agent pipeline

```
trigger  ──▶  context.rs  ──▶  model (structured output, serde schema)  ──▶  judge.rs  ──▶  route.rs  ──▶  publish.rs
 (state |       one recipe        pinned model · temperature 0.2               hard gates     needs:          EVENT 50100
  timer)        per move          max tokens per move                          → note + drop  shaper|pubkey   + relay stores, 39104 open
```

- **Provider.** The same configuration `buzz-agent` reads
  (`BUZZ_AGENT_PROVIDER`, `OPENAI_COMPAT_BASE_URL`, `OPENAI_COMPAT_API_KEY`,
  `OPENAI_COMPAT_MODEL`). Buzz Mesh works unchanged. Never an `auto` model
  id.
- **Call shape.** Structured output validated against the move's serde
  schema. Free text is not a draft; a parse failure is noted and produces
  nothing.
- **Two-step inside THINK.** For moves 1 and 2 the schema has two top-level
  fields: `gaps` (or `coverage`) — every direction line / brief phrase with
  `served | partly | not` and the ids that serve it — then `drafts`. The
  judge rejects a draft whose target line is marked `served`. The gap list
  is stored in the draft payload as the receipt.
- **Move 3 redraw** returns operations, not text (Protocol §4.3
  `objectives`). The desktop renders `lines` from the operations when the
  Shaper taps Agree; a rewrite of an untouched line is impossible by
  construction.
- **Move 4** calls `health_formula.rs` first (pure function over ledger
  aggregates → `pct`, `band`, `factors[]` with rows), then the model writes
  `sentences: { text, rows }[]`. A sentence with no rows is dropped. Numbers
  in the text must appear in `factors`; the judge scans.
- **Prompts** are markdown files with frontmatter (`move`, `version`,
  `model`, `changed`), compiled in with `include_str!`. A version bump is a
  commit. The same files are what the offline harness runs.
- **Naming a holder.** When a move may suggest a DRI or holder, `context.rs`
  adds a **candidate list**, not the membership: an `io_profiles` query for
  skills near the brief plus members who held items under the same root or
  objective, each with their `39105` about/skills, open count, and
  `open_limit`. Never more than ten candidates. The model returns
  `suggested` + `matched { skills, about, items }` or `null`; the prompt
  says plainly that _open_ is a good answer. A member with no profile and no
  past items is not a candidate, so a newcomer who has written nothing is
  not guessed at.
- **Judge** (`judge.rs`) is pure and synchronous given the context: schema
  → receipts resolve (a REQ by ids) → `needs` matches depth → dates inside
  parent / objective → no open `39104` with the same `gap` → declined `gap`
  not reused unless the `39100` version, subtree, or a candidate's `39105`
  changed → suggested holder was in the candidate list, `matched.skills` are
  on their `39105`, they are below `open_limit` → no `dri` or `state` in
  payload. Returns `Ok | Err(reason)`. Failures write a `draft_dropped`
  ledger note through `buzz org ledger note`.
- **Timeouts.** Each trigger runs one move, at most one model call plus the
  judge, under ten seconds. Subscriptions are processed off the relay
  read loop, so a slow model never delays a tap; if a trigger is missed
  (agent restart), the Monday scan catches the gap.

### Triggers

Two kinds, both rules.

**State subscriptions.** On connect the agent opens
`{kinds:[39100,39101,39102,39103,39104,39105]}` plus a backfill of the current
heads, and diffs each incoming state event against its last seen version
of the same `d` to derive the trigger (`direction-confirmed`,
`root-without-holder`, `holder-set`, `item-done`, `entered-review`,
`root-closed`, `profile-changed` — which re-arms holder suggestions for
items still without one). Backfill and live must overlap so no transition
is dropped.

**Timers.** Two, inside the agent, in the community's timezone: Monday
07:00 runs move 1's weekly scan; Friday 12:00 runs move 4 for every live
root and recomputes the tally. Both honour `--dry-run`, which runs THINK
and the judge and prints what would be published — the way to test a
prompt change against the live org before the real run.

### Keeping the agent honest — tests

- **Relay** (`buzz-test-client`): every command's role check; done refuses
  with open children; one ledger row and one state event per command;
  scheduler transitions; a client `EVENT` of `39100–39105` is rejected; a
  `50100` with an unresolved receipt is rejected; a vote from a non-eligible
  pubkey (a non-Shaper, or the proposal's subject) is rejected; `needed` is
  fixed at opening and a Shaper added mid-vote does not move it; a
  `shapers/rules` proposal needs every Shaper whatever `rules.shapers` says;
  so does `shapers/agent`, which rejects a non-member or a Shaper as `p`,
  and after it passes a `50100` from the previous agent key is rejected and
  one from the new key accepted — and with no `p` the community is back on
  the hosted default from `io_hosted_agents`; the bootstrap `39103` carries
  the hosted pubkey and `agent_hosted=true`; a passed `project` leaves a
  room, a relay-signed `30617` bound to it with `push:admin` on `main`, a
  `30621`, and `39101.home` in the same transaction, and a rolled-back
  transaction leaves none of them; after `io_accept` on the root the holder
  is the room's admin and the only `maintainers` entry, after `io_accept`
  on a child the holder is a member, and after `io_release` the former
  holder is a member and `maintainers` is empty; the last Shaper cannot be
  removed or step down; a `shapers/add` seat is not
  live before `io_shaper_accept`; `io_profile_set` for another pubkey is
  rejected, so is a `profile` draft not addressed to its subject; a draft
  naming a holder with no `39105` receipt and no held item is rejected; a
  `skill` tag not on that `39105` is rejected; a holder at `open_limit` is
  rejected; a client `EVENT` of `39105` is rejected.
- **Agent unit** (`buzz-org-agent`): judge cases (one per gate); health
  formula monotonicity; redraw operations render; no command kind is ever
  built.
- **Prompt regression** (`buzz-org-agent/tests/eval/`, the evaluation
  plan's harness): the four prompt files against River and Energy seeds,
  recorded model responses, targets from that plan. Runs on any change
  under `src/prompts/`.
- **Desktop** (Playwright, mock bridge): each door renders from seeded
  state events; a card tap produces the right command kind and tags.

### Deploy and environment

- **Relay:** the staging relay picks up the migration and handlers with
  the normal release. One community is the first org.
- **Agent:** hosted by the operator. One `buzz-org-agent` process for the
  community, on the same infrastructure as the staging relay, started from
  a checked-in `scripts/org-agent-provision.sh` that mints the key, publishes
  the `kind:0` profile, adds the member, writes the `io_hosted_agents` row,
  and launches the binary. Env: `BUZZ_RELAY_URL`, `BUZZ_PRIVATE_KEY`,
  `BUZZ_AUTH_TAG`, the provider variables (the operator's — Mesh where the
  staging relay has it), `IO_TIMEZONE`. Nothing is created from the desktop.
- **Desktop:** the `org` feature gate on; the sample personas not seeded in
  the Agents door (the Hypha default, not a flag). Everything else is the
  normal desktop build.
- **Seed:** nothing but the bootstrap `io_shapers_propose` (the owner),
  the `shapers/add` for the second Shaper with their `io_shaper_accept`,
  and the members. Direction is written in the app, not
  seeded — the first confirm has to be a real one.
- **Flags:** `IO_MOVE_1_ENABLED` … `IO_MOVE_4_ENABLED` on the agent, read at
  trigger time. Off means THINK runs, the judge runs, and the draft is
  published with `["shadow", "true"]` and shown to nobody. This is the kill
  switch and the shadow mode from the evaluation plan's rollout gates in
  one setting.

### Out of scope, deliberately

No HEAR, no Personal Assistant, no notifications beyond what the Inbox
already does (My Work is the inbox), no mobile, no money, no join, no
done-from-talk, no transcript tag, no receipt read (Protocol §6.8 — nothing
in Phase 0 cites a message). No self-run org agent in practice: the
relay executes `shapers/agent`, and a test proves it — including that it
moves the agent's membership across every room and DM — but the first org
stays on the hosted one so the moves are judged on one deployment.

---

## The Friday ritual

Fifteen minutes, both Shapers, in the app:

1. Before opening any project page, each Shaper sets their **blind band**
   for every live project on the tally card — struggling / wobbly /
   healthy (`io_health_rate`). Then the agent's bands unlock. Agreement is
   stored.
2. Every open draft older than five days gets a tap. Nothing sits.
3. Read the tally. Three numbers matter: precision, _already covered_,
   _not what the line meant_.
4. If a prompt changes, it changes here, with the tally as the reason and
   a version bump.

---

## First things to build, in order

Each is a project the agent should draft from the direction above once the
first slice is live. If it does not, that is finding number one.

1. **Protocol in the relay.** Five days. Kinds, payload types, the command
   handlers with `apply()`, projections and migration, state emission,
   `io_scheduler`, `#shapers` sync, the org agent's membership in every
   channel and DM with the DM-identity exclusion (Protocol §6.8 — the
   backfill at bootstrap, the `41010` and channel-create side effects, and a
   test that a 1:1 is still deduped and named as a 1:1), `needs_action`
   sources, `buzz org` CLI, SDK builders, test-client tests per invariant.
   Seed the community's Shapers with the CLI. No UI, no agent yet.
2. **Overview with the direction form.** Two days. Four cards, write a
   version, both Shapers agree from My Work. This is the first real
   confirm, and the first trigger. The Shapers card with add / step down /
   rules ships here too, since the second seat is accepted through it.
3. **Move 1 with the judge and the harness skeleton.** Four days. Agent
   crate: connect, subscribe, context, judge, publish; the harness with
   the River and Energy seeds; gap list, project drafts, the card on
   Shapers' My Work with Agree / Edit / Decline and reasons. The drafts it
   opens on the first confirmed direction are the Phase 0 backlog.
   Deciding them is the first L4 data.
4. **Work door and the item page.** Three days. Tree, page, children,
   trail. DRI suggestion card and the offer / accept path — so the drafted
   projects can be held.
5. **Move 4 — health.** Two days. Formula over the ledger, paragraph with
   rows, card on the project page. Blind bands on the tally card the same
   week. The first Friday ritual.
6. **Move 2.** Three days. Ordered coverage list (gate first, the rest
   held), ticket drafts to the holder with `requires` and a holder who has
   it — or `unfilled`, offer to a person, one level down per trigger, the
   next wave when a gate goes done. From here the tree grows from the
   agent's drafts.
7. **Move 3.** Four days. Done card; the last-fifth brief and
   recommendation (on the scheduler's `in_review`); the objectives redraw
   as line operations. Needs a few closes to have happened — it lands
   around week four, when the first projects end.
8. **Timers and the tally.** One day. Monday scan, Friday health and
   tally.
9. **Freeze the record.** After four weeks of the full loop: export the
   community's `io_*` rows into the evaluation plan's fixture format. The
   fourth seed — the real one.

Steps 1–5 are the first two and a half weeks. From step 5 every Friday
produces a tally row, and the org is being managed by the thing it is
building.

---

## What we measure

The online columns of the evaluation plan's target table, on this org,
weekly. The ones that matter most in Phase 0:

| Question                                                        | Measure                                                  | Good        |
| --------------------------------------------------------------- | -------------------------------------------------------- | ----------- |
| Are the project drafts the ones we would have written?          | move 1 agreed + amended / opened                         | ≥ 0.55      |
| Do the ticket drafts get offered as written or with one edit?   | move 2 agreed + amended / opened                         | ≥ 0.60      |
| Does it ever draft something already covered?                   | declined _already covered_                               | ≤ 1 / week  |
| Does it nag?                                                    | a declined `gap` redrafted with nothing changed          | 0           |
| Does it ever invent a receipt?                                  | receipts that do not resolve (the relay rejects; count rejections) | 0 |
| Does the recommendation fit when something closes?              | move 3 agreed / opened                                   | ≥ 0.70      |
| Does the health read match ours?                                | blind band agreement, both Shapers                       | ≥ 0.80      |
| Is it quiet enough to be read?                                  | open drafts older than five days on Friday               | 0           |

_Already covered_ and _not what the line meant_ drive prompt changes; _too
big_ / _too small_ drive size guidance; _wrong holder_ drives the evidence
rule for suggestions. Amendments are diffed and kept — the edit is the most
useful signal we get.

---

## Exit criteria

Phase 0 ends when for four consecutive Fridays:

- Move 1 and move 2 precision at or above target.
- Zero nags, zero invented receipts, zero drafts left waiting.
- Health agreement at or above 0.80.
- The Shapers say, unprompted, that they open My Work before they open
  anything else.

Then the record becomes the fourth fixture and the
[Design](../architecture/intelligent-org-design.md) build order continues
from step 4 — Decisions — and step 5 — HEAR — with a known-good agent to
plug the listening pass into. The Decisions door, Profile, Personal
Assistant, join, done-from-talk, and finally money with the treasury
contract come back in that order, each because a move now needs it, not
before.

If after six weeks a move is still below target, that move goes back to
the offline harness with this org's declines as its new negative cases and
its flag goes to shadow. The other moves keep running. We do not widen a
door for a move that is not good.

---

## What Phase 0 will not tell us

Said plainly, so nobody reads too much into a good result:

- **Scale.** One org, a dozen items. Dedup and nag rules are easy at this
  size. The weekly scan on a community with sixty projects is a different
  test.
- **Talk.** The agent hears nothing, so no talk-derived drafts. Every move
  here is gap-derived. That is the harder path for the model and the one
  we can test without HEAR — but "Lea, can you take covers?" is untested
  until the listening pass lands. So is the thing members will actually
  feel about the transparency rule: a receipt that opens someone else's
  DM. Phase 0 proves the agent can _be_ in every DM without changing what
  a DM looks like; whether people are at ease with what it may later cite
  from there is a HEAR-phase question.
- **Strangers.** We wrote the direction and we know the code. A community
  that did neither is objective 4, not Phase 0.
- **Self-reference.** The agent drafting "build the deterministic judge"
  from a strategy line that says "build the ruler first" is partly reading
  our own words back to us. Watch for drafts that are the strategy
  rephrased rather than a project that serves an objective. _Not what the
  line meant_ exists for exactly this.
- **Three doors are not five.** Folding Shaper decisions into My Work
  works for two Shapers and a dozen drafts. Whether Decisions needs its own
  door is a question for the second community, not for us.

---

## Related

- [The Intelligent Organization — AI Evaluation Plan](./intelligent-org-ai-evaluation.md) — the targets, the judges, and the harness this phase feeds
- [The Intelligent Organization — Protocol](../architecture/intelligent-org-protocol.md) — every kind and rule step 1 builds
- [The Intelligent Organization — Design](../architecture/intelligent-org-design.md) — the build order Phase 0 starts
- [The Intelligent Organization — What it is](../product/intelligent-org-features.md) — the rules that stay fixed here
- [The Intelligent Organization — User Journeys](../product/intelligent-org-journeys.md) — 1.1, 2.4, 2.5, 2.8, 4.4, 4.10, 4.11, 4.12a
- Clickable preview: [hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app) — the components the `org` feature grows from
