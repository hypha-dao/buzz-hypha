---
title: 'The Intelligent Organization — State and the Change Engine, short version'
date: 2026-10-05
status: draft
tags: [architecture, intelligent-org, ai, agent, planning, buzz]
parent: docs/intelligent-org/README.md
---

# State and the change engine — short version

Read this to decide whether you like the system. The full design is
[here](./intelligent-org-state-and-planning.md). Where the two contexts
are stored is [here](./intelligent-org-context-homes.md).

---

## The idea

The org agent should think the way a good operator thinks.

It looks at **where the org wants to be**, **where it is now**, and **what
is already underway**. The difference is a gap. A project is a proposal to
close one gap: *we are here, we should be there, this is how we will know,
and these are the steps*. Tickets are those steps. A prompt is one step
written so a person or a coding agent can do it without asking.

So a project, its tickets, and the prompts are **one plan**, shown at three
sizes. The agent writes the whole plan. People approve it in pieces.

---

## A walk-through

River Commons wants a weekday hall open before August. They have never run
a weekday night, and they have no evening licence.

The agent does not suggest "open the hall". It suggests a **four-week
trial**, because nobody knows yet whether weekday buyers will come. The
plan it shows the Shapers:

1. Apply for the evening licence. *(This answer decides what comes next.)*
2. One volunteer gets a hygiene certificate. *(Same.)*
3. Book four Tuesdays. *(Waits on 1.)*
4. Tell the mailing list. *(Waits on 3.)*
5. Run the four nights and count who comes. *(Waits on 2 and 3.)*

Shapers vote on the trial, not on each step. Tomasz accepts it. He gets
tickets 1 and 2. Ticket 1 comes with a ready-to-copy prompt: what to write,
the council's rules, where to post the draft, and what "done" means.

When the licence is approved, the agent drafts ticket 3, using that answer.
When the four nights are over, it compares what was promised ("we will know
if a weekday night fills") with what happened, and suggests the next
objective from that.

---

## What the agent needs to know

Eight things. Most of them we already store in some form. The bold ones are
the gaps.

| It needs to know | In plain words | We have it? |
| --- | --- | --- |
| Where we are going | Mission, vision, objectives — each objective says **how we will know it is done** | Partly. Objectives are sentences. They do not yet say "done when". |
| Where we are | A short situation, plus live numbers fetched when needed | The situation text exists. The numbers do not. |
| What is already happening | Projects, tickets, who holds them, health | Yes. |
| What we can use | People and their time, tools, repos, partners, money posture | People exist, thinly. The rest does not. |
| What we must not do | Refusals, laws, promises, deadlines | Governance yes. The refusals live inside strategy prose. |
| Why we think our work works | The bets, and what would make us drop each one | Partly, as strategy lines. |
| What we do not know yet | Open questions. The plan starts by answering these. | No. |
| What we already tried | Past decisions, declined ideas, and how projects actually turned out | Decisions and declines yes. "How it turned out" no. |

Two rules about this knowledge:

- **Beliefs are confirmed by Shapers. Facts come from what people did. Numbers are read live**, with a date, and never copied into a belief. "We are over-exposed to one funder" can be a belief. "We hold €63k" cannot.
- **Every fact says what it is about** — which objective or project. That is how the agent pulls the right page instead of reading the whole org.

---

## How a suggestion gets made

Code does the bookkeeping. The model does the writing.

1. **Compile.** Code turns the event log into a one-page card for the decision: the objective, where it stands, what is in flight, the open questions, the rules, who could hold it, what was tried before. Every line names its source.
2. **Find the gap.** Code says "this objective has nothing under it, ten weeks left, and an unanswered question". The model does not get to invent a gap.
3. **Plan.** One model call. It must: name two or three options, drop the ones that break a rule, pick one, write *from → to* and *done when*, list the steps with the question-answering steps first, check the size against how this org works, and name a holder or say nobody fits.
4. **Check.** Code rejects the draft if the gap is wrong, a step breaks a refusal, a later step depends on a question nobody is answering, the size is off, or a number was made up.
5. **Hand it to one person.** Shapers for a new project. The holder for tickets. The agent never applies the change itself.
6. **Learn.** When the project closes, code compares the promise with the result. That comparison changes the next estimates. A lesson becomes a belief only if Shapers confirm it.

Chat uses the same cards and the same checks. Asking the agent in a DM and the Monday scan produce the same kind of plan.

---

## What people see

- **Shapers** see a project card: the change, why this and why now, and the step list as a preview. They are voting on the change. The holder owns the steps and may change them.
- **A holder** sees the next tickets, not the whole future list. Steps waiting on an answer stay hidden until the answer exists.
- **Anyone about to do a ticket** sees a **Copy prompt** button. The prompt goes stale on its own when the ticket, the rules, or the code change.

---

## What does not change

- The agent drafts. People decide.
- Work is offered. Only the named person accepts.
- Shapers vote on new projects and on direction. Holders decide the work inside a project.
- No money on a project or a ticket.
- A project is still a ticket with nothing above it. Big tickets split the same way.

---

## What we would build, in order

| Step | Build | You can judge it when |
| --- | --- | --- |
| 1 | The one-page cards, from what we already store. Project drafts carry the change and the full step list. | A confirmed objective produces a trial-sized project with its steps, and a bad idea is dropped with a reason. |
| 2 | Objectives say "done when". Strategy lines are tagged bet, rule, or refusal. Declined ideas say when to raise them again. | The agent stops suggesting things the org already refused. |
| 3 | A home for constraints, repos, partners, and norms. Repo reading. The Copy prompt on tickets. | A code ticket shows a prompt naming the repo, branch, and files. |
| 4 | A list for open questions, assumptions, signals, promises, and outside dependencies. Re-plan when a question is answered. | The second wave of tickets uses the answer from the first. |
| 5 | On close, record promise against result, and let that change later estimates. | After ten closed projects, estimates get closer. |

Step 1 needs no protocol change. Most of the quality is in steps 1 and 2.

---

## Decide these

1. **One plan, approved in stages.** The agent drafts the project and its steps together. Shapers approve the change. The holder approves the steps. Yes?
2. **The agent starts from a computed gap.** It explains a gap the code found. It does not go looking for interesting projects. Yes?
3. **Unknowns come first.** If a plan depends on a question, the first ticket is the one that answers it. Yes?
4. **Numbers stay out of beliefs.** Live figures are separate readings with a date. The agent may cite them. It may not write them into mission, strategy, or a project brief as if they were permanent. Yes?
5. **A new list for the messy stuff** — open questions, assumptions, "customers keep asking for X", promises, things we are waiting on. Any member can add one. It is a work list, not a belief, so it is not a Shaper vote. This is the part that grows. Yes?
6. **On close, we score the promise.** Every finished project records whether "done when" happened, and later plans use that record. Yes?

A no on 1, 2, or 3 changes the system. A no on 4, 5, or 6 shrinks it: it still plans, with less memory.
