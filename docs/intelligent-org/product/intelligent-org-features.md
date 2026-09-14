---
title: 'The Intelligent Organization — What it is'
date: 2026-09-14
status: source of truth
tags: [product, intelligent-org, buzz]
---

# The Intelligent Organization — What it is

Source of truth for the intelligent organization, built on Buzz. This document says what it is
and what users can do thanks to it. [Design](../architecture/intelligent-org-design.md) says how
to build it on the Buzz relay; the [Protocol](../architecture/intelligent-org-protocol.md) pins
the event kinds. Where a Buzz surface has a different name from the one used here, the design
document says so; the behaviour described here is the contract.

An organization is one Buzz **community** — one relay URL, one member list, one signed event log
shared by people and the org agent.

---

## What intelligent means

An organization is intelligent when it can reliably run one loop:

```
hear what happened
    → remember what it is for
        → put the next action in front of the right person
            → watch what came of it
                → revise what it believes
```

Everything in this document is a feature of that loop.

The test is concrete: **any member opens the app and knows, without asking anyone, what matters
and what to do next.**

Two rules hold everywhere:

1. **The AI drafts. People decide.** The AI never decides money, membership, or what the
   organization believes.
2. **Work is offered, never assigned.** The named person accepts or declines. Decline sends it
   back. **And:** when a project or ticket has no DRI, anyone can open a **project DRI**
   proposal. If the Shapers agree, that person holds it. That is a naming vote, not an
   offer on a card — both paths stay.

---

## Who is who

| Who        | What they are                                                                                                   |
| ---------- | --------------------------------------------------------------------------------------------------------------- |
| **Member** | Joined. No job until they accept one.                                                                           |
| **DRI**    | Accepted a project (holds the job) or a ticket (does the work). A relationship, not a title.                    |
| **Shaper** | Sets direction: the org's mission, vision, objectives, and strategy; which projects exist; what they are worth. |

The founder — the community's owner in Buzz — is the first Shaper. After that, only Shapers name
Shapers. One person can be all three at once. The org agent is a member too, with its own key;
it never holds a role.

---

## The features

### 1. Direction stays current

Shapers have a standing chat with the org agent. They talk about what the org is for, drop
documents and links, say "we now care about X." The agent drafts four things from that talk;
a Shaper confirms each version:

| Direction      | Answers                                                                                     |
| -------------- | ------------------------------------------------------------------------------------------- |
| **Mission**    | Why we exist — what we do, for whom.                                                        |
| **Vision**     | Where we are going — what the world looks like if we succeed.                               |
| **Objectives** | What we aim to have done soon — a short list of near-term outcomes, each with a rough date. |
| **Strategy**   | How we get there — the few bets and priorities that guide which work we take on.            |

Each is a short text with a version. Mission and vision change rarely; strategy changes as the
org learns; objectives change most often — when one is reached or dropped, the list is redrawn.
Objectives are also where direction meets work: a project the org takes on should serve one of
them, and a review asks whether it did. There is no separate "org brief" — these four are what
the org believes about its own direction.

Where the talk happens depends on how many Shapers there are. With several, it is the
**Shapers room** — a private channel the org agent is a member of: everyone there sees the
draft, one of them confirms. With one Shaper — the founder in a new org — the same conversation
happens in **their own DM with the agent**. No room to open, no one to wait for. The moment a
second Shaper is added, the conversation moves to the Shapers room; nothing confirmed is lost.

This is not a one-time founding interview. The chat never closes. Everything else the AI does
reads from the latest confirmed mission, vision, objectives, and strategy.

**A Shaper can:** set or change the mission, vision, objectives, or strategy by talking, any
day, and see that reflected in what the AI suggests next.
**A founder can:** formulate all four alone, in their own chat, before anyone else joins.

### 2. The org listens

Chats, calls, and transcripts are the input. Members talk to each other and work in the open;
the org hears needs, gaps, and commitments as they happen.

Nobody files anything. There is no "new ticket" button.

**A member can:** say "who signs the hall licence?" in a group chat and trust the org heard it.

### 3. Talk becomes work — with authority

The AI drafts work from what it hears. Who can make it real depends on what it is:

| What                        | Who makes it real                 | Date it carries          |
| --------------------------- | --------------------------------- | ------------------------ |
| **Project**                 | A **Shaper**                      | **End date**             |
| **Ticket** (under anything) | The **DRI of what it sits under** | **Estimated completion** |

A project is a ticket with nothing above it. Both are the same kind of thing — a piece of work
with a holder, a date, and children — which is why one rule covers every level. (Earlier drafts
called a project a _mandate_ and gave it a pot; neither survives. See feature 7.)

One rule at every level: whoever holds the thing above you makes your draft real. If no project
covers a need, the draft goes to Shapers. If one does, the ticket draft goes to that project's
DRI. And a ticket can have tickets under it — the person holding a ticket can split it and
offer the pieces to others, as deep as the work needs, without going back up to the project DRI.
Anyone's talk is context; only the right role can stamp it live. Same gap mentioned twice stays
one card.

**Not only from talk.** The agent also holds the confirmed direction next to the live work tree
and drafts the gap between them, unprompted. The chain is fixed:

1. Shapers agree **direction** (they can also suggest their own projects).
2. AI suggests a **project** for an objective or strategy line nothing serves.
3. When a project exists with no DRI, AI suggests a **DRI** — the card says
   **AI is asking you** if it named you, or **AI is suggesting for (name)** if
   you are the one who confirms or offers.
4. When a ticket is held, if it names pieces nobody covers, AI drafts those
   **subtickets** onto **Needs your answer**.

The receipt is the objective or the project description, not a message. It
looks at three moments only: right after a Shaper confirms a new direction
version, right after a project goes live, and on a weekly scan — never per
message. One open suggestion per gap; a dismissed one is not raised again
until something changed.

Anyone can also **publish** from their **DM with the org agent** (the
_Personal Assistant_): draft a direction, project, money-out, or DRI-naming
proposal; create a ticket (for themselves or someone else); mark their own
ticket done; ask the org anything. Shapers do the same in the Shapers room.
The agent drafts; a person opens it; the Shapers decide.

A project carries no budget and a ticket carries no pay. If a job should be paid, that is a
conversation between the person taking it and the person offering it (see feature 7).

**A Shaper can:** approve a project the AI suggested, set the review date — in
chat. Naming a DRI is a separate move (offer, or a **project DRI** proposal).
**Anyone can:** when a project or ticket has no DRI, propose themselves or
someone else as DRI. Shapers do it in the Shapers room; everyone else
does it in their DM with the agent. The draft is a proposal — tagged **project DRI**
under **Decisions → Work** — nothing is held until the Shapers agree.
That vote is a Shaper decision about who is named. The offer–accept path
still applies when the work is offered to a person (they can decline).
**A Shaper can:** confirm a new objective and, without asking, get a project draft for it if
nothing in the tree serves it yet.
**A project DRI can:** be told "your project names a licence and no ticket covers it — here is
a draft" before anyone raises it in the room.
**A DRI can:** write "Lea, can you take covers?" and have that become a ticket offer, without a
form.
**A ticket holder can:** say "Jun, could you print the rota?" and offer a piece of their own
ticket to Jun — Jun holds that piece, they still hold the whole. They confirm
that piece themselves; the project DRI does not need to agree.

### 4. Work is offered, never assigned

The named person sees one clear screen: the job, why them, the dates. They accept or decline. Decline is respected — _not my thing_ or _no room right now_ — and the card
goes back to whoever offered it — a person, or the agent.

**In progress means a DRI.** A ticket that is doing always has a holder. Offered
or waiting can still say nobody yet. If the agent put it out, the ticket
page says **Offered by AI**.

**A member can:** decline work without explaining themselves to the whole org.

### 5. Talk moves work

If the **DRI** writes in a channel or in their DM with the agent that their ticket is done, the
agent marks it done and posts the receipt: which message, which room. The DRI's own signed
message is the confirm — no card, no waiting window. Anyone else saying it changes nothing; the
agent may nudge the DRI, that is all. A done with open children under it is refused with the
reason.

A done heard **on a call** is different: speaker attribution in a transcript is not a
signature. The agent surfaces it to the DRI — _"you said covers is done — mark it?"_ — and the
DRI's reply is the confirm. Transcripts nudge; they never close.

Done first; other obvious moves (took it, dropped it) later.

**A DRI can:** finish work by saying so where they already talk, and see the board agree.

### 6. Everyone has a home

Five doors, one glance each. **Overview** is the Org door. In the Buzz desktop they sit in the
sidebar beside Home, channels, Forum, DMs, Agents, and Workflows.

| Door           | What it answers                                                                                                 |
| -------------- | --------------------------------------------------------------------------------------------------------------- |
| **Overview**   | Who are we? Mission, vision, objectives, strategy, established, founder, Shapers, members, who holds which job. Each direction card opens to its full text, every version, and the proofs behind each line — with receipts. |
| **Work**       | Who is working on what? Every project and ticket, DRI or _open_, dates visible, and on each project page the agent's **health read** (feature 8a). |
| **Decisions**  | What the Shapers decide: **Work** (project approval and **project DRI**), **Money** (out only), **Direction**, **Join** (people only). Anyone can read; only Shapers vote. |
| **My Work**    | What needs my tap — including AI cards — what I hold, what I offered (**You offered**), and what is finished. Waiting-on-me stays in the first column. Shapers also see open decision cards here. |
| **My Profile** | Who I am in this community — balances, current work, earlier work, recent decisions. Identity is one keypair across communities; the profile is per community. |

The board door is called **Work**, not _Projects_: Buzz already has a Projects surface for git
repositories, and the two must not share a name. Earlier drafts of these documents say
_Projects_ for the same door.

Empty states say so: _Nothing needs you._

**Anyone can:** see the whole org — who shapes, who holds, what is open — without asking.

### 6a. Join is a people decision

A person asks to join the community. Shapers decide on **Decisions → Join**. Nothing lands
on them until they accept work. Join cards have no Recipient. Orgs do not
request to join as members in this slice. (Community owners and admins can still add members
directly — that is Buzz's existing administration, not a Join decision; a Join decision is how
a _request_ is answered.)

**A person can:** ask to join and wait on the Shapers.
**A Shaper can:** let them in, or not.

### 7. Money is agreed in chat and moved by proposal

Work flow and money flow are **separate**. No sum lives on a project or a ticket. Money is in
two places only: **Decisions** (where it moves) and **My Profile** (balances held).

Money proposals are **out only** — pay or reimburse. Incoming money (a
sponsorship, a grant as a vote) is not a proposal. “We do not take brand
money” is **strategy**, not a money decision. A grant can land as a ledger
fact; it is not something the Shapers vote in.

How pay works: when the work is done, the person who did it — or the person above them — tells
the agent _"draft a proposal for the Shapers for this work — 150 USDC"_. The draft carries the
done receipt and the sum. The Shapers decide, as with any proposal. In the MVP **every payment
is a Shaper decision** — no sum is small enough to skip them, and no DRI approves pay for the
people under them.

**Settlement is outside Buzz.** Buzz records the decision and, when a Shaper marks it, the
settlement receipt (a transaction reference, a note). It does not hold or move funds. The
balance on a profile is the sum of settled proposals. A treasury integration — Hypha or
another — can later execute passed proposals; nothing here changes when it does.

**Later, not MVP — pay agreed in chat.** The person holding the work and the person above them
agree a sum where they already talk; the agent remembers that line, so _"…whatever we agreed"_
works in the draft request, and if the named sum differs from the agreed one the draft shows
both. This needs the agent to hear rooms (feature 2) and is scoped after it.

The AI never moves money. It drafts on request — a person still takes it through governance.

**A member can:** get paid for a done ticket without filling in a form.
**A DRI can:** ask for the pay proposal for someone under them.

### 8. Reviews write themselves

Every project has an exact end date and **closes on it by default**. As the date nears — in the
**last fifth of the project's run**, not on the day, so a two-week project is briefed three days
out and a year-long one ten weeks out — the Shapers get a **brief** and a **recommendation**. The brief is
the story: what was held, what was done, what was not, what was paid out through proposals,
whether the objective it served moved.

The recommendation answers one question: _does anything follow this?_ Either the agent drafts a
**follow-up project** — the next piece in the same domain, with a suggested DRI, description, and
end date, because the objective is still live or the brief shows a clear next step — or it says
**no further work in this domain is needed**, with the reasoning and receipts. Extending the
existing project is possible but is the override, not the default.

The Shaper opens the follow-up or moves on; either way the project closes on its date. The
close itself is a **date rule the relay runs** on the end date a Shaper set — not the agent's
judgment, and not a Shaper's tap. A Shaper can override it by setting a new end date. The agent
never closes, extends, or opens a project by itself. What the Shaper chose, and what happened
after, is remembered so the next recommendation is sharper.

**A Shaper can:** end a project cleanly on its date and know, before it ends, whether something
should come next.

### 8a. The org reads its own health

Every live project carries the agent's **health read**: one band — _struggling_, _wobbly_,
_healthy_ — and a short paragraph, refreshed weekly and whenever the project's ledger changes.
The band is computed from the ledger (done against elapsed time, overdue pieces, offers nobody
answered, weeks of silence, pieces with no holder, whether the objective it serves moved); the
paragraph is written by the agent, and every sentence points at the ledger rows behind it. The
last sentence names the one thing most pulling the band down, when there is one.

It is a read, not a decision: it lives on the project page, not on a card, and nothing changes
state because of it. Shapers rate the band blind once a week; where they and the agent disagree
is how the read gets better.

**Anyone can:** open a project and see, without asking the DRI, whether it is going well and
what would change that.

### 9. Ask the org anything

"Have we dealt with this before?" gets an answer with receipts — the threads, the decisions,
the outcomes. The org remembers what it tried and what happened, not just what was said.

**Anyone can:** ask at 2am and get the history, the root cause, and who shipped the fix.

### 10. Newcomers land somewhere real

A new person builds their profile by talking — social links, what they like to do. What happens
next depends on how they arrived:

- **Invited into a specific community.** They land there as a member, nothing more. The AI
  tells them: _"I'll let the others know about your skills and that you're available for
  work."_ The org now knows who they are; offers come when there is a fit.
- **No specific community in mind.** The AI suggests communities that might fit — based on
  their profile: purpose, place, the kind of work they like. They pick one and ask to join
  (feature 6a). This needs a directory of communities and is scoped after the single-community
  loop works.

No work queue on day one. The path is stranger → member → DRI, one accept at a time.

**A newcomer can:** arrive knowing nobody and be findable for work that fits — without applying
for anything.

---

## What this is not

- Not automation of decisions. Every consequential state change has a human confirm — or a
  date rule a human set.
- Not a chat product with a bot. The chat is how the org perceives; the loop is the product.
- Not a payment system. Money is decided by proposal and settled outside Buzz.
- Not a budget tree. No pot, envelope, or sum lives on a project or ticket.

---

## Related

- [The Intelligent Organization — User Journeys](./intelligent-org-journeys.md) — DRI, Shaper, member-not-yet-DRI, and the org agent through the app
- [The Intelligent Organization — Design](../architecture/intelligent-org-design.md) — how to build the features above on Buzz
- [The Intelligent Organization — Protocol](../architecture/intelligent-org-protocol.md) — the event kinds, tags, and state machines
- [Organizational Intelligence — Memory Architecture](../architecture/organizational-intelligence.md) — the memory that makes feature 9 possible
- [The Intelligent Organization — Current State](../architecture/intelligent-org-current-state.md) — what Buzz has today and the gap
- [Intelligent Org — Exploration](./intelligent-org-exploration.md) — how we got here, Buzz vs Hypha (historical)
- Clickable preview: [hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app) — River Commons and Hypha Energy
