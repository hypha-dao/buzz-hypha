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

1. **The AI drafts. People decide.** The AI never decides money, who is in, or what the
   organization believes.
2. **Work is offered, never assigned.** The named person accepts or declines. Decline sends it
   back. **And:** when a project or ticket has no DRI, anyone can open a **project DRI**
   proposal. If the Shapers agree, that person holds it. That is a naming vote, not an
   offer on a card — both paths stay.

---

## Who is who

| Who        | What they are                                                                                                   |
| ---------- | --------------------------------------------------------------------------------------------------------------- |
| **Member** | Came in by invite link (or created the community). Says on their profile what they can do. No job until they accept one. |
| **DRI**    | Accepted a project (holds the job) or a ticket (does the work). A relationship, not a title.                    |
| **Shaper** | Sets direction: the org's mission, vision, objectives, and strategy; which projects exist; what they are worth. |

The founder — the community's owner in Buzz — is the first Shaper. After that, Shapers are added
and removed by the Shapers' own decision (feature 1a). One person can be all three at once. The
org agent is a member too, with its own key; it never holds a role.

**The org agent is there from day one, and it is the only agent the org provides.** Nobody
installs it, picks a model, or pastes a key: Hypha hosts one for every community on a Hypha
relay, and it is a member the moment the community exists — the founder's first DM works before
there is a second person. An org that would rather run its own can: the Shapers point the org at
an agent they host, under their own key and their own model (feature 1a), and the hosted one
stands down. Either way it is one org agent per org.

**It is in every conversation.** Every channel and every DM in the community has the org agent
in it from the moment it exists — nobody invites it, nobody can keep it out, and it is not
shown as a participant: your DM with a colleague looks like a DM with a colleague. It is silent
there until you tag it. What it hears anywhere is the org's to remember and to cite: an answer
or a suggestion may point at something said in a conversation you were not in. That is a
decision this org makes on purpose — **radical transparency** — and it is stated once, where
people join, not in every chat. See feature 2.

Members may still bring **their own agents** through Buzz's Agents door — a personal agent that
runs on their machine, under their key and their model, and works for them. The one ready-made
choice there is **Work sync** (feature 5a): the agent that watches the member's checkouts,
pushes their branches, and reports their progress on work they hold. Beyond that the door
starts empty in the Hypha desktop — Buzz's sample personas (Fizz, Honey, Pollen) are not seeded
— and the org agent never appears in it, because it is not anyone's to configure.

Shapers decide **together**. When there is more than one, nothing they decide — direction, a
project, a DRI, the Shaper set itself, later money and joins — passes on one person's word; it passes
when enough of them have agreed, and how many is enough is a rule the org sets (feature 1a).

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
**Shapers room** — a private channel the org agent is a member of. Everyone there sees the
draft; when a Shaper opens it as a new version, it becomes a **direction decision** that every
Shaper gets: each agrees or declines, and the version is confirmed only when the org's rule for
direction is met (feature 1a) — by default, more than half of the Shapers. Until then the
previous version stands and the agent keeps working from it. With one Shaper — the founder in a
new org — the same conversation happens in **their own DM with the agent**, and their agree is
the whole vote. No room to open, no one to wait for. The moment a second Shaper is added, the
conversation moves to the Shapers room and every later version needs the rule; nothing confirmed
is lost.

This is not a one-time founding interview. The chat never closes. Everything else the AI does
reads from the latest confirmed mission, vision, objectives, and strategy.

**A Shaper can:** propose a change to the mission, vision, objectives, or strategy by talking,
any day, and — once the other Shapers have agreed — see it reflected in what the AI suggests
next.
**A founder can:** formulate all four alone, in their own chat, before anyone else joins.

### 1a. Shapers decide together

Everything the Shapers decide is a **proposal** with the same life: someone opens it, every
Shaper gets it on **My Work → Needs your answer** and under **Decisions**, each taps **Agree** or
**Decline**, and it passes the moment the org's rule for that kind of decision is met. There is
no chair and no casting vote; the proposer votes like everyone else — opening is not an agree,
so a Shaper can put something in front of the others without pre-committing. A proposal that
can no longer pass — too many declines — is rejected. One that nobody finishes within the
**decision window** (default seven days) expires, and the agent tells the proposer. Once
passed, it executes: the version is confirmed, the project goes live, the DRI is named, the
Shaper is added.

**Four things are proposals** in the first version, and only these: **project**, **DRI**,
**direction**, and **Shapers** — the set itself, its rules, and which org agent it uses. Two more come later: **money**,
when the treasury contract lands (feature 7), and **join**, when an org wants a door people
can knock on (feature 6a). Everything else in the org is one person's tap on their own work.

**The rule.** For each proposal kind the org sets how many Shapers must agree:

| Rule         | Passes when                                                     |
| ------------ | --------------------------------------------------------------- |
| **majority** | more than half of the eligible Shapers agree — **the default**  |
| **all**      | every eligible Shaper agrees                                    |
| **N**        | at least N eligible Shapers agree (N is capped at the count)    |

A three-Shaper org on _majority_ needs two agrees, so the proposer plus one other; a
two-Shaper org needs both. **N = 1** is the fast path a small org may choose — any one Shaper
can approve a project alone — and it is a choice the Shapers make together, never the default
once there are two of them. With a single Shaper every rule collapses to their own agree.

**Who is eligible.** The Shapers at the moment the proposal opened; adding or removing a Shaper
mid-vote does not change the arithmetic of votes already open. **Nobody votes on themselves**: a
Shaper who is the named person of a proposal — the DRI being named, the member being added or
removed as Shaper, later the payee — is not eligible for that one, and the threshold is counted over the
rest. With two Shapers this means the other one decides alone; that is the point.

**Adding a Shaper.** Any Shaper opens a **Shapers → add** proposal naming a member. The Shapers
decide under the _Shapers_ rule. If it passes, the seat is **offered**, like any work: the named
person sees it on My Work, accepts or declines, and is a Shaper from their accept — never
before. On accept the relay adds them to the Shapers room; the agent tells the room. The first
add — the founder bringing in a second Shaper — passes on the founder's agree alone, because
they are the only Shaper; from then on it takes the rule.

**Removing a Shaper.** Any Shaper opens a **Shapers → remove** proposal naming another. The
named Shaper is not eligible; the rest decide under the _Shapers_ rule. On pass they leave the
set and the room in the same moment; work they hold as DRI is untouched — being a Shaper and
holding work are different relationships. A Shaper can also **step down** on their own, any
time, with no vote. Either way, an org never drops to zero: the last Shaper cannot be removed
or step down without first adding another. The founder is a Shaper like any other and can be
removed like any other; what they keep is Buzz's community ownership, which is administration
of the relay, not a say in the org's decisions.

**Changing the rule.** The rules are visible to everyone on **Overview**, under the Shapers. A
Shaper changes them from there, or by asking the agent in the Shapers room — either way it opens
a **Shapers → rules** proposal showing the current and the proposed rules side by side. It
passes under **all**: nobody's vote is made worth more or less without their agree. The change
applies to proposals opened after it; open ones finish under the rule they started with. The
same proposal sets the decision window and the offer window.

**Choosing the org agent.** Every org starts on the hosted agent — Hypha's, on a Hypha relay;
the operator's, on any other. Shapers who want their own open a **Shapers → agent** proposal
naming the pubkey of an agent they run themselves (the same `buzz-org-agent` binary, their key,
their model). Like a rules change it passes under **all**: whoever drafts for the org is
something every Shaper agreed to. On pass the new agent joins the Shapers room and the old one
leaves it; drafts already open keep their author. Naming the hosted agent again is the way back.
The agent has no authority either way — the choice is about who holds the key and pays for the
model, not what the agent may do.

**Founding.** A new org has one Shaper — the owner — on _majority_ everywhere, which is to say
their own agree. They add a second Shaper when they are ready to be outvoted. No rule needs
setting before then, and the first thing a second Shaper gets on My Work is usually the
direction they were brought in to shape.

**A Shaper can:** agree or decline every decision of the org from one column, and know before
tapping how many more agrees it needs.
**Shapers together can:** bring in a Shaper, let one go, decide how many of them it takes
to decide, and choose which org agent drafts for them.
**Anyone can:** see who shapes and by what rule, on Overview, without asking.

### 2. The org listens

Chats, calls, DMs, and transcripts are the input. Members talk to each other and work in the
open; the org hears needs, gaps, and commitments as they happen.

The org agent is a member of every channel and every DM (Who is who), so nothing said in the
community is outside its reach — and it is searchable and citable later whether or not anyone
tagged it at the time. But being everywhere is not the same as reading everything into an AI.
The agent **listens on its own** in three places: the Shapers room, every project's room, and
your own DM with it — the rooms that exist for the work it drafts. **Everywhere else** — every
other channel, every DM between members — it is silent until someone **tags it**. Tag it in a
group chat and it drafts the ticket you were just discussing; tag it in a DM with a colleague
and it does the same, there, with no invite step. The DM is otherwise unchanged.

Nobody files anything. There is no "new ticket" button — and no "add the agent" button either.

**A member can:** say "who signs the hall licence?" in a project room and trust the org heard
it; say it anywhere else and tag the agent to make it work; and ask the org later about
something that was said in a room they were never in, and get it with the receipt.

### 3. Talk becomes work — with authority

The AI drafts work from what it hears. Who can make it real depends on what it is:

| What                        | Who makes it real                 | Date it carries          |
| --------------------------- | --------------------------------- | ------------------------ |
| **Project**                 | The **Shapers**, by their rule (1a) | **End date**             |
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
   you are the one who confirms or offers. It picks from members' **skills and
   about** (feature 10) and their earlier work, and the card cites both.
4. When a ticket is held, if it names pieces nobody covers, AI drafts those
   **subtickets** onto **Needs your answer** — in the order they can be
   done. If one piece has to be settled first (a permit, a pilot, a
   supplier's yes), that one comes now and the rest wait; when it is done,
   the next pieces arrive shaped by what it found. Each draft says what the
   piece needs and who has it — or that nobody here does yet.

The receipt is the objective or the project description, not a message. It
looks at three moments only: right after a Shaper confirms a new direction
version, right after a project goes live, and on a weekly scan — never per
message. One open suggestion per gap; a dismissed one is not raised again
until something changed.

Anyone can also **publish** from their **DM with the org agent** (the
_Personal Assistant_): draft a direction, project, or DRI-naming proposal
(and, once the treasury lands, a money-out one); create a ticket (for
themselves or someone else); mark their own ticket done; ask the org
anything. The same works in **any DM or channel** by tagging the agent —
it is already there (feature 2). Shapers do the same in the Shapers room.
The agent drafts; a person opens it; the Shapers decide.

A project carries no budget and a ticket carries no pay. If a job should be paid, that is a
conversation between the person taking it and the person offering it, settled where the org's
money lives — Hypha today, a Shaper-controlled contract next (see feature 7).

**A live project has a home.** The moment the Shapers' rule is met, the org gives the project
three things without anyone setting them up: a **room** (`#<project>`) where its talk happens
and feature 5 listens; a **repository** where its work is pushed; and the Buzz **project** that
shows both in the Projects (git) view. The project's DRI is the room's admin and the only one
who lands work on `main`; every ticket holder under it is a member and pushes their branch;
anyone in the community can join the room to read and talk. Nobody creates or owns these by
hand — the org does, and when the DRI changes, the room and the repository follow.

**A Shaper can:** open a project the AI suggested as a proposal, with its end date — in chat;
it goes live when the Shapers' rule is met, with its room and repository. Naming a DRI is a
separate move (offer, or a **project DRI** proposal).
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

The named person sees one clear screen: the job, the dates, who offered it. They accept or
decline. Decline is respected — _not my thing_ or _no room right now_ — and the card goes back
to whoever offered it — a person, or the agent.

**In progress means a DRI.** A ticket that is doing always has a holder. Offered
or waiting can still say nobody yet. If the agent put it out, the ticket
page says **Offered by AI**.

**A member can:** decline work without explaining themselves to the whole org.
**A DRI can:** open a member's profile before offering — skills, about, what they hold now —
and see the same lines the agent matched on.

### 5. Talk moves work

If the **DRI** writes that their ticket is done — in the project room or their DM with the
agent, where it listens on its own, or anywhere else with the agent tagged (feature 2) — the
agent marks it done and posts the receipt: which message, which room. The DRI's own signed
message is the confirm — no card, no waiting window. Anyone else saying it changes nothing; the
agent may nudge the DRI, that is all. A done with open children under it is refused with the
reason.

A done heard **on a call** is different: speaker attribution in a transcript is not a
signature. The agent surfaces it to the DRI — _"you said covers is done — mark it?"_ — and the
DRI's reply is the confirm. Transcripts nudge; they never close.

Done first; other obvious moves (took it, dropped it) later.

**A DRI can:** finish work by saying so where they already talk, and see the board agree.

### 5a. Work reports itself

Some work happens in the platform. Some happens in an editor, on a branch, in a terminal — and
nobody should have to come back and type up what they did. A member who wants that turns on
**Work sync**: their own agent, from the Agents door, one tap, running on their machine.

From a held ticket, **Open in editor** clones the project's repository and opens a branch for
that ticket. The member works — in Cursor, in anything. Periodically, without being asked, Work
sync looks at what moved on that branch, pushes it to the project's repository, and posts a
short **progress note** on the ticket: what changed, in the member's terms, with the commits
behind it as receipts. Anyone can read the note on the ticket page; nobody can fake one — the
relay checks that the commits it names are really in the repository, and that it came from the
holder or the holder's own agent.

Three things follow from the notes, and none of them changes state by itself:

- The **ticket page** carries a **work log**, and the Work board shows when each piece last moved.
- When a note says the work is **ready** — the branch is merged, or the member said so in a
  commit — the org agent puts a **done card** on the member's My Work: _"your branch for
  Booking form is merged — mark it done?"_ **Mark done** or **Not yet**. The card is a
  suggestion; the tap is the close. A note that says **blocked** nudges the holder above.
- The project's **health read** (feature 8a) counts pieces that have gone quiet — no note, no
  move — and names them. Where nobody runs Work sync it says so, rather than reading silence as
  trouble.

Work sync speaks only about work its owner holds, only in summaries and commit ids — never
file contents — and only while the owner leaves it on. A member who never turns it on loses
nothing: done is still a sentence or a button.

**A DRI can:** work in their editor all week and find their ticket already telling the org what
moved — then close it with one tap when the card comes.
**A project DRI can:** open any ticket under them and see the branch, the commits, and the last
note, without asking.
**Anyone can:** see on Work which pieces are moving and which have gone quiet.

### 6. Everyone has a home

Five doors, one glance each. **Overview** is the Org door. In the Hypha desktop they sit in the
sidebar beside Home, channels, Forum, DMs, Agents, and Workflows. The Agents door holds a
member's own agents — Work sync (5a) among them; the org agent is not in it — it lives in DMs and
on cards.

| Door           | What it answers                                                                                                 |
| -------------- | --------------------------------------------------------------------------------------------------------------- |
| **Overview**   | Who are we? Mission, vision, objectives, strategy, established, founder, Shapers and the rule they decide by, the org agent and who hosts it, members, who holds which job. Each direction card opens to its full text, every version, and the proofs behind each line — with receipts. |
| **Work**       | Who is working on what? Every project and ticket, DRI or _open_, dates visible, when each piece last moved; on each project page its room, its repository, and the agent's **health read** (feature 8a); on each ticket the **work log** (feature 5a). |
| **Decisions**  | What the Shapers decide: **Work** (project approval and **project DRI**), **Direction**, **Shapers** (add, remove, rules, agent) — later **Money** (out only) and **Join** (people only). Each card shows agrees so far against the rule. Anyone can read; only Shapers vote. |
| **My Work**    | What needs my tap — including AI cards — what I hold, what I offered (**You offered**), and what is finished. Waiting-on-me stays in the first column. Shapers also see open decision cards here. |
| **My Profile** | Who I am in this community — **about and skills** I wrote myself (the agent and DRIs read them when suggesting or offering work), current work, earlier work, recent decisions; later, what I have been paid. Identity is one keypair across communities; the profile is per community. |

The board door is called **Work**, not _Projects_: Buzz already has a Projects surface for git
repositories, and the two must not share a name. Earlier drafts of these documents say
_Projects_ for the same door.

Empty states say so: _Nothing needs you._

**Anyone can:** see the whole org — who shapes, who holds, what is open — without asking.

### 6a. Join is by invite

In the first version there is exactly one way into an org: **an invite link, or having created
the community**. Any **Shaper** can create an invite link (Buzz's existing invite — a code
with a lifetime and, if they want, a use limit); the community owner and admins can too, as
today. Whoever opens the link and signs in **is a member** — no card, no vote, no waiting.
Nothing lands on them until they accept work. The invite page is where the org says the one
thing a newcomer must know before joining: **the org agent is in every conversation here,
including DMs, and what is said may be cited to any member** (feature 2). It is said there,
once, and not again.

There is no join _request_ in this version: a person without a link cannot ask the org to let
them in from inside the app, and Shapers decide nothing about membership. Orgs do not join
orgs.

**Later — join as a Shaper decision.** When an org wants a door people can knock on, a join
request becomes a **Join** proposal the Shapers decide by their rule, under **Decisions →
Join**, people only, no Recipient. That is where "who may ask to join, and who answers" goes;
it is not in the first version.

**A Shaper can:** make an invite link and send it to someone; they are in when they open it.
**A person can:** join by a link they were given, not otherwise.

### 7. Money is separate from work — and not in the first version

Work flow and money flow are **separate**. No sum lives on a project or a ticket, ever. That
rule holds from the first version; the money flow itself does not ship in it.

**First version: no money in Buzz.** There is no money proposal, no balance on a profile, no
settlement receipt. Payments the org makes are handled where they are today — the existing
Hypha treasury and its proposals — outside this app. A done ticket is a done ticket; whether and
how someone is paid for it is a conversation and a Hypha proposal, not a card here. The
Decisions door has no Money filter until the flow below exists; the Shaper rules (feature 1a)
carry no `money` entry until then.

**Next version: Shapers vote, a contract pays.** Money comes into Buzz as the sixth proposal
kind, and it is settled by a **smart contract**, not by a person:

- The org's funds sit in a contract the Shapers control. Passing a money proposal in Buzz _is_
  the release: the Shapers' agrees are the signatures the contract needs, and when the rule
  is met the contract pays the payee. No one "marks it settled"; the chain receipt is the
  settlement, and Buzz records it next to the decision.
- Money proposals are **out only** — pay or reimburse. Incoming money (a sponsorship, a grant)
  is not a proposal. _"We do not take brand money"_ is **strategy**, not a money decision. A
  grant can land as a ledger fact; it is not something the Shapers vote in.
- How pay will work: when the work is done, the person who did it — or the person above
  them — tells the agent _"draft a proposal for the Shapers for this work — 150 USDC"_. The
  draft carries the done receipt and the sum. The Shapers decide, as with any proposal, under
  the money rule. **Every payment is a Shaper decision** — no sum is small enough to skip
  them, and no DRI approves pay for the people under them.
- Money will live in two places only: **Decisions** (where it moves) and **My Profile** (what
  a person has been paid, as the sum of released proposals with their chain receipts).
- **After that — pay agreed in chat.** The person holding the work and the person above them
  agree a sum where they already talk; the agent remembers that line, so _"…whatever we
  agreed"_ works in the draft request, and if the named sum differs from the agreed one the
  draft shows both. This needs the agent to hear rooms (feature 2) and is scoped after it.

The AI never moves money, in any version. It drafts on request — the Shapers' votes move it.

The reason to keep the treasury out of the first version is not that money is unimportant; it
is that the work loop has to be trusted before a vote in it can release funds, and the
contract path is the right one to build once, rather than a person-settled stopgap first.

**In the first version, a member can:** finish work in Buzz and get paid through Hypha, as
today.
**Next, a member can:** get paid for a done ticket without filling in a form, from a contract
the Shapers released.

### 8. Reviews write themselves

Every project has an exact end date and **closes on it by default**. As the date nears — in the
**last fifth of the project's run**, not on the day, so a two-week project is briefed three days
out and a year-long one ten weeks out — the Shapers get a **brief** and a **recommendation**. The brief is
the story: what was held, what was done, what was not, whether the objective it served moved
(and, once money is in Buzz, what was paid out through proposals).

The recommendation answers one question: _does anything follow this?_ Either the agent drafts a
**follow-up project** — the next piece in the same domain, with a suggested DRI, description, and
end date, because the objective is still live or the brief shows a clear next step — or it says
**no further work in this domain is needed**, with the reasoning and receipts. Extending the
existing project is possible but is the override, not the default.

A Shaper opens the follow-up as a project proposal, or the Shapers move on; either way the
project closes on its date. The
close itself is a **date rule the relay runs** on the end date a Shaper set — not the agent's
judgment, and not a Shaper's tap. A Shaper can override it by setting a new end date. The agent
never closes, extends, or opens a project by itself. What the Shaper chose, and what happened
after, is remembered so the next recommendation is sharper.

**Shapers can:** end a project cleanly on its date and know, before it ends, whether something
should come next.

### 8a. The org reads its own health

Every live project carries the agent's **health read**: one band — _struggling_, _wobbly_,
_healthy_ — and a short paragraph, refreshed weekly and whenever the project's ledger changes.
The band is computed from the ledger (done against elapsed time, overdue pieces, offers nobody
answered, weeks of silence, pieces with no holder, pieces that have stopped moving by their
progress notes (feature 5a), whether the objective it serves moved); the
paragraph is written by the agent, and every sentence points at the ledger rows behind it. The
last sentence names the one thing most pulling the band down, when there is one.

It is a read, not a decision: it lives on the project page, not on a card, and nothing changes
state because of it. Shapers rate the band blind once a week; where they and the agent disagree
is how the read gets better.

**Anyone can:** open a project and see, without asking the DRI, whether it is going well and
what would change that.

### 9. Ask the org anything

"Have we dealt with this before?" gets an answer with receipts — the threads, the decisions,
the outcomes. The org remembers what it tried and what happened, not just what was said. The
answer draws on **every conversation in the community**, DMs included; a receipt may open a
message from a room you were not in, and it opens that one message, not the room.

**Anyone can:** ask at 2am and get the history, the root cause, and who shipped the fix — even
when the fix was agreed between two other people in a DM.

### 10. Newcomers land somewhere real

A newcomer arrives **by invite** (feature 6a). They create their identity through Buzz's own
flow — key, name, picture — the same as any Buzz user; the org adds nothing to sign-up. When
they open the link they land in the community as a member, nothing more. The org agent greets
them in their DM and says what happens next: _"You're in. Nothing is waiting on you. When there
is work that fits, it will be offered — you can say no. Tell me what you're good at and I'll
know who to ask."_

**Skills and about, on the profile.** On **My Profile** a person writes, in their own words,
what they can do and what they like to do — a short **about** and a list of **skills** (_grant
writing_, _Rust_, _hosting events_, _Spanish_) — and, if they want, how much they can take on
at once. This is the one thing the org asks of a member that Buzz does not. The agent reads it
when it names a **suggested holder** (feature 3, chain step 3, and every ticket draft): the
suggestion cites the skill line it matched, next to the earlier work it matched, so _why me_
is answerable from the card. Nothing on the profile is inferred — the person writes it and
edits it whenever they like; the agent may draft an update from what they told it in their DM,
but the person confirms before anything changes. The profile is community-readable, like the
rest of the org; a DRI choosing whom to offer a ticket to reads the same lines the agent does.

No work queue on day one. The path is invited → member → DRI, one accept at a time.

**Later.** A **directory** — a person with no community in mind describes themselves and the
agent suggests orgs that fit, which they then ask to join (the Join decision of 6a). Not in the
first version.

**A newcomer can:** open a link, be in, say what they are good at, and be left alone until
something fits.

---

## What this is not

- Not automation of decisions. Every consequential state change has a human confirm — or a
  date rule a human set. Nothing the Shapers decide passes on one Shaper's word once there are
  two of them, unless they chose that rule together.
- Not a chat product with a bot. The chat is how the org perceives; the loop is the product.
- Not a place with conversations the org cannot see. The agent is in every channel and DM,
  unlisted, silent until tagged outside the rooms it listens to; nothing said here is private
  from the org, and the org says so once, at the door. What it does not do is read every
  message into an AI — presence is not surveillance.
- Not an agent you set up. One org agent per org, present from the day the org exists, hosted
  unless the Shapers choose their own; no persona to pick, no key to paste. Members' own agents
  are theirs — Buzz's Agents door, empty but for the Work sync template — and the org never
  depends on one running.
- Not a tracker fed by hand, and not a tracker that closes itself. Work sync tells the org what
  moved; a person says when it is done.
- Not a payment system. In the first version money stays in Hypha; after that the Shapers'
  vote releases funds from a contract. Buzz never holds funds in either.
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
