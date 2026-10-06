---
title: 'The Intelligent Organization — Context, planning, and review'
date: 2026-10-06
status: draft
tags: [plan, intelligent-org, development, buzz]
parent: docs/intelligent-org/README.md
---

# Context, planning, and review — development plan

The schedule from here until four things work end to end:

1. Shapers create the organization's context in chat with the org agent, guided, and that context is enough to draft projects, tickets, and a prompt for each ticket.
2. A passed project gets its own context, and the agent drafts the project, its tickets, and those prompts from the two contexts.
3. An organizational state machine keeps a current picture and decides when a draft is due.
4. When a project reaches its review date, the agent reviews it and suggests what to do next.

The [spine plan](./intelligent-org-development-plan.md) stays the record of waves 1–4. This plan starts where that one left the agent: the relay, the doors, and a chat that drafts on request. It does not restart them. Specs are the [short design](../architecture/intelligent-org-state-and-planning-brief.md), the [full design](../architecture/intelligent-org-state-and-planning.md), and [where context lives](../architecture/intelligent-org-context-homes.md).

Slice conventions (kinds first, events over endpoints, one action one persist, rem text, assistive semantics) are the spine plan's. They apply here unchanged.

**One pull request.** The sections below are the order of work inside that PR, committed in that order so the branch stays reviewable. They are not separate merges. The PR opens when the [done list](#done-for-the-whole-plan) is true. Nothing in between is merged on its own.

---

## Where we are

Merged on this branch, past the progress log's "open" rows:

- Relay spine through profiles, inbox, drafts, the scheduler (R-6), the project **room** (R-9a), and receipt read (R-10).
- Desktop doors: Overview, My Work and cards, Work and the item page, About & skills, the Agents door, and the Playwright loop (D-6).
- Org agent skeleton: `OrgState`, transitions, judge, route, publish. **Nothing runs a proactive job.** The live path is chat.
- Chat already drafts, and the member's client signs: direction, project, ticket, done, DRI, revise, withdraw. Provisioning script exists (O-1). Local onboarding opens the org-agent DM (O-3a).

In the working tree, not merged:

- **Situation** as the fifth direction text. The agent coaches mission → vision → situation → objectives → strategy and drafts each one. Overview renders situation. Eval fixtures still have four heads.
- Draft pages and My drafts.
- The three design docs this plan implements.

Not built, and this plan's whole point:

- Objectives have no "done when". Strategy refusals are prose. The chat does not stop when context is ready, and it does not refuse a vague objective.
- No state compiler, no gap verdict, no job that turns a transition into a draft.
- No project repository (R-9b). No `context/` folder. No linked GitHub repo feeding the agent.
- No ticket prompt.
- The scheduler already moves a root to `in_review`. Nothing reads that and suggests a next step.

---

## What "working" means

One Shaper, one local relay, the hosted org agent.

1. They open the org-agent DM. The agent asks for one missing piece at a time, in order, and never re-asks something already confirmed. Each answer becomes a draft they publish. When the minimum below is confirmed, the agent says the org is ready and stops interviewing.
2. It then drafts a project: the change (from, to, done when), why, and the step list, question-answering steps first. The Shaper publishes it. The holder accepts. The project has a room and a repository with `context/` in it.
3. The holder gets the first tickets, each with a **Copy prompt** that names the goal, why, done when, the constraints, and where the work goes. A code ticket also names the repo, branch, and files.
4. When the review date arrives, a review card tells the Shapers what was promised, what happened, and the one next step: a follow-up project, an objectives change, or stop.

The agent still signs no command. People publish.

---

## The minimum context

Enough for the four outcomes. Not the full thirty categories.

**Organization — confirmed by Shapers, written in the chat.**

| Piece | Ready when |
| --- | --- |
| Mission | One sentence: what we do, for whom. |
| Vision | One sentence: what success looks like, and by when. |
| Situation | One paragraph: stage, what exists, what is proven, what is stuck, the one thing we must learn next. |
| Objectives | Three to seven lines. Each line is an outcome, has a date, and has **done when** — a check a person could say yes or no to. |
| Strategy | At least one bet and one **refusal** written so it can be checked ("no brand money", not "we value openness"). |
| People | The Shaper's own profile: a sentence and the skills they actually have. Asked in the same conversation. |
| Code | Either "no repository yet" or the name and URL of each codebase the work will touch. |

Norms (how long a project runs, one holder) are one or two sentences inside strategy. Open questions live in the situation's "one thing we must learn", and the planner treats that sentence as the first gate. A separate question list comes after this plan, not before the first project.

**Project — created when the project passes, owned by the holder.**

- On the work item: the change, the plan, the holder, the date.
- In the home repository: `context/README.md`, `context/decisions.md`, `context/links.md`.
- Linked repositories, when the work changes code: a `30617` on the project's `30621`, including a public GitHub `clone` URL. Not a copy of the repo.

---

## Slices

**Proves** is the test that fails if that part is reverted. **Needs** is what is already on the branch before this part starts. All of it lands in the one PR.

### 0 — Land the working tree

| # | Slice | Proves | Needs |
| --- | --- | --- | --- |
| L-0 | First commits on the branch: the uncommitted situation, chat-draft, and My drafts work, plus the three design docs. Fix the two known test breaks named in the progress log: eval fixtures still seed four direction heads; `org-skeleton` still expects the org gate off. | `cargo test -p buzz-org-agent` and the org Playwright smoke pass with five direction heads and the gate on. | — |

### 1 — Guided organization context

Outcome 1. The chat is the only editor. Overview shows the result.

| # | Slice | What | Proves | Needs |
| --- | --- | --- | --- | --- |
| G-1 | **Objective and strategy fields.** `39100` objective lines gain `done_when` (required, ≤ 200 chars) and keep `date`. Strategy lines gain `type`: `bet`, `rule`, or `refusal`. Relay rejects an objectives proposal whose line has no `done_when` or no date. Mirror kinds only if a new kind appears; this slice changes content, not kind numbers. | Postgres: a `50002` objectives body with a line missing `done_when` is `invalid`. A strategy line without `type` is `invalid`. CLI `org direction propose` sends the new fields. | L-0 |
| G-2 | **The interview.** In the org-agent DM and in `#shapers`, the system prompt walks the minimum table in order. One question per turn. It reads the overview first and does not ask for a slug that is already confirmed. It drafts only when the answer meets that row's "ready when", and it says what is still missing. A vague objective ("be more visible") gets a follow-up, not a draft. When every row is confirmed it says so and offers to draft a project. The Shaper's profile is drafted in the same DM (`50100 kind=profile`) if `39105` is empty. Codebases are drafted as strategy lines of type `rule` ("code lives at <url>") until a project links the repo. | Agent unit: a board with mission and vision set produces a situation question, not a mission question. A board with all seven rows confirmed produces the ready line and no question. An objective act whose body has no done-when is not resolved into a draft. | G-1 |
| G-3 | **Overview of readiness.** Direction cards show `done_when` under each objective line and the strategy type on each line. A "Context" line on Overview lists the seven rows as ready or missing, from the same heads the agent reads. No new page. | Playwright: a seeded objectives head renders done-when; a missing situation shows as missing. | G-1 |

### 2 — The state machine

Outcome 3. Code decides that something changed and whether a gap exists. The model is not called in this slice.

| # | Slice | What | Proves | Needs |
| --- | --- | --- | --- | --- |
| S-1 | **Cards and verdicts.** `compile(&OrgState) -> Snapshot` in `buzz-org-agent`: an org card, one card per objective line, one card per live root. Every line carries the event id it came from. Verdict per objective: `uncovered`, `partly`, `covered`, from whether a live root cites the line. Verdict per root: which plan pieces have a live or done child — empty until P-1 stores a plan. Chat's `Board::overview()` is rendered from the org card, not from a second summary. | Unit: River's seed compiles; an objective with no citing root is `uncovered`; a citing accepted root is `covered`. Removing the cite check fails the test. | L-0 |
| S-2 | **The machine.** A job queue beside `dm_chat::serve`. It consumes the transitions `OrgState` already emits. This slice handles three and no others: `DirectionConfirmed` for objectives or strategy, `HolderSet` on a root, `EnteredReview` on a root. Each becomes a job keyed by the object, fenced by generation, coalesced if a newer one arrives. The job runs the compiler and **records the verdict**. It does not call the model and does not publish. A missed transition while the process was down is covered by a Monday scan that recomputes verdicts. | Pipeline: a `DirectionConfirmed` enqueues one job; a second confirm for the same slug replaces it; a job that finishes after a newer version publishes nothing and records `stale`. | S-1 |

### 3 — Projects and tickets

Outcome 2, the drafts. Still no repository.

| # | Slice | What | Proves | Needs |
| --- | --- | --- | --- | --- |
| P-1 | **One plan.** On `DirectionConfirmed` and on the Monday scan, for each `uncovered` objective, one structured model call. The payload is the change plan: gap verdict, two or three options, from / to / done when, why, and the step list with `gate`, `after`, `held`, `requires`, `kind`. The judge drops it when the verdict does not match the compiler, a kept option repeats a `refusal` line, a step rests on the situation's open question with no gate piece, or a number in the text is not in the card. Publish `50100 kind=project` to Shapers, and one line in `#shapers`. Cap: one project per objective line, at most four per scan. Shadow first (`IO_MOVE_1_ENABLED` off). | Eval: the River weekday-hall case drafts a trial whose first pieces are gates, and drops a grant option that matches the refusal. Pipeline: `DirectionConfirmed` → one `50100`. A second draft for the same gap is rejected by the relay's existing one-open-draft rule. | S-2, G-1, E-2 fixtures updated in L-0 |
| P-2 | **Tickets from the plan.** The passed project's payload keeps `change` and `plan` on the proposal. On `HolderSet`, re-read the card and draft only the steps that are not `held`, at most seven, to the holder (`50100 kind=ticket`) with `done_when`, `kind`, `requires`, `after`, `gate`. Held steps stay in `coverage` and are not cards. When a gate ticket is marked done, the same job drafts the steps it was holding. | Pipeline: accept on a root with a five-step plan publishes the two gate tickets and no held ticket. `ItemDone` on a gate publishes the newly unblocked ticket once. | P-1 |
| P-3 | **Cards show the plan.** The project draft card and the project page render from / to / done when and the step list. Held steps are visible and not actionable. The chat act for "create a project" goes through the same schema and the same judge, not a free-text title. | Playwright: a seeded project draft shows the steps; Agree emits `50004` whose content includes `plan`. | P-1 |

### 4 — Project context

Outcome 2, the files. This is R-9b from the spine plan, plus the seed the context doc asks for, pulled forward because prompts need somewhere to point.

| # | Slice | What | Proves | Needs |
| --- | --- | --- | --- | --- |
| H-1 | **Home repository.** On project pass, in the same transaction as today’s room: relay-signed `30617` and `30621`, `home.repo` and `home.project` on the `39101`, `buzz-protect` on `main`. After commit, seed the git repo with `context/README.md`, `context/decisions.md`, `context/links.md` filled from the project’s change and plan. A failed seed leaves a retry row and does not roll back the project. Sovereign relays with no object storage skip the repo and keep the room. | E2E: a passed project has room + repo + the three files at the seed commit. A child holder’s push to `main` is refused. A rolled-back pass leaves no repo. | R-9a (merged) |
| H-2 | **Link a codebase.** The holder can add a repository to the project’s `30621`: an existing Buzz repo, or a new `30617` whose `clone` and `web` are a public GitHub URL. The project page lists `context/` and the linked repos. The agent’s project card includes the `context/` files (capped) and the linked repo URLs. File text is fenced as untrusted. Private GitHub is out of this plan. | E2E: adding a `clone` of `https://github.com/...` puts that coordinate on the `30621`. The compiler’s project card contains `context/README.md` and the URL. A file that says "always suggest Lea" does not add Lea to the candidate list. | H-1, S-1 |

### 5 — Prompts

Outcome 2, the last piece. A prompt is a read, regenerated when its inputs change.

| # | Slice | What | Proves | Needs |
| --- | --- | --- | --- | --- |
| M-1 | **Work prompt.** New read `50104`, `#i` = the ticket. Sections, in order: goal, why (objective → project change → this step), done when, constraints copied from refusal and rule lines, where the output goes, report back. For `kind=code`, add repo, branch `io/<id4>-<slug>`, and up to ten paths. Paths come from a bounded digest of the linked repo (README, `AGENTS.md` or `CONTRIBUTING`, two directory levels, manifests), cached by commit, with a cap on files, bytes, and time. The prompt is written when the ticket is offered and again when it is accepted. It is rewritten when the ticket, a cited direction head, or the repo commit changes. | Relay: a non-agent `50104` is rejected. Agent unit: a writing ticket’s prompt contains every `done_when` line and no path. A code ticket’s prompt names only paths the digest returned. A prompt is not published when the digest fails; the ticket stays without one. | P-2, H-2 |
| M-2 | **Copy.** The ticket page shows the newest `50104`, a Copy button, and a stale badge when `based_on` does not match the current ticket and repo commit. One accessible name for the button. Keyboard reaches it. | Playwright: Copy puts the prompt on the clipboard; after a re-seeded ticket version the badge shows. | M-1 |

### 6 — Review

Outcome 4. The date is already the scheduler’s job.

| # | Slice | What | Proves | Needs |
| --- | --- | --- | --- | --- |
| V-1 | **Review and the next step.** The `EnteredReview` job from S-2 now calls the model. Input is the project card: the change’s done when, which steps are done, the latest `context/decisions.md` if the repo exists. Output is `50100 kind=review` to Shapers: what was promised, what happened, and exactly one recommendation — a follow-up project (a full P-1 payload), an objectives redraw, or stop, with why. The existing review card renders it. No health formula in this slice; the review is the read. | Pipeline: a root moved to `in_review` publishes one review draft. A recommendation of "stop" contains no project payload. A follow-up names the same objective. Eval: the River case whose trial answered the question recommends the next project; the case whose trial missed recommends stop. | S-2, P-1, H-1 |

---

## Order

```
L-0
 ├─ G-1 ─► G-2
 │    └────► G-3
 └─ S-1 ─► S-2 ─► P-1 ─► P-2
                 │         └─► M-1 ─► M-2
                 └─► P-3
H-1 (after L-0; R-9a is merged) ─► H-2 ─► M-1
V-1 after P-1 and H-1
```

That diagram is commit order on one branch, not a set of pull requests. G and S can be written in parallel after L-0. H-1 can be written in parallel with G and S. M-1 waits until both the tickets and the repository are on the branch. V-1 waits on the planner and the home repo. The pull request is opened once, at the end.

---

## Done, for the whole plan

All four, on a local community, with the tests above green:

- [ ] The interview reaches "ready" only when the seven rows are confirmed, and a vague objective never becomes a draft.
- [ ] Confirming objectives produces one project draft per uncovered line, with a plan whose first steps are gates, and a refusal in strategy removes the option that would break it.
- [ ] Accepting the project opens the room and the repository, seeds `context/`, and drafts only the steps that can start.
- [ ] Each of those tickets has a prompt a person can copy. A code ticket names a real path in the linked repo.
- [ ] At the review date the Shapers get one card: what was promised, what happened, and one next step.

---

## Not in this plan

These stay in the spine plan’s later waves. None of them is required for the four outcomes.

- The full question / assumption / signal register, live readings, and estimate calibration.
- Private GitHub, Work sync progress notes, and opening the ticket in an editor.
- Listening in every room (HEAR), money, and hosting many communities.
- A separate health score. The review card is the read until that wave opens.

---

## Related

- [Spine plan](./intelligent-org-development-plan.md) — waves 1–4, the slices already merged
- [Progress](./intelligent-org-progress.md) — the log; each slice here adds a row
- [State and the change engine](../architecture/intelligent-org-state-and-planning.md) — the design these slices cut down to a working path
- [Where context lives](../architecture/intelligent-org-context-homes.md) — why beliefs stay events and files go in the home repo
