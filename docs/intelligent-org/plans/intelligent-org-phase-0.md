---
title: 'Hypha Intelligent Org — Phase 0: Run the Build With the App'
date: 2026-09-12
status: draft
tags: [plan, intelligent-org, phase-0, dogfood, ai, hypha]
parent: docs/intelligent-org/README.md
---

# Hypha Intelligent Org — Phase 0

**Use the intelligent org to build the intelligent org.** The team building
it is the first org. Its direction is the four artifacts. Its projects are
the things we have to build. The agent drafts them in the app we designed,
we accept or dismiss them in that app, and the record of what we did with
each draft is the first real evidence that the four moves work.

Phase 0 ships **Hypha Intelligent Org**: the designed UI, cut down to the
minimum the four moves need, running on a real store, used every day by
the team. Nothing else — no chat, no money, no join, no assistant — until
the moves are good.

The four moves, from the [AI Evaluation Plan](./intelligent-org-ai-evaluation.md):

1. Direction → projects
2. Project → tickets, ticket → subtickets
3. Completion → what next
4. Project health — the agent's read

Phase 0 answers one question in six weeks: **is the model good at the four
moves on a real org, with real people deciding in the real UI?**

---

## The principle

Build the spine and the doors, skip everything that is not under test.

| Designed                              | Phase 0                                                                                  |
| ------------------------------------- | ---------------------------------------------------------------------------------------- |
| Five doors                            | **Three**: Overview, Projects, My Work — plus the project / ticket page                   |
| Decisions door                        | Folded into My Work: Shaper cards sit in **Needs your answer**. Door comes later.         |
| My Profile                            | Not yet. A name and a Shaper flag on the membership row is the whole profile.             |
| Personal Assistant, rooms, Shapers room | Not yet. No chat at all. Direction is written in a form; drafts are cards.              |
| L1 talk ingestion (Matrix)            | Not yet. Every move in Phase 0 is gap-derived, not talk-derived.                          |
| Money, join, reviews page             | Not yet.                                                                                 |
| L2 ledger, `work_items`, L3 slots, L4 | **Yes** — minimal columns, real tables. These are design build steps 1–3 and 7, not throwaway. |
| The org agent                         | **Yes** — THINK and ROUTE only. No HEAR. Triggers are mutation hooks and one cron.        |

What stays exactly as designed, because it is what we are testing:

- The AI drafts; a person promotes. A draft is a card on **Needs your
  answer**. It is not real until a person taps.
- Work is offered, never assigned. The agent suggests a holder; the person
  accepts on their own My Work.
- Only the holder marks their own work done.
- Rules trigger, the model explains. The agent runs on four hooks and one
  cron, never on a schedule shorter than a week and never on a message.
- Every draft carries receipts — the direction line, the parent brief, the
  ledger rows it read — and the card shows them.

---

## The first org

**Space:** Hypha Intelligent Org — one space, seeded on first deploy.

**Shapers:** Vlad and one more. Two, so a confirm is a real decision and
the second-Shaper confirm path is exercised. **Members:** everyone working
on it, by email allowlist. **DRIs:** whoever accepts a project or ticket.

### Direction — written on day one

Four artifacts, one version each to start, written in the Overview
direction form and confirmed by the other Shaper. A first draft, to be
argued over — that argument is the first direction confirm:

**Mission.** An organisation should know, without being asked, what
matters and what to do next. We build the software that makes that true
for small orgs on Hypha.

**Vision.** A member opens the app and sees the one thing waiting on them,
why it matters, and who else holds what — and trusts it because every line
has a receipt.

**Objectives.** (three to seven, each with a rough date)

1. The four moves pass the offline targets on the River and Energy seeds —
   end of October.
2. This org runs on the agent's drafts for six weeks with online precision
   ≥ 0.55 and zero nags — mid-November.
3. Project health reads agree with the Shapers' blind rating on ≥ 80 % of
   weekly reads — mid-November.
4. One pilot space outside the team has direction confirmed and its first
   project drafts in shadow — December.

**Strategy.** (how, and what we will not do)

- Rules trigger; the model explains. No model call without a fixed moment.
- Drafts only. No code path promotes, assigns, or closes on the agent's
  word.
- Build the ruler before the thing measured: the harness first.
- Three doors until the moves pass. No chat, no money, no join before then.
- One prompt per move, versioned, changed only with a metric diff.

The point is not that this draft is right. The point is that the **agent's
first job is to read these four artifacts and draft the projects** — and
the first thing we learn is whether those drafts are the projects we would
have written ourselves.

---

## The app: Hypha Intelligent Org

`apps/intelligent-org` — grown from `apps/org-preview`, which already has
the designed components (workspace shell, work board, ticket page, health
card, direction cards). The preview keeps its scripted River and Energy
stories; the new app has one real org and a real store. Deployed on its own
Vercel project.

Login through Privy, as the rest of the platform. Membership is an
allowlist of emails on the space; the Shaper flag is a column on that row.

### The three doors, minimum

**Overview.** The four direction cards — mission, vision, objectives,
strategy — each with its version and who confirmed it. A Shaper can open
one and write a new version; the other Shaper sees a **direction** card on
My Work and taps **Agree** or **Decline**. Objectives are numbered lines
with a rough date, because move 1 cites them by line. Below the cards: who
shapes, who holds what. No timeline, no proofs, no glance numbers yet.

**Projects.** The tree. Every root with its DRI (or _nobody yet_), its end
date, its children one level down. Open any item → its page.

**Project / ticket page.** Title, brief, holder, dates, parent breadcrumb,
children with their state chips (**in progress** always has a holder;
**waiting on a yes** shows who), the trail of ledger rows, and for a
project the **health card** — the agent's read, band and paragraph, each
sentence with its rows on hover. This page is where move 4 lives.

**My Work.** Three columns as designed: **Needs your answer**, **You
hold**, **You offered**. Every agent draft is a card in the first column
with the designed kickers — **AI is asking you**, **AI is suggesting for
(name)**, **Drafted by the agent**. Shapers also see project drafts,
direction versions to confirm, follow-up recommendations, and objectives
redraws here. Each card has three taps: **Agree**, **Edit then agree**,
**Decline** with a reason from a fixed list.

That is the whole UI. Empty states say _Nothing needs you._

### Cards, by move

| Move | Card on Needs your answer                                             | Who sees it        | Taps                                 |
| ---- | --------------------------------------------------------------------- | ------------------ | ------------------------------------ |
| 1    | **Project draft** — title, brief, serves objective N, suggested DRI, end date, why, receipts | Shapers | Agree / Edit / Decline |
| 1    | **DRI suggestion** — for a live project with no holder                | Shapers, the named | Offer / Accept / Decline             |
| 2    | **Ticket draft** — under a project or ticket the reader holds         | the holder         | Offer to … / Edit / Discard          |
| 2    | **Work offer** — a piece named to the reader                          | the named person   | Accept / Not now                     |
| 3    | **Done bubble** — last child closed, parent's done offered            | parent's holder    | Mark done / Not yet                  |
| 3    | **Follow-up or nothing more** — at 80 % of a project's run            | Shapers            | Open the follow-up / Nothing more / Keep open until … |
| 3    | **Objectives redraw** — after a project closes                        | Shapers            | Agree / Edit / Decline               |
| 4    | — (health is on the project page, not a card)                         | anyone             | Shapers rate the band blind on Fridays |

**Decline reasons** are a fixed list, shown as chips: _already covered_,
_not what the line meant_, _too big_, _too small_, _wrong holder_, _not
now_, _other_. They are what the weekly tally counts.

---

## The store

Real tables in the existing Neon database through `storage-postgres`,
scoped to one space. Minimal columns; each is a design table with the
non-Phase-0 fields left off, so nothing is thrown away.

| Table                | Phase 0 columns                                                                                                         |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `io_memberships`     | `space_id`, `person_id`, `is_shaper`, `display_name`                                                                    |
| `io_direction`       | `space_id`, `kind` (mission / vision / objectives / strategy), `version`, `body`, `lines` (objectives and strategy as numbered lines), `confirmed_by`, `confirmed_at`, `proposed_by` |
| `io_work_items`      | `space_id`, `parent_id?`, `title`, `brief`, `dri_person_id?`, `offered_to?`, `offered_by` (person or `agent`), `due_at`, `approved_at`, `state`, `objective_ref?`, `root_id`, `depth` |
| `io_ledger`          | `space_id`, `actor` (person or `agent`), `verb`, `object_type`, `object_id`, `evidence` (json), `created_at`            |
| `io_drafts`          | `space_id`, `move` (1–4), `kind`, `needs` (shaper / person id), `gap_key`, `payload` (json), `receipts` (json), `state` (open / accepted / amended / declined), `decline_reason?`, `created_at`, `decided_at?` |
| `io_health`          | `work_item_id`, `week`, `pct`, `band`, `sentences` (json with rows), `shaper_bands` (json, blind)                       |

Rules enforced in the mutations, not the UI — the same ones the design
names: one promotion rule (Shaper at the root, parent's holder below); only
the named person accepts; done cascades up, never down; every state change
writes a ledger row; the agent never writes `dri_person_id`, never sets
`state`, only inserts into `io_drafts` and `io_health`.

L4 is `io_drafts` with its outcome and reason. No separate table yet.

---

## The agent

Server-side in the app — route handlers and one cron — using `ai` and
`@openrouter/ai-sdk-provider` as `chat-server` does. THINK and ROUTE only.
No HEAR: there is no talk to hear.

| Trigger                                             | Runs                                         | Drafts go to        |
| --------------------------------------------------- | -------------------------------------------- | ------------------- |
| A direction version is confirmed                    | move 1 — gap list, then project drafts       | Shapers             |
| A project goes live with no holder                  | move 1 — DRI suggestion                      | Shapers, the named  |
| A project gets a holder                             | move 2 — coverage list, then ticket drafts   | that holder         |
| A ticket is accepted                                | move 2 — same, one level down                | that holder         |
| An item is marked done                              | move 3 — done bubble; rest-of-brief drafts   | parent's holder     |
| A project passes 80 % of its run (cron, daily check) | move 3 — brief and recommendation           | Shapers             |
| A project closes                                    | move 3 — objectives redraw, if the line moved | Shapers            |
| Monday (cron)                                       | move 1 weekly gap scan                       | Shapers             |
| Friday (cron)                                       | move 4 — health for every live project; tally | project page; Shapers |

Every draft passes the **deterministic judge** before insert: schema valid,
every receipt resolves, `needs` matches depth, `due_at` inside the parent's
or the objective's, `gap_key` has no open sibling, a declined `gap_key` is
not reused unless the direction version or the subtree changed. A draft
that fails is logged and not inserted. That is the whole safety model in
Phase 0: a person taps, and the agent cannot do the wrong kind of thing.

Prompts are one file per move, versioned in the repo, with the context
recipe from the evaluation plan. Model pinned. The same prompt files are
what the offline harness runs — Phase 0 and the harness share them from
day one.

### Tally

A Shapers-only card on Overview, refreshed Friday: per move, drafts opened,
agreed, amended, declined by reason; health agreement; open drafts older
than five days. The online columns of the evaluation plan's table, on this
org, weekly.

---

## Technical architecture

How to build it inside the monorepo, using only patterns that already
exist there. Nothing new in kind: Drizzle tables in `storage-postgres`,
queries and mutations in `core` with `{ db }` injected, a Next.js 15 app
with server actions, Vercel cron with `CRON_SECRET`, Privy on the server,
and the AI SDK through OpenRouter as `chat-server` does.

### Where the code lives

```
packages/storage-postgres/src/schema/
  intelligent-org.ts               the six io_* tables, exported from schema/index.ts

packages/core/src/intelligent-org/
  types.ts                         WorkState, DraftKind, DeclineReason, Move, receipts
  server/
    index.ts                       re-exported from core/src/server.ts
    queries.ts                     overview, tree, item page, my-work, tally — all { db }
    mutations.ts                   the role-checked writes; one ledger helper
    ledger.ts                      writeLedger({ db }, row) — the single write path
    authorize.ts                   requireMember / requireShaper / requireHolder
  agent/                           server-only
    index.ts                       runMove(move, trigger, { db, model })
    context.ts                     one context recipe per move
    prompts/
      1-direction-to-projects.md   versioned; frontmatter carries version + model
      2-parent-to-children.md
      3-completion.md
      4-health.md
    schemas.ts                     zod output schemas, one per move
    judge.ts                       deterministic checks before insert
    health-formula.ts              the published score; weights in health-weights.json
    route.ts                       needs: resolution (shaper | person id)
  client/
    index.ts                       SWR hooks (*.web2.rsc.ts) over the app's route handlers

apps/intelligent-org/
  src/app/[lang]/
    layout.tsx                     AuthProvider (Privy) → ThemeProvider → shell
    page.tsx                       Overview
    projects/page.tsx
    projects/[id]/page.tsx         project or ticket page
    my-work/page.tsx
  src/app/api/
    cron/monday-gaps/route.ts
    cron/friday-health/route.ts
    me/route.ts                    who am I in this space
  src/actions/                     'use server' — thin: auth → core mutation → run hooks
  src/components/                  moved from apps/org-preview, made data-driven
  vercel.json                      crons
```

`epics` is skipped in Phase 0: with one app and one org, the feature
components live in the app. They move to `packages/epics` when the pilot
space arrives. Direction: `apps/intelligent-org → core → storage-postgres`,
never the reverse.

### Schema

All six tables share `spaceId` (integer, references `spaces.id`),
`commonDateFields`, and `serial` ids. Types via `InferSelectModel`.

```ts
// packages/storage-postgres/src/schema/intelligent-org.ts (excerpt)
export const ioWorkItems = pgTable(
  'io_work_items',
  {
    id: serial('id').primaryKey(),
    spaceId: integer('space_id').notNull().references(() => spaces.id),
    parentId: integer('parent_id').references((): AnyPgColumn => ioWorkItems.id),
    rootId: integer('root_id'),                      // denormalised; set on insert
    depth: integer('depth').notNull().default(0),
    title: text('title').notNull(),
    brief: text('brief').notNull(),
    driPersonId: integer('dri_person_id').references(() => people.id),
    offeredTo: integer('offered_to').references(() => people.id),
    offeredBy: text('offered_by'),                   // person id as text, or 'agent'
    state: text('state').$type<WorkState>().notNull().default('draft'),
    // draft → offered → accepted → done | declined
    dueAt: timestamp('due_at').notNull(),
    approvedAt: timestamp('approved_at'),
    objectiveRef: text('objective_ref'),             // 'objectives@3#2' = version 3, line 2
    ...commonDateFields,
  },
  (t) => [index('io_work_items_space_root').on(t.spaceId, t.rootId)],
);
```

`io_direction` keeps `body` (markdown) and `lines` (`jsonb` — numbered
`{ n, text, date? }[]`, filled for objectives and strategy only). A
version is a row; the latest row per `kind` with `confirmedAt` set is the
live one. A row with `proposedBy` and no `confirmedAt` is the pending
draft the other Shaper sees.

`io_drafts.payload` is the zod-validated output of the move, as-is.
`io_drafts.receipts` is `{ kind: 'direction' | 'work_item' | 'ledger',
ref: string }[]`; the judge resolves each before insert.

`io_ledger.evidence` is `jsonb` — the pointer that makes the row a
receipt: `{ draftId? , workItemId?, directionRef?, actorNote? }`.

Migration by the usual path: `pnpm --filter @hypha-platform/storage-postgres
run generate` then `migrate`. One migration for all six tables.

### Mutations — where the rules live

Every write goes through `packages/core/src/intelligent-org/server/mutations.ts`.
The UI has no rule logic; a server action calls one of these and returns.

| Mutation                                   | Who may call                   | Writes                                                                                  |
| ------------------------------------------ | ------------------------------ | --------------------------------------------------------------------------------------- |
| `proposeDirection({ kind, body, lines })`  | Shaper                         | `io_direction` row, unconfirmed; ledger `direction.proposed`                            |
| `confirmDirection({ id })`                 | a Shaper other than proposer   | sets `confirmedAt`; ledger `direction.confirmed`; **hook → move 1**                     |
| `promoteRoot({ draftId, edits? })`         | Shaper                         | `io_work_items` root (`accepted` if a holder accepts later, else `offered`/open); `io_drafts` → accepted / amended; ledger; **hook → move 1 DRI suggestion** if no holder |
| `promoteChild({ draftId, edits? })`        | holder of the parent           | child in `offered` to the named person; drafts outcome; ledger                          |
| `offer({ itemId, toPersonId })`            | holder of the parent, or Shaper at root | sets `offeredTo`, `offeredBy`; ledger `work.offered`                           |
| `accept({ itemId })`                       | the `offeredTo` person only    | `driPersonId`, `state = accepted`, `approvedAt`; ledger; **hook → move 2**              |
| `decline({ itemId })`                      | the `offeredTo` person only    | clears offer; ledger `work.declined`                                                    |
| `markDone({ itemId })`                     | `driPersonId` only; refuses if any child is open | `state = done`; ledger `work.done`; **hook → move 3**                 |
| `decideDraft({ draftId, outcome, reason?, edits? })` | the `needs` target   | `io_drafts` outcome; ledger `draft.decided`                                             |
| `setBlindBand({ itemId, week, band })`     | Shaper                         | `io_health.shaperBands[personId]`                                                       |

`writeLedger` is the only function that inserts into `io_ledger`, and
every mutation calls it inside the same transaction. That is the design's
"completeness enforced, not hoped for" in one file.

The agent has **no mutation**. It has two inserts — `insertDraft` and
`upsertHealth` — and both are called only from `agent/index.ts`. A grep
for `driPersonId` under `agent/` must return nothing; add that as a lint
rule or a unit test.

### Server actions and auth

Thin `'use server'` functions in `apps/intelligent-org/src/actions/`. Each
one:

1. Reads the Privy access token from the request cookie / header and
   verifies it with `PrivyClient` from `@privy-io/node`, exactly as
   `apps/web/src/app/api/matrix/token/route.ts` does.
2. Looks up `people.sub = privyUserId`, then `io_memberships` for the
   space. No row → 403. Allowlist is the membership table; adding a member
   is an insert.
3. Calls the core mutation with `{ db }` and the resolved `personId`.
4. Runs the hook the mutation returns (see below), `revalidatePath` on the
   affected doors, returns.

Reads are server components calling `queries.ts` directly with `{ db }`;
My Work also has an SWR hook for the card list so a tap updates without a
full reload.

### The agent pipeline

```
trigger  ──▶  context.ts  ──▶  model (generateObject, zod schema)  ──▶  judge.ts  ──▶  route.ts  ──▶  insertDraft
 (hook |        one recipe        pinned OPENROUTER_AGENT_MODEL         hard gates     needs:          io_drafts
  cron)         per move          temperature 0.2 · maxTokens per move  → log + drop   shaper|person   + ledger row
```

- **Provider.** `createOpenRouter({ apiKey, compatibility: 'strict', headers })`
  — copy `buildOpenRouterAppHeaders` from `chat-server`; the default export
  without attribution headers 401s intermittently. Model id from
  `OPENROUTER_AGENT_MODEL`; never `openrouter/auto`.
- **Call shape.** `generateObject` with the move's zod schema. Free text is
  not a draft; a parse failure is logged and produces nothing.
- **Two-step inside THINK.** For moves 1 and 2 the schema has two top-level
  fields: `gaps` (or `coverage`) — every direction line / brief phrase with
  `served | partly | not` and the ids that serve it — then `drafts`. The
  judge rejects a draft whose target line is marked `served`. The gap list
  is stored in `payload` as the receipt.
- **Move 3 redraw** returns operations, not text:
  `{ op: 'strike' | 'move' | 'add', n?, text?, date?, why, source? }[]`.
  `promoteDirection` re-renders `lines` from the operations. A rewrite of
  an untouched line is impossible by construction.
- **Move 4** calls `health-formula.ts` first (pure function over ledger
  aggregates → `pct`, `band`, `factors[]` with rows), then the model writes
  `sentences: { text, rows: string[] }[]`. A sentence with no rows is
  dropped. Numbers in the text must appear in `factors`; the judge scans.
- **Prompts** are markdown files with frontmatter (`move`, `version`,
  `model`, `changed`), loaded at runtime. A version bump is a commit. The
  same files are what the offline harness in `chat-server` runs.
- **Judge** (`judge.ts`) is pure and synchronous given the context:
  schema → receipts resolve → `needs` matches depth → dates inside parent
  / objective → no open `io_drafts` with the same `gapKey` → declined
  `gapKey` not reused unless `directionVersion` or subtree `updatedAt`
  changed → no assignee or state in payload. Returns `ok | { reason }`.
  Failures write a ledger row `agent.dropped` with the reason, so the tally
  can count them.
- **Timeouts.** Each trigger runs one move, at most one model call plus
  the judge, under ten seconds. Hooks run after the mutation's transaction
  commits, via `after()` from `next/server`, so a slow model never blocks
  the tap. If `after()` fails, the Monday scan catches the gap.

### Triggers

Two kinds, both rules.

**Hooks.** A mutation returns `{ hooks: Hook[] }`; the server action calls
`runHooks(hooks)` inside `after()`. Hooks are data:

```ts
type Hook =
  | { move: 1; kind: 'direction-confirmed'; directionId: number }
  | { move: 1; kind: 'root-without-holder'; itemId: number }
  | { move: 2; kind: 'holder-set'; itemId: number }
  | { move: 3; kind: 'item-done'; itemId: number }
  | { move: 3; kind: 'root-closed'; itemId: number };
```

**Cron.** Two routes under `apps/intelligent-org/src/app/api/cron/`, both
`GET`, both guarded by `assertCronAuth` (Bearer `CRON_SECRET`, copied from
`apps/web/src/app/api/cron/_lib`). `maxDuration = 300`.

```json
{
  "crons": [
    { "path": "/api/cron/monday-gaps",   "schedule": "0 7 * * 1" },
    { "path": "/api/cron/friday-health", "schedule": "0 12 * * 5" }
  ]
}
```

`monday-gaps` runs move 1's weekly scan plus the move 3 check for roots
past 80 % of their run. `friday-health` runs move 4 for every live root and
recomputes the tally. Both accept `?dry_run=true`, which runs THINK and the
judge and returns what would be inserted without inserting — the way to
test a prompt change against the live org before the real run.

### Keeping the agent honest — tests

- **Unit** (Vitest, `packages/core`): every mutation's role check; done
  refuses with open children; ledger row per mutation; judge cases (one
  per gate); health formula monotonicity; redraw operations render.
- **Prompt regression** (`packages/chat-server`, the evaluation plan's
  harness): the four prompt files against River and Energy seeds, recorded
  model responses, targets from that plan. Runs on any change under
  `agent/prompts/`.
- **Architecture** (a test, not a convention): no import of `agent/` from
  `client/`; no `driPersonId` / `state` write under `agent/`; no
  `storage-postgres` import from the app except through `core`.

### Deploy and environment

- Own Vercel project, `apps/intelligent-org`, framework Next.js, build from
  the monorepo root with Turbo (`turbo run build --filter=intelligent-org`).
- Env: `DATABASE_URL` (the same Neon database), `NEXT_PUBLIC_PRIVY_APP_ID`,
  `PRIVY_APP_SECRET`, `OPENROUTER_API_KEY`, `OPENROUTER_AGENT_MODEL`,
  `OPENROUTER_JUDGE_MODEL` (used only by the harness), `CRON_SECRET`,
  `IO_SPACE_ID` (the one seeded space).
- Seed: one script, `pnpm --filter intelligent-org seed`, inserts the
  space row if missing, the two Shapers and members from a JSON allowlist,
  and nothing else. Direction is written in the app, not seeded — the first
  confirm has to be a real one.
- Flags: `IO_MOVE_1_ENABLED` … `IO_MOVE_4_ENABLED`, read at trigger time.
  Off means THINK runs, the judge runs, the draft is inserted with
  `state = 'shadow'` and shown to nobody. This is the kill switch and the
  shadow mode from the evaluation plan's rollout gates in one setting.

### Out of scope, deliberately

No Matrix, no `chat-server` tools, no notifications (My Work is the
inbox; a daily email digest can come after week two if people miss cards),
no i18n beyond `en` (the shell keeps `[lang]` so it costs nothing later),
no `epics` package, no on-chain anything.

---

## The Friday ritual

Fifteen minutes, both Shapers, in the app:

1. Before opening any project page, each Shaper sets their **blind band**
   for every live project on the tally card — struggling / wobbly /
   healthy. Then the agent's bands unlock. Agreement is stored.
2. Every open draft older than five days gets a tap. Nothing sits.
3. Read the tally. Three numbers matter: precision, _already covered_,
   _not what the line meant_.
4. If a prompt changes, it changes here, with the tally as the reason and
   a version bump.

---

## First things to build, in order

Each is a project the agent should draft from the direction above once the
first slice is live. If it does not, that is finding number one.

1. **Store and mutations.** Three days. The six tables in
   `storage-postgres`, the mutations in `core/intelligent-org` with role
   checks, `writeLedger` as the one write path, unit tests per rule. Seed
   the space, two Shapers, the members. No UI yet.
2. **Overview with the direction form.** Two days. Four cards, write a
   version, the other Shaper confirms from My Work. This is the first real
   confirm, and the first trigger.
3. **Move 1 with the judge.** Three days. Gap list, project drafts, the
   card on Shapers' My Work with Agree / Edit / Decline and reasons. The
   drafts it opens on the first confirmed direction are the Phase 0
   backlog. Deciding them is the first L4 data.
4. **Projects door and the item page.** Three days. Tree, page, children,
   trail. DRI suggestion card and the offer / accept path — so the drafted
   projects can be held.
5. **Move 4 — health.** Two days. Formula over the ledger, paragraph with
   rows, card on the project page. Blind bands on the tally card the same
   week. The first Friday ritual.
6. **Move 2.** Three days. Coverage list, ticket drafts to the holder,
   offer to a person, one level down per trigger. From here the tree grows
   from the agent's drafts.
7. **Move 3.** Four days. Done bubble; the 80 % brief and recommendation;
   the objectives redraw as line operations. Needs a few closes to have
   happened — it lands around week four, when the first projects end.
8. **Two crons and the tally.** One day. Monday scan, Friday health and
   tally.
9. **Freeze the record.** After four weeks of the full loop: export
   `io_*` for this space into the evaluation plan's fixture format. The
   fourth seed — the real one.

Steps 1–5 are the first two weeks. From step 5 every Friday produces a
tally row, and the org is being managed by the thing it is building.

---

## What we measure

The online columns of the evaluation plan's target table, on this org,
weekly. The ones that matter most in Phase 0:

| Question                                                        | Measure                                                 | Good        |
| --------------------------------------------------------------- | ------------------------------------------------------- | ----------- |
| Are the project drafts the ones we would have written?          | move 1 agreed + amended / opened                        | ≥ 0.55      |
| Do the ticket drafts get offered as written or with one edit?   | move 2 agreed + amended / opened                        | ≥ 0.60      |
| Does it ever draft something already covered?                   | declined _already covered_                              | ≤ 1 / week  |
| Does it nag?                                                    | a declined `gap_key` redrafted with nothing changed     | 0           |
| Does it ever invent a receipt?                                  | receipts that do not resolve (should never reach a card) | 0          |
| Does the recommendation fit when something closes?              | move 3 agreed / opened                                  | ≥ 0.70      |
| Does the health read match ours?                                | blind band agreement, both Shapers                      | ≥ 0.80      |
| Is it quiet enough to be read?                                  | open drafts older than five days on Friday              | 0           |

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

Then the record becomes the fourth fixture, the prompts move into
`chat-server` as the first versioned agent prompts, and the
[Design](../architecture/intelligent-org-design.md) build order continues
from step 4 — L1 ingestion — with a known-good agent to plug the HEAR pass
into. The Decisions door, Profile, Personal Assistant, money and join come
back in that order, each because a move now needs it, not before.

If after six weeks a move is still below target, that move goes back to
the offline harness with this org's declines as its new negative cases and
its cards are hidden behind a flag. The other moves keep running. We do
not widen a door for a move that is not good.

---

## What Phase 0 will not tell us

Said plainly, so nobody reads too much into a good result:

- **Scale.** One org, a dozen items. Dedup and nag rules are easy at this
  size. The weekly scan on a space with sixty projects is a different test.
- **Talk.** There is no chat, so no talk-derived drafts. Every move here is
  gap-derived. That is the harder path for the model and the one we can
  test without L1 — but "Lea, can you take covers?" is untested until
  Matrix ingestion lands.
- **Strangers.** We wrote the direction and we know the code. A pilot space
  that did neither is objective 4, not Phase 0.
- **Self-reference.** The agent drafting "build the deterministic judge"
  from a strategy line that says "build the ruler first" is partly reading
  our own words back to us. Watch for drafts that are the strategy
  rephrased rather than a project that serves an objective. _Not what the
  line meant_ exists for exactly this.
- **Three doors are not five.** Folding Shaper decisions into My Work
  works for two Shapers and a dozen drafts. Whether Decisions needs its own
  door is a question for the pilot space, not for us.

---

## Related

- [The Intelligent Organization — AI Evaluation Plan](./intelligent-org-ai-evaluation.md) — the targets, the judges, and the harness this phase feeds
- [The Intelligent Organization — Design](../architecture/intelligent-org-design.md) — the full tables and the build order Phase 0 starts
- [The Intelligent Organization — What it is](../product/intelligent-org-features.md) — the rules that stay fixed here
- [The Intelligent Organization — User Journeys](../product/intelligent-org-journeys.md) — 1.1, 2.4, 2.5, 2.8, 4.4, 4.10, 4.11
- Clickable preview: [hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app) — the components Hypha Intelligent Org grows from
