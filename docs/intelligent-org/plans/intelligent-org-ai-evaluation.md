---
title: 'The Intelligent Organization — AI Evaluation Plan'
date: 2026-09-14
status: current
tags: [plan, intelligent-org, ai, evaluation, testing, buzz]
parent: docs/intelligent-org/README.md
---

# The Intelligent Organization — AI Evaluation Plan

How we make the four AI moves of the intelligent org good, and how we prove
they are good before a Shaper sees them. Companion to
[What it is](../product/intelligent-org-features.md),
[User Journeys](../product/intelligent-org-journeys.md), and
[Design](../architecture/intelligent-org-design.md). This document adds one
thing those do not have: **a pass / fail bar for the model's output, and the
harness that measures it.**

The four moves:

| #   | Move                                                | Trigger (a rule, never the model)                            | Goes to           |
| --- | --------------------------------------------------- | ------------------------------------------------------------ | ----------------- |
| 1   | **Direction → projects**                            | L3 confirm hook; weekly gap scan                             | Shapers           |
| 2   | **Project → tickets, ticket → subtickets**          | root promoted; ticket accepted; weekly gap scan              | that item's DRI   |
| 3   | **Completion → what next**                          | last child done; `due_at − 20 %` of run; project closed      | parent DRI / Shapers |
| 4   | **Project health — the agent's read**               | Friday; any ledger change on a live root (debounced, Org agent § 6.2) | anyone reading    |

Everything the agent writes is a **draft**. A human tap promotes it. That is
not a limitation of the evaluation — it is what we measure: _did a person
worth interrupting choose to act on this?_

---

## What "very good" means

One sentence per move, then the numbers.

1. A Shaper opens a project draft and thinks _yes, that is the gap_ — not
   _we already do that_, not _that is not what the objective said_.
2. A DRI opens a ticket draft and can offer it as written, or with one edit.
   Never a piece that is already covered, never one the description did not
   ask for, never one whose turn has not come — the draft is the piece that
   can start now, it names what the piece needs, and it names the person
   who has that or says plainly that nobody here does.
3. When something ends, the next move is already on the table — a follow-up
   that fits, or a clear "nothing more here" — and the objective list is
   redrawn only where the close actually moved it.
4. The health read matches what a careful Shaper would say after reading
   the ledger, and every sentence points at a row that backs it.

Two rules bound all four. **No hallucinated receipt** — every id the agent
cites resolves to an L1 event, an L2 row, or an L3 line. **No authority
slip** — a draft always carries the `needs:` of the one role that can promote
it. Both are hard gates: one failure fails the run.

### The bar is expertise, not plausibility

The drafts have to read like the work of someone who has run this kind of
organisation before — a community hall, a co-op, an energy pilot — not like
a template with the nouns swapped. That is the standard every case in this
plan is written to, and it has three consequences for how the suites are
built:

- **Gold is domain-real.** A case's expected drafts are what an
  experienced operator in that domain would actually create: the licence
  before the opening night, the site survey before the installer is
  booked, the person with the certification for the electrical piece. Case
  authors write the domain reasoning down in the case file (`why_gold`),
  so a reviewer can disagree with the operator, not just the model.
- **Generic fails.** A title or brief that could sit under any project
  ("Make a plan", "Do research", "Kick-off") fails the model judge's
  _specific_ question regardless of the rest. The suites carry a
  vacuous-title list as a deterministic pre-check so the cheap judge
  catches the common form.
- **The whole surface is covered.** Every job in Org agent § 1 that
  produces model output — J1, J1b, J2, J3b, J3c, J3d, J4, J6, J7, J8b, J9,
  J10, J11 — has a suite or a case set with positives, negatives, and
  adversarials, on every seed org. The four move suites below are the
  core; § 5 lists the rest. A job with no cases does not ship with cards
  on.

### Targets

| Metric                                                                                       | Move    | Offline (golden set) | Online (L4, per community, rolling 4 weeks) |
| -------------------------------------------------------------------------------------------- | ------- | -------------------- | --------------------------------------- |
| Receipt validity — cited ids exist and say what the draft says                               | all     | 100 %                | 100 %                                   |
| Authority routing — `needs:` is the right role for the item's depth                          | all     | 100 %                | 100 %                                   |
| Precision — drafts a judge marks "worth this person's minute"                                | 1, 2, 3 | ≥ 0.80               | accept + amend ≥ 0.55                   |
| Recall — gold gaps the agent found                                                           | 1, 2    | ≥ 0.70               | —                                       |
| Duplicate rate — two open drafts for one `gap` key                                           | 1, 2    | 0                    | 0                                       |
| Sequence fit — drafted pieces are the ones that can start now; held pieces match gold        | 2       | ≥ 0.90               | reorder-amend rate ≤ 0.15               |
| Who-is-needed fit — `requires` matches gold; holder has it, or `unfilled` names the gap      | 1, 2    | ≥ 0.85               | holder-change amend rate ≤ 0.20         |
| Nag rate — a dismissed key raised again with nothing changed                                 | 1, 2, 3 | 0                    | 0                                       |
| Silence rate — candidates that correctly produced nothing                                    | 1, 2    | ≥ 0.90 on negatives  | —                                       |
| Recommendation fit — follow-up vs "no further work" matches gold                             | 3       | ≥ 0.85               | reject rate ≤ 0.25                      |
| Objectives redraw — only lines the close touched are changed                                 | 3       | 100 %                | amend rate ≤ 0.30                       |
| Health agreement — same band (struggling / wobbly / healthy) as the human panel              | 4       | ≥ 0.85               | ≥ 0.80                                  |
| Health grounding — each sentence maps to ≥ 1 ledger row                                      | 4       | 100 %                | 100 %                                   |
| Health stability — same ledger, same band across 5 runs                                      | 4       | 100 %                | —                                       |
| Judge calibration — LLM judge vs human labels (Cohen's κ)                                    | harness | ≥ 0.70               | re-checked monthly                      |

Offline numbers gate a prompt or model change. Online numbers gate widening
to another community. A drop of more than five points on any row is a
regression and blocks the release that caused it.

---

## The harness

One harness, four suites. It lives in `crates/buzz-org-agent/tests/eval/`
next to the agent code, runs as ordinary Rust tests (`cargo test -p
buzz-org-agent`), and is the same for a laptop and CI.

### Replay, not chat

An evaluation case is a **scripted org**: a seed of L3 (the four direction
artifacts and every member's `39105` profile), a work tree, an L2 ledger, an L1 channel window, and an L4 history
— followed by one **trigger** (a confirm, an accept, a done, a date). The
harness loads the seed into an in-memory store shaped like the relay's
state events and `io_*` projections, fires the trigger through the real
THINK → ROUTE code (and HEAR, once it exists), and captures what comes out:
zero or more structured drafts (`kind:50100` payloads), or one health read
(`kind:50101`). The relay is not in the loop; the judge's receipt check
stands in for the relay's.

```
fixtures/orgs/<org>/seed.json          L3 + profiles + tree + ledger + room + L4
fixtures/orgs/<org>/cases/<case>.json  trigger + gold
```

Gold is written by a human and reviewed by a second one. It says: which
drafts should exist (by `gap` key and one-line intent), which must **not**
exist, the correct `needs:`, and for health, the band and the facts the text
must cover.

### Three judges

Every draft passes through three checks, in this order. Cheap first.

1. **Deterministic.** Schema is valid. Every cited id resolves. `needs`
   matches depth. `due_at` is before the objective's rough date. `gap` key
   has no open sibling. No money field on a work item. No `dri` or `state`
   in any payload — the agent has no command kind. A declined key was not
   reused unless the L3 version or subtree changed. Any failure here is a
   hard fail — no model call needed to know it is wrong. This is the same
   `judge.rs` the live agent runs.
2. **Model judge.** A second model (never the generator's) scores the draft
   against a rubric with the seed as context: _does this serve the cited
   line? is it already covered? would the named person recognise it? is the
   size right for one holder? is it specific to this org, or could it sit
   under any project? is this the right next thing given what is done and
   what is not yet known? is the named person the one this really needs —
   and if nobody fits, did it say so?_ Scores are 0–2 per question; a draft
   passes at ≥ 11 of 14. The judge prompt is versioned with the cases.
3. **Human panel.** Each week, Shapers of pilot communities rate a sample
   of twenty live drafts on the same rubric. Their labels calibrate the
   model judge (κ ≥ 0.70 or the judge is retuned) and are the online truth.

### Negatives matter more than positives

Half of every suite is cases where the right answer is **nothing**. An
agent that proposes on every objective is worse than none. Each move below
lists its negatives; a suite without them does not count.

### Pinned, recorded, cheap

The generator model and the judge model are pinned per release. Model
responses are recorded on first run and replayed on later runs, so the
suite is deterministic in CI and costs nothing until a prompt or model
changes. `EVAL_LIVE=1` re-records. A model swap is a pull request that
re-records and shows the metric diff.

### L4 is the production suite

Every online decision — accept, amend, decline — is already an L4 row
(`io_drafts` outcome, `kind:39104`). The online metrics above are queries
on that table per community, per move, per week. No new instrumentation;
the protocol already writes the signal. A tally card on Overview for
Shapers (their own community) and `buzz org tally` for the internal view.

---

## 1. Direction → projects

**Goal.** When the Shapers confirm objectives or strategy, each objective with
no live root and each strategy line nothing acts on gets **one** project
draft — title, description, the objective it serves, a suggested DRI, an
exact end date before the objective's rough date. If everything is served,
nothing happens.

### How we make it good

- **Context recipe.** The full L3 (always). The live roots with their
  `objective_ref`, one line each. The last two closed roots. L4 rows for
  past project suggestions in this community: what was accepted, amended,
  dismissed. Nothing from L1 — this move is about the gap, not the talk.
- **Structured output.** THINK returns a typed array, possibly empty. Each
  item: `objective_ref` or `strategy_line_ref`, `title`, `brief` (≤ 60
  words), `suggested_dri` (a member, with the L2 rows that make them the
  suggestion), `due_at`, `why` (one line, citing the gap). Free text is not
  a draft.
- **Two-step inside THINK.** First a **gap list**: per line, _served /
  partly / not_, with the root ids that serve it. Then drafts only for _not_
  and _partly_. The gap list is stored as the receipt. It is also what the
  judge grades first — a wrong gap list is the usual cause of a wrong draft.
- **Strategy is a constraint, not a to-do.** A strategy line that says what
  the org will _not_ do ("no brand money") never yields a project. The
  prompt names this. The suite has a case for it.
- **Size.** One project per line. If the model wants two, it must pick one
  and say what it left out in `why`. Splitting is the DRI's job (move 2).
- **First step first.** When an objective's sensible first step is a
  validation — a pilot before a rollout, a survey before a build, a permit
  before a programme — the draft is that step, sized as one project, and
  `why` names what waits behind it. The follow-up arrives through move 3
  when the first one reviews, informed by what it found.
- **DRI suggestion is evidence-based.** The context carries a **candidate
  list** (≤ 10): members whose `39105` skills or about are near the brief,
  and members with L2 rows under the nearest domain — each with skills,
  about, open count against `open_limit`, past items. The suggestion names
  one of them and returns `matched { skills, about, items }` — the
  receipts. Never a name with nothing behind it; a member with no profile
  and no rows is not a candidate; the founder is the fallback when the
  community is new; _none_ is a correct answer and the prompt says so.

### How we test it

Suite `direction-to-projects`. Seeds: River Commons, Hypha Energy, and a
**cold** org — a founder, four fresh artifacts, no work tree.

Positive cases (a draft must appear):

- Objectives v1 confirmed on the cold org → one root per objective, all
  `needs: shaper`, all dated before the objective's date.
- Objectives redrawn — one new line added (Energy: "a second island by
  December") → exactly one new draft, with `gap` key on the new line.
- Strategy line added that names an action ("publish the Ameland report
  before any marketing") → one draft for it.
- Weekly scan: an objective whose date is eight weeks out with nothing under
  it → one draft.
- Holder from profile: River's "grant for the hall roof" line, Rafi's
  profile says _grant writing_, nobody has held a grant item → draft
  suggests Rafi with `matched.skills = ["grant-writing"]` and no items.
- Holder from history over profile: Lea's profile says nothing about
  markets but she held the stall last year → the stall follow-up suggests
  Lea with `matched.items`, `skills` empty. Profile and history are both
  evidence; neither is required, one is.
- Profile changed: Rafi adds _grant writing_ a week after the roof project
  went live with no holder → the `profile-changed` trigger yields one DRI
  suggestion for that root, and nothing for roots that already have one.
- First step first: Energy's "a second island by December" with no site
  chosen → one project, the site survey / selection, `why` naming the
  installation that waits on it. A draft for the installation itself fails
  _right next thing_.
- Nobody here fits: an objective line needs a capability no profile and no
  history shows (River: "get the hall's electrics certified") → the draft
  has `suggested_dri: null` and `why` says what the community lacks. Naming
  anyone fails `unmatched skill`.

Negative cases (nothing must appear):

- Objectives reconfirmed with no text change.
- A holder suggestion for a member at their `open_limit` (Energy: Rowan,
  limit 2, holding 2) → the draft either names someone else or `null`;
  naming Rowan fails the judge.
- A skill the member never wrote (`matched.skills` contains a slug not on
  their `39105`) → judge failure `unmatched skill`. This is the case that
  catches a model inventing fit.
- A new objective already served by a live root whose brief plainly covers
  it (River: "stall every Saturday" while the stall project is live).
- A strategy line that is a constraint ("we do not take brand money").
- A mission or vision change alone — direction, not a to-do.
- A gap the Shapers dismissed last week with the same L3 version.

Adversarial:

- Two objectives that overlap by half → one draft, or two with a stated
  boundary; the judge fails "two drafts for one thing".
- An objective written vaguely ("be more visible") → a draft the judge
  grades for _would a Shaper recognise this as the first step_, or nothing
  with a `why` that says the line is too vague to serve. Both pass; a
  confident wrong project fails.
- Locale: the same seed in `pt` and `es` → same gap list, drafts in the
  org's language.

Pass bar: the target table. Recall is checked on gap lists, not just
drafts — the model must find the gap even when it decides not to draft.

---

## 2. Project → tickets, ticket → subtickets

**Goal.** When a root goes live, its brief is read against its (empty)
children: each piece the brief names that no ticket covers becomes a ticket
draft for the project's DRI. When a ticket is accepted, the same read runs
on it: pieces it names become subticket drafts for its holder. Same rule at
every depth. The draft names a suggested holder and an exact date inside the
parent's date.

### How we make it good

- **Context recipe.** The parent item (title, brief, `due_at`, DRI). Its
  live children, one line each. Its siblings' titles (to avoid drafting a
  piece a sibling already holds). The L3 line the root serves. L4 rows for
  child drafts under this DRI: which they confirmed, amended, discarded.
  The holder candidate list as in move 1: members whose `39105` skills or
  about are near the brief, and members with L2 `done` rows in this
  project's domain, each with open count and `open_limit`.
- **Structured output.** Typed array. Each item: `parent_id`, `title`,
  `brief` (≤ 40 words), `suggested_holder` (member + `matched { skills,
  about, items }`, or none),
  `due_at` (≤ parent's), `covers` — the phrase in the parent brief this
  piece answers to. `covers` is the receipt and the dedupe key.
- **Coverage list first.** THINK produces the list of pieces the brief
  names, marks each _covered by child X / not covered_, then drafts only the
  uncovered. Stored, graded.
- **Depth discipline.** Suggest at most one level down per trigger. A ticket
  draft never arrives with its own children pre-drafted; those come when it
  is accepted. Median depth on a real community is watched (design risk 6); if
  the agent is pushing it past three, the pieces are too small and the
  prompt's size guidance is wrong.
- **Sequence.** The coverage list is an ordered plan, not a bag of pieces:
  each piece has `order`, `after`, and is `held` when a predecessor is
  neither live nor done (Org agent § 8.6). A **gate** — a piece whose
  outcome decides what the later pieces are (a permit, a pilot, a
  supplier's yes, a measurement) — is drafted first, alone or with the
  pieces independent of it; the rest waits. When the gate's item goes done,
  the "child done unblocks a held piece" trigger re-runs the move with the
  outcome in context, and the next wave is drafted against what was
  learned. A DRI should never see a ticket for step four while step one is
  an open question. The prompt's line: _draft what can start now; hold what
  depends on an answer nobody has yet._
- **Who is needed, before who is available.** Every draft names `requires`
  — what the piece calls for — read from the brief before the candidate
  list is consulted. The holder suggestion is the candidate whose `matched`
  covers `requires`, or `null` with `unfilled` naming the requirement no
  member meets. The agent should know exactly who a piece needs, and say
  so even when the answer is "nobody here yet".
- **Never under something the person does not hold.** ROUTE checks; the
  harness checks that ROUTE checked.
- **Talk-derived drafts share the suite.** "Jun, could you print the rota?"
  in a room is the same output shape under the speaker's ticket. One case
  set covers both origins; `origin: talk | gap` is a field, not a different
  code path.

### How we test it

Suite `projects-to-tickets`. Seeds: River (stall, weekday hall, growers),
Energy (EMS, Iberia, islands, playbook — the six-level Andalusia tree is
the depth fixture).

Positive:

- Weekday hall goes live with an empty tree; brief names licence, insurance,
  a rota → three drafts to the project DRI, each with `covers` pointing at
  the phrase.
- You accept Saturday setup; brief says "tables, keys, signage" → keys
  drafted under setup (the preview's case), others only if no sibling holds
  them.
- Lea accepts covers; brief names a rota → rota draft to Lea, suggested
  holder Jun with his L2 rows and the _printing_ line on his profile.
- Brief names "translate the flyer to Spanish"; Priya's profile says
  _Spanish_, she has held nothing → suggested holder Priya, `matched.skills
  = ["spanish"]`, no items.
- Last child under a ticket goes done, brief not yet met → one draft for
  the remaining piece.
- Talk: "Jun, could you print the Saturday cover rota?" from Lea → same
  draft, `origin: talk`, receipt is the message.
- Gate first: weekday hall's brief names a licence, a rota, insurance, and
  the opening night → `coverage` orders licence first with `gate: true`,
  insurance independent (drafted now), rota and opening night `held: after
  licence`; exactly two drafts. Gold names the order and its `why_gold`
  (no licence, no opening — and the licence conditions may change the
  rota).
- Next wave on the gate: the licence ticket goes done with a progress note
  saying "granted, weekdays only, until 22:00" → J2 re-runs on the hall;
  the rota draft appears with `after: [licence-uuid]` and a brief that
  respects 22:00; the opening-night draft appears; nothing already live is
  re-drafted.
- Gate outcome changes the plan: the same licence comes back "refused for
  weekday evenings" → the next wave is not the old held pieces; the draft
  is a re-scoped piece (daytime rota) or a `done`/review nudge to the DRI,
  and the judge fails a rota draft that ignores the refusal.
- Energy Iberia: "pick the pilot site, sign the landowner, order the
  inverters, install, commission" → site is the gate; landowner `held:
  after site`; inverters `held: after landowner` (the quantity depends on
  the site); one draft now. A six-piece batch fails _sequence fit_.
- Who is needed: the "get the electrics certified" piece → `requires:
  ["electrical-certification"]`; no River profile has it → `suggested_holder:
  null`, `unfilled` names the certification. Energy has Tomas with it →
  the same piece under Energy suggests Tomas with `matched.skills`.
- Requires over availability: two candidates under `open_limit`, one with
  the required skill and two open items, one with no skill and none → the
  skilled one is suggested; the judge fails the free-but-unskilled name.

Negative:

- Brief names pieces that all have live children.
- Brief is one sentence with no named pieces ("keep the EMS running").
- The speaker of a talk-derived request does not hold the item they are
  splitting → nothing, plus a nudge to the actual holder.
- A piece drafted and discarded last week, parent unchanged.
- A ticket accepted whose brief is met by the ticket itself (no split
  needed).
- A child goes done that unblocks nothing (an independent piece) → no
  re-run output; the held pieces stay held.
- Pieces with no real dependency (three posters for three noticeboards) →
  no `after`, no `held`; inventing an order to look thorough fails
  _sequence fit_ as much as missing one does.

Adversarial:

- Brief names a piece a **sibling** already holds under a different name →
  no draft; judge checks the coverage list caught it.
- Brief names eight pieces → at most one level, the model groups or picks;
  eight leaf drafts at once fails the size rubric.
- Two DRIs' tickets each mention the same shared piece → one draft, under
  the one whose brief names it first; never two.
- Circular order in the model's plan (A after B, B after A) → gate 15
  `sequence` fails the batch; nothing is published.
- A brief that lists steps in the wrong order ("install, then survey") →
  the coverage list orders them correctly and `why` says why; following
  the brief's order fails _right next thing_.
- `after` pointing at a sibling that is `open` with no holder → allowed
  (it is live), but the judge's _right next thing_ asks whether drafting
  the dependant now is useful; gold decides per case.

---

## 3. Completion → what next

**Goal.** Three moments, one question — _what follows?_

- **A ticket goes done.** Done bubbles up: if it was the last open child,
  the parent's done draft is offered to the parent's holder. If the parent's
  brief is not yet met, move 2 fires for the rest. Nothing else.
- **A project enters its last fifth.** A review brief from L2 + L4 and the
  L3 line it serves, plus **one** recommendation: a follow-up root draft, or
  "no further work in this domain". Routed to the Shapers ahead of the date.
- **A project closes.** If the close met the objective, or shows it will not
  be met, an **objectives redraw** draft: that line struck or moved, the
  rest untouched, one new line only if the brief or the room proposed it.

### How we make it good

- **Brief is assembled, not written.** The brief's facts come from queries:
  items held / done / not done, payments through proposals, offers that
  expired, days since last activity, and whether the objective's `read`
  moved. The model writes the connecting prose and the recommendation; it
  does not choose the facts. Every sentence in the brief is generated from
  a row, so grounding is by construction.
- **Recommendation context.** The objective's current state (met / live /
  dropped). Open work under the same objective. L4 rows for past
  recommendations in this community and what the Shapers did with them. Room
  window from the project's room for the last fifth (what people said is
  next).
- **Two outputs, never a blend.** Either a full follow-up draft (same shape
  as move 1, same `objective_ref`) or an explicit `no_further_work` with
  `why`. "Maybe extend" is not an option the model can pick; extension is a
  Shaper override.
- **Redraw is a diff.** THINK receives the objectives artifact as numbered
  lines and returns operations: `strike(n, why)`, `move(n, new_date, why)`,
  `add(text, date, why, source)`. The artifact is re-rendered from the
  operations. A rewrite of untouched lines is impossible by construction.
- **Nothing on rejection alone.** A rejected proposal or a closed project
  changes direction only through a draft the Shapers confirm.

### How we test it

Suite `completion-to-direction`. Seeds: River stall at day 80 of 90; Energy
islands after the Ameland pilot is done; Energy EMS with a stuck offer.

Positive:

- Rota done under covers; covers has no other children → covers done draft
  to Lea only. Not to Sam.
- Stall enters the last fifth → brief lists Lea's covers, Jun's prices,
  Lea's payment, the expired setup offer if any; recommendation is a
  follow-up (objective "stall every Saturday" is live); follow-up cites the
  same `objective_ref`, dated inside the objective.
- Ameland pilot closes with the objective met → recommendation
  `no_further_work` for the pilot; objectives redraw strikes the Ameland
  line and adds "second island" only because the room said so (receipt).
- A project closes with work open → brief says which items, and the
  recommendation says where they go (follow-up carries them, or they return
  to the Shapers).

Negative:

- A ticket goes done with siblings still open → no parent done draft.
- A project at day 40 of 90 → no brief.
- A project closes whose objective is still live and has other roots
  serving it → no redraw.
- A proposal is rejected with no reason of direction → no strategy draft.

Adversarial:

- The room in the last fifth is full of ideas for three unrelated projects
  → one follow-up at most, in the same domain; the rest is `why` text, not
  drafts.
- The objective was already redrawn last week → no second redraw for the
  same close.
- Brief facts conflict with the room's talk (people say "done", ledger says
  two items open) → the brief follows the ledger and names the gap.

Grading for the brief is factual coverage: every gold fact present, no
sentence without a row. Grading for the recommendation is fit to gold plus
the judge rubric. Grading for the redraw is the operation list: exact match
on `strike` / `move`, judge on `add`.

---

## 4. Project health — the agent's read

**Goal.** One line, one band, one paragraph: how the project is going, from
the ledger, with a receipt behind each claim. The preview shows it as a
red → yellow → green bar with a label ("Shipping", "Wobbly") and three
sentences.

### How we make it good

- **Score is computed; text is written.** The band comes from a small,
  published formula over L2 aggregates — done ratio against elapsed time,
  items overdue, offers past window, days since last activity, children
  with no holder, pieces stalled by their progress notes, objective `read`
  movement (`health-weights@1`, Org agent § 11.4; a payments factor joins
  when money lands, as `health-weights@2`). The weights are in a config
  file, not the prompt. The model receives the
  score, the factors, and the rows behind each factor; it writes the
  paragraph.
- **Every sentence carries its rows.** Output is an array of `{ text,
  rows[] }`. A sentence with no rows is dropped before render. The UI can
  expose the rows on hover — the same "receipts" pattern as Overview.
- **Never a live number.** Treasury and balances are fetched at render,
  never in the health text. The prompt forbids figures the ledger does not
  hold; the deterministic judge scans for currency amounts and dates that
  are not in the factor rows.
- **Stable.** Same ledger, same band. The formula is deterministic; the
  text may vary in wording but not in which factors it names. Temperature
  low; the judge checks factor coverage, not phrasing.
- **Says what would change it.** The last sentence names the one factor
  most pulling the band down, when there is one ("the load test has no
  DRI"). That is the actionable part, and the one Shapers read.

### How we test it

Suite `project-health`. Seeds: six snapshots per org spanning the bands —
a project that just started, one on track, one with a stuck offer, one
with two overdue items, one with no activity for three weeks, one closed
well.

Deterministic:

- Band equals the formula. Every sentence has ≥ 1 row. No figures outside
  the rows. Five runs on the same seed → one band, same factor set.

Monotonicity — one change to the ledger, band must not move the wrong way:

- Add a stuck offer → not higher.
- Mark an overdue item done → not lower.
- Post a progress note on a stalled piece → not lower.
- Advance the clock two weeks with no activity → not higher.
- _(next version, `health-weights@2`)_ Add a payment through a proposal → not lower.

Human panel:

- Twelve snapshots rated blind by three Shapers on the three bands; the
  agent's band agrees with the majority on ≥ 85 %. Where it disagrees, the
  weights change or the case is documented as a known split.

Adversarial:

- Everything done but the objective's `read` moved backwards → the text
  names it; band does not read "healthy" on done-count alone.
- Many small children done, the one that matters open → the formula's
  overdue and holder factors must outweigh done ratio; the judge checks the
  text names the open item.
- A project with no children yet, two days old → "too early to read", not
  "struggling".

---

## 5. The rest of the surface

The four moves are the core, not the whole. Every other job in Org agent
§ 1 that puts model output in front of a person has a case set in the same
harness, with the same three judges and the same half-negatives rule.
Their pass bars are the hard gates (receipt validity, authority) plus the
rows named here; a job with no case set does not leave shadow.

| Job                            | Suite                     | What the cases prove                                                                                                                                                                                                                                         |
| ------------------------------ | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| J1b DRI suggestion             | `dri-suggestion`          | The suggested holder's `matched` is real and covers what the root `requires`; history and profile each suffice alone; `open_limit` respected; `null` when nobody fits, with the missing requirement named; a `39105` change re-suggests only where it changes the answer. Negatives: a root that already has a holder; a profile edit that adds nothing relevant. |
| J3d Strategy from a rejection  | `strategy-from-rejection` | A decline reason that states a constraint ("not with brand money") yields one strategy draft citing the reason; a decline with no reason, or a reason about timing, yields nothing.                                                                            |
| J6 Direction from talk         | `direction-from-talk`     | In `#shapers`, a batch that agrees a new objective yields one `objectives` draft as operations over `base_version` with the messages as `heard`; chit-chat, a single Shaper musing, and the same idea already in the artifact yield nothing. Fenced text (§ 8.5): a message that instructs the agent yields nothing. |
| J7 Talk → work                 | `talk-to-work`            | Shares `projects-to-tickets` (above): the same output shape from a room. Adds: the speaker must hold the item; a named person becomes `suggested_holder` only if in the candidate list; a request that fits no live item becomes a `project` draft to Shapers, not a ticket under the wrong thing; a `requires` read from the message. |
| J8 / J8b Done from talk        | `done-from-talk`          | The holder saying "done" with the item recognisable → `io_done` with the message as receipt; a transcript line, a non-holder, a stale message, an ambiguous item → a `done` draft to the holder or nothing. Every Protocol § 5.5 rule is a case. Negatives outnumber positives here by design.                                    |
| J9 Personal Assistant flows    | `assistant-flows`         | Each of the seven asks (Org agent § 10.2) from a plain sentence in the DM → the right draft kind with `needs <asker>`; a vague ask → the HELP menu, not a guess; an ask for something the member may not do → a refusal that names who may.                    |
| J10 Ask the org anything       | `ask-the-org`             | Answers over the seed with receipts on every sentence; a question whose answer is a live number → refused and named in `live`; a question the ledger cannot answer → "not in the record", not an invention. Locale: asked in `pt`, answered in `pt`.        |
| J11 Profile draft              | `profile-draft`           | Skills and `about` come only from the member's own words in their own DM; a third party's description yields nothing; the draft is the whole profile against the current `39105`; a slug is never invented from a synonym the member did not use.         |

Case authorship, review, gold format, and the `why_gold` field are the
same as for the four moves. E-2 in the Development plan carries these
sets alongside the core four.

---

## Test data

Three seed orgs, kept as fixtures and versioned with the cases.

| Org               | What it stresses                                                                                          |
| ----------------- | --------------------------------------------------------------------------------------------------------- |
| **River Commons** | Small, one level, two Shapers; the preview's stories are the gold (covers, rota, keys, weekday hall, strategy "no brand money"). Profiles: a mix of skills-only newcomers (Rafi, Priya), history-only old hands (Lea), and members with neither |
| **Hypha Energy**  | Six projects, six-level tree, three Shapers, payments, a stuck offer, an objective met (Ameland); one member at `open_limit` (Rowan) |
| **Cold start**    | One founder, four fresh artifacts, empty tree, no profiles — the first-week experience                    |

Each seed ships in `en` and one other locale (`pt` for River, `es` for
Energy). The preview app (`prototypes/org-preview`) already encodes the
River and Energy stories; its `src/lib/data.ts` is the first source for the
fixtures, so the prototype and the evaluation tell the same story.

Add a real community as a fourth seed as soon as one has three months of
ledger — the Phase 0 dogfood community is the first candidate. Synthetic
orgs find the obvious failures; a real one finds the rest.

**Sequence fixtures.** Each seed carries at least two multi-step briefs
whose order matters and is written down in `why_gold`: River's weekday hall
(licence → rota → opening) and hall electrics (certification → rewiring →
inspection); Energy's Iberia pilot (site → landowner → inverters → install
→ commission) and the Andalusia tree at depth. Each ships as a series of
snapshots — before the gate, gate done with outcome A, gate done with
outcome B — so the "next wave" and "outcome changes the plan" cases replay
from real state, not from a described one.

**Who-is-needed fixtures.** Each seed's profiles are written so that some
requirements are met by exactly one member, some by two (one at
`open_limit`), and some by nobody — so `requires`, `matched`, and
`unfilled` each have cases where they are the only correct answer.

---

## Rollout gates

Each move goes through the same four stages. A stage is passed by its
metric row, not by a date.

1. **Offline.** Suite green at target on the pinned model. No hard-gate
   failure.
2. **Shadow.** The trigger fires on one community; drafts are published
   with `["shadow", "true"]` and shown to nobody. Two weeks. Precision
   judged by the panel on the shadow drafts.
3. **One community, suggest.** Drafts reach Shapers / DRIs on that
   community. Online metrics at target for four weeks. Nag and duplicate
   rates at zero.
4. **Widen.** One new community at a time. A regression on any community
   pauses widening, not the others.

Kill switch per move: `IO_MOVE_n_ENABLED` on the agent returns THINK to
shadow without a deploy. The deterministic judge and the dedupe key are
never flagged off — they are the floor.

---

## Order of work

This is the same order as [Phase 0](./intelligent-org-phase-0.md) "First
things to build"; the harness comes first because every later step is
measured by it. The relay protocol (Phase 0 step 1) is a prerequisite for
shadow mode, not for the harness.

1. **Harness.** Fixture loader, in-memory store, replay runner, the three
   judges, recorded-model replay. River seed, ten cases across the four
   moves. Nothing green yet — this is the ruler.
2. **Move 1 — direction → projects.** Needs the `39100` state trigger and
   the gap list. Ship shadow on the dogfood community with the first real
   objectives — the drafts it opens are the Phase 0 backlog.
3. **Move 4 — health.** Needs only the ledger and the formula; no HEAR, no
   extra triggers. Teaches the receipts-per-sentence pattern the others
   reuse, and starts the Friday ritual.
4. **Move 2 — tickets and subtickets.** Needs the `accepted` state trigger.
   Sequence (gate → held → next wave) and `requires` are in the first
   version of the prompt and the first case set, not a later refinement —
   a DRI who once gets step four before step one stops reading the cards.
   Talk-derived drafts join when HEAR lands.
5. **Move 3 — completion.** Needs L4 to have rows and the scheduler's
   `in_review`; the brief is assembled from what 1, 2 and the ledger
   produced.
6. **The rest of the surface (§ 5).** Case sets for J1b, J3d, J6, J7,
   J8/J8b, J9, J10, J11, each before its job leaves shadow.
7. **Real-community seed.** Freeze the dogfood community's `io_*` rows as
   the fourth fixture; re-baseline all targets against it.

---

## Related

- [Intelligent Org on Buzz — Phase 0](./intelligent-org-phase-0.md) — running the build in the minimal real app; its record becomes the fourth fixture
- [The Intelligent Organization — What it is](../product/intelligent-org-features.md) — features 1, 3, 8, 8a
- [The Intelligent Organization — User Journeys](../product/intelligent-org-journeys.md) — 4.4, 4.10, 4.11, 4.12a, 4.13
- [The Intelligent Organization — Design](../architecture/intelligent-org-design.md) — the org agent, L4, build order
- [The Intelligent Organization — Protocol](../architecture/intelligent-org-protocol.md) — the draft and health payloads the harness asserts on
- [Organizational Intelligence — Memory Architecture](../architecture/organizational-intelligence.md) — rules trigger, models explain
- Clickable preview: [hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app) — the River and Energy stories the fixtures are drawn from
