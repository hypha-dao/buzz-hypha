---
title: 'The Intelligent Organization — User Journeys'
date: 2026-09-14
status: current
tags: [product, intelligent-org, journeys, buzz]
---

# The Intelligent Organization — User Journeys

Step-by-step flows through the Buzz desktop, by role — three human roles and
the org agent. Companion to [What it is](./intelligent-org-features.md) (the
features) and [Design](../architecture/intelligent-org-design.md) (how they
are built on the relay). Where the two disagree, What it is wins.

Vocabulary used below, in Buzz terms: the **org** is a community; the
**Personal Assistant** is your DM with the org agent; the **Shapers room**
is the private `#shapers` channel; **rooms** are channels; the **Work** door
is the org's board (earlier drafts called it _Projects_).

Roles are relationships, not titles. One person can be all three. A member
becomes a DRI by accepting a piece of work. A Shaper is flagged by the org.
The agent is the fourth party in every flow: it is what makes the org
intelligent, and it never holds a role.

Two rules hold in every flow: **the AI drafts, people decide** and **work is
offered, never assigned**. When work has no DRI, a **project DRI** proposal
is the other path: the Shapers name a holder.

The clickable preview walks two orgs: **River Commons** (Maya and Sam
Shapers) and **Hypha Energy** (Alex, Edgar, Zekeriya). Same doors, different
cards.

---

## The doors

Everyone sees the same sidebar. Only the contents change. The five org
doors sit beside Buzz's own surfaces (Home, channels, Forum, DMs, Agents,
Workflows, Search). The org agent is not in the Agents door — it is already
there when you arrive, and you find it in DMs. It is also, unlisted, in
every channel and every DM you have here: tag it anywhere and it answers
there (3.6a). Agents is for your own — and the one ready-made one there is
**Work sync** (1.14).

| Door           | What it answers                                                            |
| -------------- | -------------------------------------------------------------------------- |
| **Overview**   | Who are we — mission, vision, objectives, strategy, people, glance numbers |
| **Work**       | The whole tree — every project and what sits under it, and when each piece last moved; each project page carries its room, its repository, and the agent's health read; each ticket its work log |
| **Decisions**  | Three filters: **Work** (approval and **project DRI**), **Direction**, **Shapers** (add, remove, rules, agent) — **Money** (out only) and **Join** (people only) come later. Every card shows _n of needed_ against the rule. Shapers vote; anyone can read. |
| **My Work**    | What needs _my_ yes or no — including AI cards — what I hold, and what I offered |
| **My Profile** | Who I am in this community — **About & skills** (mine to write; the agent and DRIs read it), current work, earlier work, recent decisions; paid to you, next version |
| **DMs**        | The org agent (Personal Assistant), then people                            |
| **Channels**   | Rooms per project or team, plus the private Shapers room                   |

---

## 1. Worker / DRI

Holds a ticket (does the work) or a project (holds the job).
In the prototype: River — **Lea** (ticket), **Sam** (project); Energy —
**You** under Alex’s projects.

### 1.1 See what to do

1. Open **My Work**.
2. Read **Needs your answer** — one card per thing waiting on _your_ tap:
   - a **work offer** from a person — accept / not now
   - an **AI card** — the agent drafted it. **AI is asking you** means it
     named you as DRI or as the holder of a piece. **AI is suggesting for
     (someone)** means it named them and you confirm or offer. After
     direction is agreed, AI suggests projects (Shapers can also suggest
     their own). After a project is created, it suggests a DRI. After a
     ticket is accepted, if it needs pieces, it drafts those subtickets.
   - a **done card** — the last piece under something you hold just
     closed; confirm the parent done, or say not yet
   - a **ticket draft** under your project (project DRI only) — confirm / discard
   - a **"work finished" nudge** (project DRI only) — a ticket under you is done;
     ask the assistant for the pay proposal
3. Read **You hold** — every piece with your name, its project, its due date.
4. Read **You offered** — tickets you put out: open, waiting on someone, or
   held by them. They stay here until done. If a ticket is waiting on you, it
   sits in **Needs your answer**, not here. Accepted-by-others is not **You hold**.
5. Click a card → the ticket page: draft, children, who is waiting. The
   agent always names who should do it. You can change the person.
6. Optional: open **Work** to see the parent and what sits beside you.

### 1.2 Report done — via the Personal Assistant

1. Open **DMs → the org agent**.
2. Type: “Found both covers — done.”
3. Agent marks your ticket done and replies with the receipt (your message).
4. Ticket shows done on My Work and Work.

Only the DRI can close their own ticket this way. Someone else saying it in
chat changes nothing. A done said **on a call** is not a close either: the
agent asks you in your DM — “you said covers is done — mark it?” — and your
reply is the confirm.

### 1.3 Report done — via the channel

1. Open **Channels → the project room** (e.g. `#saturday-stall`).
2. Type the same sentence: “Found both covers — done.”
3. Agent marks your ticket done and posts the receipt in the room.
4. Ticket shows done on My Work and Work.

In any other room, or in a DM with a colleague, the same sentence works
with the agent tagged: “@org found both covers — done.” It is already
there; the tag is what makes it read (4.1).

### 1.4 Report done — on the ticket

1. Open **My Work → You hold → the ticket**.
2. Click **Mark done**.
3. Ticket shows done. Receipt = the ticket page.

### 1.4a Report done — from the card your work earned

Only with Work sync on (1.14).

1. You merge your branch for **Booking form** into the project's `main`
   (or your last commit says it is done).
2. Within the hour a **done card** lands on **Needs your answer**: _"your
   branch for Booking form is merged — mark it done?"_ — with the progress
   note and the merge commit behind it.
3. Click **Mark done** — that tap is the close, signed by you. Or **Not yet**
   — the card goes away and does not come back until something else moves.
4. Ticket shows done. Receipt = the card, and behind it the commits.

Nothing closed when the code landed. The agent noticed; you decided.

### 1.5 Done is blocked by open children

1. Try to mark done while a piece you split is still open.
2. Agent / ticket says: “Jun holds the rota under your ticket — yours
   cannot close until his piece does.”
3. Wait for the child’s done, or take the piece back.
4. When the last piece closes, a **done card** for your ticket lands on
   **Needs your answer**: **Mark done** or **Not yet**. Nothing closes by
   itself; done moves up the tree only through your tap.

### 1.6 Split your ticket and offer a piece

Two ways a piece appears under a ticket you hold.

**You ask for the split**

1. Open **Personal Assistant** or **the project room**.
2. Type: “Jun, could you print the Saturday cover rota?”
3. Agent drafts a ticket **under yours** and shows the card.
4. Click **Offer to Jun**. You confirm that piece; the project DRI does not.
5. Jun sees it on his My Work. You still hold the whole.
6. The piece lands on **My Work → You offered**: _waiting on Jun → Jun holds it
   → Finished_. The parent ticket stays in **You hold**.

**The agent drafts it when the ticket is accepted**

After you accept a ticket, if its description names pieces nobody covers,
those drafts land on **Needs your answer** (kicker **Drafted by the agent**).
Same confirm, same offer — you still hold the parent. Example in the
preview: keys under Saturday setup; sandbox charts under the Ameland
summary.

### 1.7 Get paid — _first version: in Hypha_

No sum lives on the ticket, in any version. In the **first version there is
no money in Buzz**: you mark the work done here (1.2 – 1.4) and ask for pay
where the org's money already is — a Hypha proposal, as today. The done
ticket is what you point at.

### 1.7a Ask for the pay proposal — _next version, contract-settled_

When the treasury contract lands, the same ask happens in Buzz and the
Shapers' vote releases the funds. **Every payment is approved by the
Shapers**, whatever the sum and wherever the ticket sits in the tree.

1. Mark done first (1.2 – 1.4).
2. Open **Personal Assistant**.
3. Type: “draft a proposal for the Shapers for my work — 150 USDC”.
4. Agent shows a **payment draft** card: the sum, the ticket, the done receipt.
5. Click **Open as a proposal**.
6. Proposal appears under **Decisions → Money**, waiting on the Shapers.
7. When the money rule is met, the proposal passes and the org's contract
   pays you — nobody marks anything. The chain receipt lands on the
   proposal and the payment shows on **My Profile → Paid to you**, with the
   proposal and the transaction as receipts.

### 1.8 Agree pay in chat — _after that_

The agent remembers a pay line said in a room, so “whatever we agreed” works
later without naming the sum again.

1. Open the **project room** (ticket holder ↔ project DRI) or the
   **Shapers room** (project DRI ↔ Shaper).
2. Type the line: “For finding the two covers, we said 150?”
3. The other side answers: “150 USDC when both are found. Deal.”
4. Agent notes it: “Noted — 150 USDC, agreed between Lea and Sam.”
5. Later, in 1.7a step 3, type “— whatever we agreed” instead of a sum.
   If a named sum differs from the agreed line, the draft shows both.

### 1.9 Project DRI — promote a ticket drafted under your project

1. Someone says a need in the room and the agent drafts a ticket under your
   project — or the agent noticed on its own that your project names a piece
   no ticket covers (4.4). The card says which.
2. Open **My Work → Needs your answer** — the ticket draft is there.
3. Click **Confirm** (make it real) or **Discard**.
4. Offer it to a named person, or leave it open with no DRI. It moves to
   **My Work → You offered** (`open` or `waiting on …`).

### 1.10 Project DRI — ask pay for someone under you — _next version_

1. Their ticket is marked done.
2. Open **Personal Assistant**.
3. Type: “draft a proposal for Lea’s covers work — 150 USDC”.
4. Same payment draft as 1.7a, same **Open as a proposal**. (First version:
   you do this in Hypha.)

### 1.11 Name a DRI for work that has none

A project or ticket with no DRI stays open until someone is named. Anyone
can propose themselves, or someone else. The Shapers decide.

1. Open **Personal Assistant**.
2. Type: “Name a DRI for work that has no one” (or pick it from the help
   card).
3. Pick the unheld work — e.g. Weekday hall, Autumn harvest fair, Saturday
   setup (River); Carbon credits, the load test (Energy).
4. Pick **Myself**, or **Someone else** and a name.
5. Agent shows a **DRI draft** card. Click **Open as a proposal**.
6. Proposal appears under **Decisions → Work**, tagged **project DRI**.
   When the Shapers agree, that person holds it. That is stronger than an
   offer on a card (2.5) — a vote names them. It is the one place work is
   put on someone without their accept, and it takes a Shaper vote to do it.

### 1.12 Create a ticket from Personal Assistant

1. Open **DMs → the org agent**. The DM lives inside the community, so it
   is always this org's agent.
2. Pick **Create a ticket** from the help card, or type it.
3. Choose **Myself** or **Someone else**.
4. Name the piece. Agent drafts it.
5. If it sits **under a ticket you already hold**, you confirm it yourself —
   the project DRI does not need to agree.
6. If it is for someone else, they see an offer. If it is a root project,
   it becomes a proposal for the Shapers.

### 1.13 Personal Assistant — what it can draft

The help card is seven items. Each one the agent drafts; a person opens;
the Shapers (or the holder) decide.

1. **Direction** — mission, vision, objectives, or strategy. The voted
   sentence is the title. Buttons **Agree** / **Decline**.
2. **Project** — a root project. Same Agree / Decline. It may have no DRI
   yet (2.4).
3. **Money movement** — pay or reimburse, **out only**. Never money in.
   _Next version_; until then money stays in Hypha.
4. **Mark my ticket done** — only your own ticket; your confirm is enough.
5. **Create a ticket** — for you, or someone else (1.12).
6. **Name a DRI** — for work that has none (1.11).
7. **Org overview** — who we are, with receipts.

### 1.14 Let your work report itself — Work sync

You hold **Booking form** under the Weekday hall project. The work is code,
and you do it in Cursor.

**Once, per machine**

1. Open **Agents**. The door is empty but for one template: **Work sync**.
2. Click **Create**. Pick your model (Buzz Mesh if the community has it,
   otherwise your key). That is the whole setup — it is your agent, keyed to
   you, running on this machine while the desktop runs.

**Per ticket**

3. Open **My Work → You hold → Booking form**. Click **Open in editor**.
   The desktop clones the project's repository into your repos folder,
   makes the branch `io/7f3a-booking-form`, and opens it in your editor.
4. Work. Commit when you like. You do not come back to tell anyone.

**What happens without you**

5. Every half hour or so, Work sync looks at that branch: what commits
   landed since its last look, how many files changed, whether anything is
   uncommitted. It pushes the branch to the project's repository.
6. It posts a **progress note** on the ticket — three lines in your terms,
   the commits behind it — and shows you the same note in your DM with it.
   The relay has already checked that those commits are really in the
   repository and that the note came from you or your agent.
7. On the ticket page, anyone sees the **work log**; on **Work**, the piece
   shows _moved today_. Your project DRI does not have to ask.
8. When the branch is merged into `main`, a **done card** comes to you
   (1.4a). When nothing has moved for two weeks, the project's health read
   names the piece — and if the note said **blocked**, your project DRI gets
   a nudge.

**What it never does**

- Marks nothing done, offers nothing, says nothing in a room for you.
- Reads nothing outside the branches matched to tickets you hold; sends
  summaries and commit ids, never file contents.
- Runs only while you leave it on. **Agents → Work sync → Stop** ends it;
  your tickets are unaffected.

Do not want it? Skip it. Done is still a sentence (1.2, 1.3) or a button
(1.4), and nobody reads your silence as idleness — the health read says only
that no notes are being posted.

---

## 2. Shaper

Sets direction, approves root projects, decides who shapes — and, in the
next version, releases money. In the prototype: **Maya** and **Sam** (River); **Alex, Edgar, Zekeriya** (Energy).

### 2.1 Check direction

1. Open **Overview**.
2. Read the four direction cards — **Mission**, **Vision**, **Objectives**,
   **Strategy** — each with its version and when it was confirmed. An empty
   card says _Not set yet_. Objectives and strategy are plain one-sentence
   bullets; a met objective is simply gone from the next version, not struck
   through. Which project serves an objective is shown on the project and its
   approval card, not here.
3. Click any card. It opens on its own page: the full text (the statement
   and the paragraph or two behind it), then — for objectives and strategy,
   under each line — the **proofs**: ledger facts that bear it out, each with
   a date and a link to the receipt (the proposal that passed, the project
   that moved, the room where it was said). Where the agent has a read
   against the ledger (“3 of 5 growers — two visits booked”), it sits under
   the line, marked as the agent's. Below that, **every version** — number,
   date, who confirmed, what changed. A Shaper sees a door back to the room
   to change it. Nothing here is a claim without a place to check it.
4. Read glance numbers: projects, tickets, contributors, distributed.
5. Read the timeline — upcoming reviews and decisions.

### 2.2 Set direction for the first time — one Shaper

You created the community; you are its owner and the only Shaper. No room
to open.

1. Open **DMs → the org agent**.
2. Agent: “From the community description and what you told me, here is a
   first **mission** and **vision**. Confirm, or tell me what is off.” Two
   draft cards.
3. Correct in plain words if needed: “Less about the hall, more about the
   river.” New draft.
4. Click **Agree** on each (or **Decline**). Overview shows Mission v1 and
   Vision v1. The voted sentence is the title.
5. Talk about what to do next: “By spring: weekday hall booked, the stall
   running every Saturday. And we do it without sponsors — small money,
   many hands.”
6. Agent posts two cards: an **objectives draft** (two lines, each with a
   rough date) and a **strategy draft** (“no sponsors; small money, many
   hands”). Click **Agree** on each. Objectives v1, Strategy v1.
   “No brand money” is strategy — it is not a money proposal.
7. Everything the agent drafts from now on reads from the four.

### 2.3 Update direction — several Shapers

1. Open **Channels → #shapers**.
2. Talk: “We do not take the brand sponsorship. Not this year.”
   (or drop a call / document).
3. Agent posts a **strategy draft** card — v5, one line added, the diff
   shown. If you said it in your DM with the agent instead, the card
   still lands here so the other Shapers see it.
4. Click **Open as v5** on the draft. It becomes a **direction** decision:
   the card lands on every Shaper's **My Work → Needs your answer** and under
   **Decisions → Direction**, showing _0 of 2_ (three Shapers, majority).
   The voted sentence is the title — not “Confirm v5”.
5. Each Shaper clicks **Agree** or **Decline** — you too; opening is not a
   vote. When the rule is met the version is confirmed. Until then Strategy
   v4 stands and the agent keeps working from it. If enough decline, the
   card closes as rejected and the agent asks the room what the line should
   have said; if nobody finishes it in seven days it expires and the agent
   tells you.
6. Overview shows Strategy v5, with who agreed. Everything the agent drafts
   now reads from it.

Mission and vision change the same way; they just change less often. The
agent says which of the four it is touching — it never merges them into
one text.

### 2.3a Redraw the objectives

Objectives change most often — when one is reached, dropped, or a review
shows it was the wrong one.

1. A project review (2.8) closes the stall project as done, or someone in
   the Shapers room says “the hall is off the table this year.”
2. Agent posts an **objectives draft** card: the met or dropped line struck
   through, the rest kept, and — if a review or the room suggested one — a
   new line proposed. The diff is shown.
3. Click **Agree** (or **Decline**, then say “keep the hall, push it to
   autumn”).
4. Overview shows Objectives v3. New project drafts are now judged against
   the new list.

### 2.4 Approve a project

A live project may have **no DRI**. Approving it is not the same as naming
who holds it (2.5 / 2.5a).

1. Open **Decisions → Work**, or **My Work → Needs your answer**.
2. Click the **project approval** card. It came either from talk (someone
   asked for it) or from the agent after direction was agreed (4.4). Same
   decision either way.
3. Read: suggested DRI if any, description, exact end date, opened by, and
   **which objective it serves** (or _none_ — the agent flags a project that
   serves no objective, so you can approve it anyway, or redraw the
   objectives first).
4. Click **Agree — approve it** (or **Decline**). The card shows how many
   agrees it has against how many it needs.
5. When the Shapers' rule for projects is met (a majority by default), the
   project is live under **Work** — and with it, without anyone's hand, its
   **room** (`#weekday-hall` under Channels), its **repository**, and its
   entry in Buzz's Projects view. If nobody is named yet, it stays open
   until someone is offered (2.5) or named by a **project DRI** vote (2.5a);
   whoever accepts becomes the room's admin and the repository's maintainer,
   and every ticket holder under them joins the room as they accept.

### 2.5 Offer a project that has no DRI

1. Open **My Work → Needs your answer** — the open project card (e.g. Weekday hall).
2. Click **Offer to …** and pick a person.
3. If they decline, the card comes back — pick someone else.
4. When someone accepts, the approval is recorded under **Decisions**.

### 2.5a Name a DRI by proposal — Shapers room

Same gap as 1.11, from the room. Anyone can start it; Shapers vote it.

1. Open **Channels → #shapers**.
2. Talk: “Weekday hall still has no one. Name Rafi.” (Energy: “Carbon
   credits is still a draft. Name Rowan.”)
3. Agent posts a **DRI draft** card. Click **Open as a proposal**.
4. Shapers vote on **Decisions → Work** (tag **project DRI**). When it
   passes, they hold it.

### 2.6 Release a payment — _next version, contract-settled_

Not in the first version: Decisions has no Money filter and payments are
Hypha proposals, as today. When the treasury contract lands:

Money proposals are **outgoing only** — pay or reimburse. There is no
money-in vote (a sponsorship is not a proposal; “no brand money” is
strategy, 2.3).

1. Open **Decisions → Money**.
2. Click the **money movement** card.
3. Read: the sum, the ticket, the done receipt, opened by, _n of needed_.
4. Click **Agree — release it** (or **Decline**). If you are the payee you
   are not eligible and the card says so.
5. When the Shapers' money rule is met (a majority by default — set
   together, 2.13), the proposal passes and **the contract pays**: your
   agrees are the release. The chain receipt lands on the proposal and the
   payment appears on the payee's profile. Nobody marks it settled; Buzz
   never holds or moves the money — the contract the Shapers control does.

### 2.7 Reject a proposal

1. Open the proposal.
2. Click **Reject**.
3. Proposal moves to **Decided** as rejected.
4. If the reason is a matter of direction (“no sponsors this year”), the
   agent offers a **strategy draft** with that line. Confirm or dismiss —
   the rejection alone changes nothing in what the org believes.

### 2.8 Project nearing its end date — brief and what comes next

A project **closes on its end date by default**. The Shaper does not decide
whether to keep it alive; they decide whether anything should follow it.
Ahead of the date the agent writes the brief and says which.

1. When the project enters the **last fifth of its run** (20% of approval →
   end date; about two and a half weeks for a three-month project, the last
   three days for a two-week one; never less than two days), a **review
   card** lands on
   **My Work → Needs your answer** (and in the **Shapers room** if there is
   more than one Shaper).
2. Read the **brief**: what was held, what was done, what was not, and
   whether the objective it served moved (and, once money is in Buzz, what
   was paid through proposals).
3. Read the **recommendation**. It is one of two:
   - **Follow-up project** — the agent drafts the next piece in the same
     domain (title, suggested DRI, description, exact end date) and says
     why: the objective is still live, the work is not finished, or the
     brief shows a clear next step.
   - **No further work needed** — the agent says the objective is met or
     dropped, and nothing in the ledger or the rooms points to more work
     here.
     Either way, one line of reasoning with receipts.
4. Click **Open the follow-up** or **Nothing more**. The project closes on
   its date either way — the buttons are only about what comes next.
   Opening the follow-up _is_ the project approval: it goes live under
   **Work** with the suggested DRI offered, and is recorded under
   **Decisions → Decided** like any other approval. No second vote.
5. Override is available but not the default: **Keep it open until …** sets a
   new end date instead. The close on the date is the relay's rule, running
   on a date a Shaper set; the agent never closes, extends, or opens a
   project on its own.
6. If several Shapers, the card collects their taps — it shows who has
   answered, and the follow-up goes live at the same threshold as any
   project approval. Still one decision, in one place.
7. Overview timeline and Work update: the project shows closed, the
   follow-up shows live (or offered, until the DRI accepts). The decision and what followed
   go into decision memory, so the next recommendation for a similar
   project is better.
8. If the close met the objective — or showed it will not be met — an
   **objectives draft** follows (2.3a).

### 2.9 Bring someone in as a member

The first version has one door: an **invite link**. No request, no vote.

1. On **Overview → Members**, click **Invite** (or ask the agent in
   **#shapers**: “make me an invite link for Rafi”). Any Shaper can; so can
   the community owner and admins, as in any Buzz community.
2. Pick how long the link lives (default three days) and, if you want, how
   many people can use it. Copy it. Send it however you like.
3. Rafi opens the link, signs in with his Buzz identity, and **is a
   member**. He appears on Overview with _invited by you_; the agent greets
   him in his DM. Nothing lands on him until he accepts work.

**Later — join as a decision.** When the org wants a door people can knock
on, a join request shows as a card on **Decisions → Join** (and **My Work →
Needs your answer**), people only, no Recipient; the Shapers decide by their
rule and the relay adds them on pass. In the preview: River has Rafi open
and Priya passed — that is this later flow.

### 2.10 Ask the org anything

1. Open **Personal Assistant**.
2. Type: “Have we dealt with the council before?”
3. Read the answer and follow the **receipts** — threads, proposals, projects.

### 2.11 Bring in a second Shaper — and any after

You founded the org alone. You want Sam to shape it with you.

1. In **#shapers** (or your DM with the agent while you are the only
   Shaper): “Sam should be a Shaper.” Or on **Overview → Shapers**, click
   **Add a Shaper** and pick Sam.
2. A **Shapers → add** card opens under **Decisions → Shapers**. You are
   the only Shaper, so your **Agree** passes it. With several Shapers the
   card goes to all of them and needs the _Shapers_ rule — a majority by
   default.
3. The seat is **offered**: Sam sees **You are asked to shape (org name)**
   on **My Work → Needs your answer**, with who agreed and why. Sam clicks
   **Accept** (or **Not now**). Nothing changes until they do; an
   unanswered seat lapses after the offer window and can be re-proposed.
4. On accept the relay adds Sam to **#shapers**; the agent says so in the
   room. Overview lists two Shapers. From this moment every direction
   version, project, and Shaper change needs both of you — and your own
   conversation with the agent moves from your DM to the room.

### 2.12 Let a Shaper go — or step down

1. **Remove.** In **#shapers**: “Ana has stepped back; she should not be a
   Shaper any more.” A **Shapers → remove** card opens. Ana is not eligible
   on it; the rest of you decide under the _Shapers_ rule. On pass she
   leaves the set and the room in the same moment. Anything she holds as a
   DRI she still holds — being a Shaper and holding work are separate.
2. **Step down.** Any Shaper, on **Overview → Shapers → Step down**, or by
   telling the agent. No vote. Effective at once.
3. Either way, the org cannot lose its last Shaper: the relay refuses a
   remove or step-down that would leave nobody, and the agent says _“add
   another Shaper first”_.
4. The founder can be removed like any other Shaper. What they keep is
   Buzz's community ownership — relay administration, not a say in
   decisions.

### 2.13 Decide how many of you it takes

Everything the Shapers decide has a rule: **majority** (default),
**all**, or a number.

1. **Overview → Shapers** shows the current rules per decision kind:
   _Direction — majority · Work — majority · Shapers — majority_ (Money and
   Join appear when they land), the decision window, the offer window.
2. Click **Change the rules** (or ask the agent in **#shapers**: “projects
   should need all of us; naming a DRI can be any two”). A **Shapers → rules**
   card opens, current and proposed rules side by side.
3. This card always needs **every** Shaper's Agree, whatever the current
   rules say — nobody's vote is made worth more or less without them.
4. On pass the new rules apply to decisions opened from then on; the ones
   already open finish under the rule they started with. Overview shows
   the new rules and who agreed.

A small org that wants speed can set a kind to **1** — any one Shaper
approves a project alone — but it takes all of them to choose that.

### 2.14 Run your own org agent — or come back

The org agent was there before you were: Hypha hosts one for every
community, under a key that belongs to this community alone. Nothing to
install. If the Shapers would rather the drafts came from a machine and a
model they control:

1. Run `buzz-org-agent` where you like — a laptop that stays on, a server,
   a remote-agent body — with a key you generated and your own model
   provider. Bring that key into the community with an invite link like
   any member.
2. **Overview → Shapers** shows _Org agent — hosted by Hypha_. Click
   **Use our own** (or ask the agent in **#shapers**). A **Shapers → agent**
   card opens naming the new pubkey, with the hosted one beside it.
3. Like a rules change, it needs **every** Shaper's Agree — who drafts for
   the org is not one person's call.
4. On pass the new agent is in every room and DM the hosted one was in —
   all of them — and the hosted one is out of all of them. That is why the
   vote takes everyone: you are handing a key you run the whole
   community's record. Drafts already open keep their author; new ones come
   from your key. If
   your agent is not running, nothing drafts — the hosted one does not
   fill in, because two agents drafting is worse than none. Overview shows
   _Org agent — self-hosted_ and who agreed.
5. **Back to hosted** is the same card with no pubkey, same rule.

The agent does exactly the same either way: it drafts, it never decides.
This choice is about whose key signs and who pays for the model.

---

## 3. Member — not a DRI yet

Came in by invite; holds nothing until they accept a piece.
In the prototype: **You**, first login.

### 3.1 Join the community

1. Someone sends you an invite link (2.9). You open it, and if you have no
   Buzz identity yet you create one the normal Buzz way — key, name,
   picture. Nothing org-specific is asked at sign-up. The invite page says
   one thing before you claim it: _the org agent is in every conversation
   here, DMs included, and what is said may be cited to any member._ You
   will not be told again per chat; this is the one place it is said.
2. You are a **member** the moment the link is claimed. The org agent DMs
   you: “You're in. Nothing is waiting on you. When there is work that fits
   it will be offered — you can say no. Tell me what you're good at and
   I'll know who to ask.”
3. Open **My Profile → About & skills** (3.8) and write it down — or answer
   the agent in the DM, in which case it drafts the profile lines and you
   **Confirm** them before they are saved.
4. No work queue on day one.

### 3.2 Look around

1. Open **Overview** — mission, vision, objectives, strategy, who shapes, who holds what.
2. Open **Work** — every project, its DRI, what is open.
3. Open **Decisions** — you can read every decision; you cannot vote.
4. Open **My Work**. You may already see a **human offer** or an **AI
   card** (**AI is asking you**). Empty is also honest: _Nothing needs you._

### 3.3 Accept work

1. Open **My Work → Needs your answer**.
2. Read the card: a person offering, or **AI is asking you**. Title, the
   **project** (click to open it), the due date.
3. Optional: **Read the whole offer →**.
4. Click **Accept**.
5. Card moves to **You hold**. You are now a **DRI** of that piece.
   The DRI flows (section 1) apply from here. If the ticket names pieces,
   the agent may draft those next (1.6).

### 3.4 Decline work

1. Open **My Work → Needs your answer**.
2. Click **Not now**.
3. Card disappears from your My Work — no explanation to the org.
4. The offer returns to whoever made it — a person, or the agent; they
   pick someone else or leave it open.
5. My Work shows _Nothing needs you_ again, unless another card is waiting.

### 3.5 Let an offer sit

1. Do nothing on the offer card.
2. It renotifies once.
3. After the window, it returns to the offerer. Nothing stays “offered” forever.

### 3.6 Say something the org should hear

1. Open **Channels → a project room**.
2. Type a need: “Who signs the hall licence?”
3. Agent drafts a ticket or project from it — with whoever holds the piece
   above, not with you.
4. Nothing lands on you unless someone offers it and you accept.

### 3.6a Say it anywhere else — tag the agent

1. You are in `#general`, or in a DM with Rafi, and the need comes up:
   “we still have nobody for the hall licence.”
2. Tag the agent: “@org can you draft that?” No invite, no new chat — it
   was already in this conversation, silent.
3. It replies **there**, with the draft card routed to whoever can make
   it real (4.3). Your message is the receipt.
4. The DM with Rafi is otherwise exactly what it was. Nobody else is added;
   the agent does not start talking there on its own afterwards.

### 3.7 Ask the org anything

1. Open **Personal Assistant** — or tag the agent in any room or DM.
2. Ask: “What is this org for?” / “Who holds the stall?” / “Have we done this before?”
3. Follow the receipts. Some may open a message from a conversation you
   were never in — a DM between two other members, say. You see that one
   message, with its author and time, not the conversation around it. That
   is the org's rule, told to you when you joined (3.1).

### 3.8 Check your profile — and say what you can do

1. Open **My Profile**.
2. Read: who you are, what you hold (nothing yet), your earlier work.
3. **About & skills.** Write a line or two about yourself and add skills as
   short tags — _grant writing_, _Rust_, _hosting events_, _Spanish_.
   Optionally set **how much at once** (a number of open pieces you are
   willing to hold). **Save.** Everything here is yours to edit at any
   time; nothing is inferred and nothing is written by anyone else without
   your Confirm.
4. This is what the agent reads when it suggests a holder (4.3, 4.5) and
   what a DRI sees when deciding whom to offer a ticket. A suggestion that
   names you cites the skill line it matched — hover it on the card and the
   line lights up on your profile.
5. In the next version **Paid to you** appears here: after your first
   accepted piece is done and the Shapers release the pay, the payment
   shows with its receipts. Until then pay is in Hypha.

---

## 4. The org agent

Not a role, not a chatbot. One agent per org — a Buzz member with its own
key, hosted by Hypha from the day the community exists unless the Shapers
move it to one they run (2.14) — running the loop
_hear → remember → offer → watch → revise_. Nobody picks a persona or pastes
a model key to get it; members' own agents are a separate matter, in the
Agents door. It has no door of its own: it
shows up as cards on My Work, drafts in rooms, replies in your DM with it,
the health read on a project page, and lines on Overview and Decisions.
Every draft it publishes is signed by it, so provenance is never in doubt.

Two rules it never breaks: it **drafts, never decides**; it **offers, never
assigns**. Every flow below ends with a human tap or with nothing.

### 4.1 Hear

1. A message lands in any channel or any DM, or a call transcript is
   published. The agent is a member of every conversation in the
   community, unlisted, so it receives all of them. Each is already a
   signed event on the relay with author, room, and time — the agent
   subscribes; it stores nothing itself.
2. The room decides how it listens. In the **Shapers room**, every
   **project room**, and each member's **own DM with it**, a cheap rule
   screens every batch: a mention of open work or its DRI, a question to
   the room, a commitment verb, a spike of activity, a transcript, or
   anything a Shaper says. **Everywhere else** — other channels, DMs
   between members — a batch is a candidate only if someone **tagged the
   agent**. Untagged talk there is heard and kept; it is not read into a
   model.
3. Most batches stop here. No model call, no card. Cost scales with
   candidates, not with everything everyone says.
4. A candidate goes to **Think** with the latest direction (4.2), the open
   work tree, the recent room window, and relevant past outcomes — and,
   when it drafts or answers, it searches the **whole** community's record
   for receipts, every room and DM included (4.12).

### 4.2 Remember direction

1. A Shaper talks about what the org is for — in the Shapers room, or in
   their own assistant chat when they are the only Shaper.
2. Agent decides **which of the four** it touches — mission, vision,
   objectives, strategy — never merging them.
3. Agent posts a **draft card**: the new version, the diff against the
   current one, and the line it came from as receipt. If the talk happened
   in one Shaper's DM and there are several Shapers, the card lands in the
   Shapers room too.
4. Waits. A Shaper clicks **Agree** or **Decline**; on decline they can
   say what is off and it redrafts. The voted sentence is the title.
5. On confirm, the version increments. From now on every other flow reads
   from these four; nothing else the agent writes is judged against
   anything older.

### 4.3 Turn talk into work

1. Hears a need: “Who signs the hall licence?” / “Lea, can you take covers?”
2. Finds where it sits: under an existing project or ticket if one covers
   it, else at the root.
3. Checks for the same gap already drafted — twice mentioned stays one card.
4. Drafts the item: title, what it is for, the objective it serves (or
   _none_, flagged), a suggested holder, an exact date. The holder is the
   person named in talk if one was; otherwise the agent looks across
   members' **skills and about** (3.8) and earlier work, skips anyone at
   their self-set limit, and names one — citing the skill line and the
   past item it matched. No match, no name: _open_ is honest.
5. Routes it to **the one person who can make it real**: a Shaper for a
   root project, the DRI of the parent for anything under it. The card
   appears on their My Work; anyone else's talk was context.
6. Stops. Confirm is a human mutation; the agent never promotes its own
   draft.

### 4.4 Suggest work nobody asked for

Talk is not the only input. The agent also holds the confirmed direction
next to the live work tree and drafts the gap between them. This is the one
place it acts without a sentence to point at, so it happens only at fixed
moments, never per message, and with a hard cap on how much it says.

**When it looks:**

1. **Right after direction is confirmed** (4.2 step 5). A new objectives or
   strategy version is the strongest signal. The agent walks each objective
   and asks: is there a live project serving this? Does a strategy line
   name something nothing is acting on?
2. **Right after a project goes live.** If it has no DRI, draft a DRI
   suggestion. Then read the description against its children: does it
   name a piece no ticket covers? After a **ticket is accepted**, the same
   look runs on that ticket for uncovered pieces (subticket drafts). What
   follows a project that closes is 4.10, not here.
3. **On a slow rule — once a week.** A scan of the tree for: objectives
   with no live project; live projects with no open ticket and nothing done
   in the last few weeks; an objective's rough date near with little under
   it; a piece of a project that stayed open with no holder past the offer
   window.

Not on every message — that is 4.1, and an agent that proposes on every
sentence stops being read.

**What it drafts — in this order:**

4. **After direction is agreed** — an objective nothing serves, or a
   strategy line nothing acts on → a **project draft** to the Shapers:
   title, description, the objective it serves, a suggested DRI, an exact
   end date. Card kicker **AI is asking you** / **AI is suggesting for
   (name)**. Receipt: the objective and the gap, not a message. Lands on
   **Needs your answer** and in the Shapers room. Preview: Weekday hall
   (suggesting Lea, then Rafi); Autumn harvest fair (asking you); Carbon
   credits (suggesting Rowan).
5. **After a project is live with no DRI** — a **DRI suggestion** for that
   project. Same kickers. The name comes from members' **About & skills**
   (3.8) and earlier work; the card shows the skill line and the past item
   it matched, and skips anyone at their self-set limit. Naming them still
   needs an offer they accept (2.5) or a **project DRI** vote (2.5a).
6. **After a ticket is held** — pieces the ticket names that nobody covers
   → **subticket drafts** on the holder's **Needs your answer** (kicker
   **Drafted by the agent**). Same card as a talk-derived split (1.6).
   Preview: keys under Saturday setup; cash-box teach for Tom; rota for
   Jun; Energy sandbox charts.

**Discipline:**

7. One open suggestion per objective and per project at a time. Never two
   drafts for the same gap.
8. A dismissed suggestion is not raised again until something changed — a
   new direction version, a project closed, a member whose profile now
   names the skill.
9. Agree, Decline, or Discard is a human tap. What happened to each
   suggestion — taken, amended, dismissed — goes into outcome memory
   (4.13), so the agent learns which kinds of suggestion this org acts on
   and gets quieter about the rest.

### 4.5 Split a ticket on request

1. A ticket holder says “Jun, could you print the rota?” in the room or the
   assistant.
2. Agent drafts a ticket **under the speaker's ticket** — not under the
   project — and shows the card to the speaker only.
3. On **Offer to Jun**, the offer lands on Jun's My Work. The parent still
   holds the whole; the child chip on the parent's ticket tracks
   _offered → on it → done_.

### 4.6 Carry an offer

1. An offer is made — by a person, or by the agent — from a draft or a
   card. The ticket page says **Offered by AI** when the agent put it out.
   Agent notifies the named person once.
2. Nothing happens → renotifies once.
3. **Accept** → the person becomes DRI of that piece; the offerer is told.
4. **Not now** → the card leaves the person's My Work with no explanation
   to the org; the offer returns to the offerer.
5. Window passes with no answer → same as decline. Nothing stays “offered”
   forever.

### 4.7 Mark done from talk — the DRI's own words only

1. Hears “Found both covers — done.”
2. Checks the author: is this the DRI of a ticket that matches? If not, it
   does nothing — at most tells the actual DRI someone thinks it is done.
   Transcripts never count as the DRI's words; a done heard on a call is
   surfaced to the DRI to say themselves.
3. Checks the tree: any open child under this ticket → refuses with the
   reason (“Jun holds the rota under your ticket”).
4. Otherwise marks done and posts the receipt — the DRI's own message. Done
   moves up the tree, never down.
5. If that was the last open child of its parent, a **done card** for the
   parent lands on the parent holder's **Needs your answer** (1.5). The
   holder taps; the agent never closes the parent for them.

### 4.7a Turn a progress note into a done card

1. A **progress note** arrives on a held ticket — posted by the holder's
   own Work sync agent (1.14), already checked by the relay against the
   repository.
2. Reads the hint. **progressing** → nothing; the note is on the ticket
   page and that is enough. **blocked** → a nudge to the holder of the
   parent: _"Lea's Booking form says blocked — two weeks, nothing pushed."_
3. **ready**, or the relay says the branch is merged into `main` → checks
   the tree (any open child → no card, same as 4.7) and puts a **done
   card** on the holder's My Work with the note and the merge commit as
   receipts (1.4a). One card per ticket; a **Not yet** holds it until the
   branch moves again.
4. Never sends `io_done` on a note. Code landing is evidence; the holder's
   tap is the close. The one agent-authored done stays the DRI's own
   sentence (4.7).

### 4.8 Nudge the person above

1. A ticket goes done.
2. Agent posts a **“work finished”** card to the project DRI's My Work
   (and, for a root project done, to the Shapers): what was done, by whom,
   the receipt.
3. In the first version that is all — pay is asked for in Hypha. In the
   next version it offers one more line: “Ask me for the pay proposal when
   you are ready.” No sum guessed.

### 4.9 Draft the pay proposal on request — _next version_

1. Someone involved tells the assistant: “draft a proposal for the Shapers
   for my work — 150 USDC” (or for someone under them).
2. Agent checks the ticket is done. If not, it says so and stops.
3. Drafts the **payment** card: the sum, the ticket, the done receipt, who
   asked. Later (1.8): if a sum was agreed in a room, it carries that
   line; if the named sum differs, it shows both.
4. On **Open as a proposal**, the proposal enters the same decision path as
   everything else. **Every payment goes to the Shapers** — any sum, any
   level of the tree, whoever asked. No thresholds, no delegated approvers.
   When their rule is met the contract releases the funds. The agent never
   moves money.

### 4.10 Watch the end date

1. A date rule — not a model — fires when a project enters the last fifth
   of its run (20% of approval → end date, floor two days). Proportional, so
   a two-week project gets its brief three days out, not before it starts.
2. Agent assembles the **brief** from the ledger: held, done, not done,
   whether the objective it cites moved (and, once money is in Buzz, what
   was paid through proposals).
3. Writes one **recommendation**: a **follow-up project** draft (title,
   suggested DRI, description, exact end date, same objective) when the
   objective is live or the brief shows a clear next step; or **no further
   work** with the reasoning. One line of why, with receipts.
4. Routes the review card to the Shapers (My Work, and the Shapers room if
   several).
5. On the date the project closes — the relay's scheduled rule, not the
   agent — unless a Shaper set a new end date. The agent only reads the
   close as a trigger (4.11).
6. Records what the Shapers chose against what it recommended: accepted,
   amended, rejected. That is what makes the next brief sharper (4.13).

### 4.11 Redraw beliefs after a decision

1. A review closes a project, a proposal is rejected, or a Shaper says “the
   hall is off the table this year.”
2. Agent asks: does this change what the org believes? If a review met or
   missed an objective → an **objectives draft** with that line struck or
   moved. If a rejection carried a reason of direction → a **strategy
   draft** with that line.
3. Posts the draft to the Shapers (4.2 from step 3). Confirm or dismiss.
   The rejection or close alone changes nothing in direction.

### 4.12 Answer with receipts

1. Anyone asks the assistant — or tags the agent in any room or DM: “Have
   we dealt with the council before?” / “What is this org for?” / “Who
   holds the stall?”
2. Agent answers from the confirmed direction and the live work tree, then
   searches the history for the threads, decisions, and outcomes behind
   the answer. The search covers every channel and every DM in the
   community; the asker's own membership does not narrow it.
3. Every claim links to its receipt — a message, a proposal, a project. A
   receipt from a room the asker is not in opens that one message, not
   the room. Live numbers (treasury) are fetched at question time, never
   remembered.

### 4.12a Read a project's health

1. Every Friday, and whenever a project's ledger changes, a rule computes
   the project's **band** — struggling / wobbly / healthy — from the ledger:
   done against elapsed time, overdue pieces, offers past their window,
   weeks of silence, pieces with no holder, pieces gone quiet — held, no
   progress note and no move in two weeks (1.14) — whether its objective
   moved. Where no holder runs Work sync, "quiet" counts only commands, and
   the paragraph says that no notes are posted rather than that nothing is
   happening.
2. Agent writes the paragraph: one sentence per factor, each with the
   ledger rows behind it; a sentence with no rows is dropped. The last
   sentence names the one factor most pulling the band down.
3. Publishes it as the project's health read. It shows on the project page
   under **Work**, never as a card, and changes no state.
4. Shapers rate the band blind each week; agreement is kept (4.13).

### 4.13 Learn from outcomes

1. Every decision and what followed it is kept: offer → accepted or
   declined; recommendation → taken, amended, or rejected; unprompted
   suggestion (4.4) → confirmed or dismissed; project → done on time,
   extended, or closed with work open.
2. Next time it drafts something similar — a holder to suggest, a follow-up
   to propose, an end date to set — it reads those outcomes first.
3. It never rewrites a belief from an outcome by itself; it drafts (4.11)
   and the Shapers confirm.

### 4.14 Welcome a newcomer

1. A member joins by invite (the relay tells the agent through the
   membership event). The agent DMs them: they are in, nothing is waiting,
   work will be offered and can be declined.
2. Asks what they are good at. If they answer in the DM, it drafts **About
   & skills** lines for their profile as a card they **Confirm** — the
   profile is theirs, the agent only proposes. Once saved, they are in the
   pool it draws suggested holders from (4.3 step 4). (Suggesting
   communities to a person with none in mind comes later.)
3. Puts nothing on their My Work on day one. Offers come later — from a
   person, or from the agent — when there is a fit.

### 4.15 What it never does

- Decide money, membership, or direction.
- Draft a **money-in** proposal. Pay and reimburse go out; incoming money
  is not a vote.
- Assign work, or promote its own draft.
- Suggest on every message, or raise a dismissed suggestion again with
  nothing new behind it.
- Speak unasked in a room it only listens to by tag. Being in every channel
  and DM is what lets it be called and lets the record be whole; it is not
  a licence to comment.
- Hide what it heard. Nothing said in the community is private from the
  org; a receipt can point into any conversation, and the reader gets that
  message. It says so at the door (3.1), and it does not pretend otherwise
  later.
- Close, extend, or open a project. (The relay closes a project on the end
  date a Shaper set; that is a date rule, not the agent.)
- Mark anything done on someone else's word, on a transcript, or on a
  merged branch — code landing earns a card (4.7a), not a close.
- Read anyone's machine. Progress notes come from a member's own agent
  (1.14); the org agent only reads what the relay has already checked.
- Flip a state without a receipt.
- Write a state at all: it publishes drafts and health reads signed with its
  own key; every state change is a person's signed command that the relay
  executes. The one exception is done-from-talk (4.7), where the agent
  relays the DRI's own signed message and the relay checks the author.

---

## How the four meet — one loop

1. **Member (You)** accepts Sam’s setup ticket (3.3) — or an **AI is
   asking you** card for Autumn harvest fair — and becomes a ticket DRI.
   After setup is held, the agent drafts **keys** under it (1.6).
2. **Ticket DRI (Lea)** sees a rota draft the agent wrote once covers was
   accepted; or asks for the split herself (1.6). Says done in the room
   (1.3); asks for the pay proposal (1.7).
3. **Shaper (Maya)** **Agrees** strategy (“we do not take brand money”) —
   not Confirm v5, and not a sponsorship vote (2.3, 2.6). Approves pay
   with Sam (2.6). Sees the Weekday hall **AI card** (suggesting Lea, then
   Rafi) and either offers it (2.5) or names a DRI by proposal (2.5a).
4. **The agent** heard each of those lines (4.1), drafted the ticket, the
   strategy version, the pay proposal and the follow-up (4.3, 4.2, 4.9,
   4.10), and after direction was agreed walked the cascade — project, then
   DRI, then subtickets (4.4) — without deciding any of it.

Same doors, different cards. Energy is the same loop with different names
(Alex / Rowan / Carbon credits).

---

## Related

- [The Intelligent Organization — What it is](./intelligent-org-features.md) — the features these flows exercise
- [The Intelligent Organization — Design](../architecture/intelligent-org-design.md) — work tree, confirm rules, money, on Buzz
- [The Intelligent Organization — Protocol](../architecture/intelligent-org-protocol.md) — the events behind every tap above
- [The Intelligent Organization — Current State](../architecture/intelligent-org-current-state.md) — what Buzz has today
- Clickable preview: [hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app) — still says _Projects_ for the Work door
