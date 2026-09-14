---
title: 'The Intelligent Organization — User Journeys'
date: 2026-09-11
status: draft
tags: [product, intelligent-org, journeys, hypha]
---

# The Intelligent Organization — User Journeys

Step-by-step flows through the app, by role — three human roles and the
org agent. Companion to [What it is](./intelligent-org-features.md) (the
features) and [Design](../architecture/intelligent-org-design.md) (how they
are built).

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

Everyone sees the same left menu. Only the contents change.

| Door            | What it answers                                                            |
| --------------- | -------------------------------------------------------------------------- |
| **Overview**    | Who are we — mission, vision, objectives, strategy, people, glance numbers |
| **Projects**    | The whole tree — every project and what sits under it                      |
| **Decisions**   | Four filters: **Projects** (approval and **project DRI**), **Money** (out only), **Direction**, **Join** (people only). Shapers vote; anyone can read. |
| **My Work**     | What needs _my_ yes or no — including AI cards — what I hold, and what I offered |
| **My Profile**  | Who I am across orgs — health, balances, current work, earlier work, recent decisions    |
| **DMs**         | Personal Assistant, then one-to-one chats                                  |
| **Group chats** | Rooms per project or team, plus the Shapers room                           |

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
     ticket is assigned, if it needs pieces, it drafts those subtickets.
   - a **ticket draft** under your project (project DRI only) — confirm / discard
   - a **"work finished" nudge** (project DRI only) — a ticket under you is done;
     ask the assistant for the pay proposal
3. Read **You hold** — every piece with your name, its project, its due date.
4. Read **You offered** — tickets you put out: open, waiting on someone, or
   held by them. They stay here until done. If a ticket is waiting on you, it
   sits in **Needs your answer**, not here. Accepted-by-others is not **You hold**.
5. Click a card → the ticket page: draft, children, who is waiting. The
   agent always names who should do it. You can change the person.
6. Optional: open **Projects** to see the parent and what sits beside you.

### 1.2 Report done — via the Personal Assistant

1. Open **DMs → Personal Assistant**.
2. Type: “Found both covers — done.”
3. Agent marks your ticket done and replies with the receipt (your message).
4. Ticket shows done on My Work and Projects.

Only the DRI can close their own ticket this way. Someone else saying it in
chat changes nothing.

### 1.3 Report done — via the group chat

1. Open **Group chats → the project room** (e.g. “Saturday stall”).
2. Type the same sentence: “Found both covers — done.”
3. Agent marks your ticket done and posts the receipt in the room.
4. Ticket shows done on My Work and Projects.

### 1.4 Report done — on the ticket

1. Open **My Work → You hold → the ticket**.
2. Click **Mark done**.
3. Ticket shows done. Receipt = the ticket page.

### 1.5 Done is blocked by open children

1. Try to mark done while a piece you split is still open.
2. Agent / ticket says: “Jun holds the rota under your ticket — yours
   cannot close until his piece does.”
3. Wait for the child’s done, or take the piece back.
4. Mark your own done afterwards. Done moves up the tree, never down.

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

**The agent drafts it when the ticket is assigned**

After you accept a ticket, if its description names pieces nobody covers,
those drafts land on **Needs your answer** (kicker **Drafted by the agent**).
Same confirm, same offer — you still hold the parent. Example in the
preview: keys under Saturday setup; sandbox charts under the Ameland
summary.

### 1.7 Ask for the pay proposal (after done)

No sum lives on the ticket. When the work is done, the person names the sum
and the agent drafts the proposal. In the MVP **every payment is approved by
the Shapers**, whatever the sum and wherever the ticket sits in the tree.

1. Mark done first (1.2 – 1.4).
2. Open **Personal Assistant**.
3. Type: “draft a proposal for the Shapers for my work — 150 USDC”.
4. Agent shows a **payment draft** card: the sum, the ticket, the done receipt.
5. Click **Open as a proposal**.
6. Proposal appears under **Decisions → Waiting on the Shapers**.
7. When it passes, the payment shows on **My Profile → Paid to you**.

### 1.8 Agree pay in chat — _future, not MVP_

The agent remembers a pay line said in a room, so “whatever we agreed” works
later without naming the sum again.

1. Open the **project room** (ticket holder ↔ project DRI) or the
   **Shapers room** (project DRI ↔ Shaper).
2. Type the line: “For finding the two covers, we said 150?”
3. The other side answers: “150 USDC when both are found. Deal.”
4. Agent notes it: “Noted — 150 USDC, agreed between Lea and Sam.”
5. Later, in 1.7 step 3, type “— whatever we agreed” instead of a sum.
   If a named sum differs from the agreed line, the draft shows both.

### 1.9 Project DRI — promote a ticket drafted under your project

1. Someone says a need in the room and the agent drafts a ticket under your
   project — or the agent noticed on its own that your project names a piece
   no ticket covers (4.4). The card says which.
2. Open **My Work → Needs your answer** — the ticket draft is there.
3. Click **Confirm** (make it real) or **Discard**.
4. Offer it to a named person, or leave it open with no DRI. It moves to
   **My Work → You offered** (`open` or `waiting on …`).

### 1.10 Project DRI — ask pay for someone under you

1. Their ticket is marked done.
2. Open **Personal Assistant**.
3. Type: “draft a proposal for Lea’s covers work — 150 USDC”.
4. Same payment draft, same **Open as a proposal**.

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
6. Proposal appears under **Decisions → Projects**, tagged **project DRI**.
   When the Shapers agree, that person holds it. That is stronger than an
   offer on a card (2.5) — a vote names them.

### 1.12 Create a ticket from Personal Assistant

1. Open **DMs → Personal Assistant**. The header names the org
   (`Personal Assistant (River Commons)` / `(Hypha Energy)`).
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
4. **Mark my ticket done** — only your own ticket; your confirm is enough.
5. **Create a ticket** — for you, or someone else (1.12).
6. **Name a DRI** — for work that has none (1.11).
7. **Org overview** — who we are, with receipts.

---

## 2. Shaper

Sets direction, approves root projects, decides money.
In the prototype: **Maya** and **Sam** (River); **Alex, Edgar, Zekeriya** (Energy).

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

You founded the space; you are the only Shaper. No room to open.

1. Open **DMs → Personal Assistant**.
2. Agent: “From what you told me at creation, here is a first **mission**
   and **vision**. Confirm, or tell me what is off.” Two draft cards.
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

1. Open **Group chats → Shapers**.
2. Talk: “We do not take the brand sponsorship. Not this year.”
   (or drop a call / document).
3. Agent posts a **strategy draft** card — v5, one line added, the diff
   shown. If you said it in your Personal Assistant instead, the card
   still lands here so the other Shapers see it.
4. Click **Agree** or **Decline**. The voted sentence is the title — not
   “Confirm v5”. Other Shapers are notified.
5. Overview shows Strategy v5. Everything the agent drafts now reads from it.

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

1. Open **Decisions → Projects**, or **My Work → Needs your answer**.
2. Click the **project approval** card. It came either from talk (someone
   asked for it) or from the agent after direction was agreed (4.4). Same
   decision either way.
3. Read: suggested DRI if any, description, exact end date, opened by, and
   **which objective it serves** (or _none_ — the agent flags a project that
   serves no objective, so you can approve it anyway, or redraw the
   objectives first).
4. Click **Agree — approve it** (or **Decline**).
5. When the Shapers agree, the project is live under **Projects**. If
   nobody is named yet, it stays open until someone is offered (2.5) or
   named by a **project DRI** vote (2.5a).

### 2.5 Offer a project that has no DRI

1. Open **My Work → Needs your answer** — the open project card (e.g. Weekday hall).
2. Click **Offer to …** and pick a person.
3. If they decline, the card comes back — pick someone else.
4. When someone accepts, the approval is recorded under **Decisions**.

### 2.5a Name a DRI by proposal — Shapers room

Same gap as 1.11, from the room. Anyone can start it; Shapers vote it.

1. Open **Group chats → Shapers**.
2. Talk: “Weekday hall still has no one. Name Rafi.” (Energy: “Carbon
   credits is still a draft. Name Rowan.”)
3. Agent posts a **DRI draft** card. Click **Open as a proposal**.
4. All Shapers vote on **Decisions → Projects** (tag **project DRI**).
   When it passes, they hold it.

### 2.6 Approve a payment

Money proposals are **outgoing only** — pay or reimburse. There is no
money-in vote (a sponsorship is not a proposal; “no brand money” is
strategy, 2.3).

1. Open **Decisions → Money**.
2. Click the **money movement** card.
3. Read: the sum, the ticket, the done receipt, opened by.
4. Click **Agree — pay it** (or **Decline**).
5. When every Shaper agrees, the sum moves; it appears on the payee’s profile.

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
2. Read the **brief**: what was held, what was done, what was not, what was
   paid through proposals, and whether the objective it served moved.
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
   **Projects** with the suggested DRI offered, and is recorded under
   **Decisions → Decided** like any other approval. No second vote.
5. Override is available but not the default: **Keep it open until …** sets a
   new end date instead. A Shaper taps; the agent never closes, extends, or
   opens a project on its own.
6. If several Shapers, the card collects their taps — it shows who has
   answered, and the follow-up goes live when enough have agreed. Still one
   decision, in one place.
7. Overview timeline and Projects update: the project shows closed, the
   follow-up shows live (or offered, until the DRI accepts). The decision and what followed
   go into decision memory, so the next recommendation for a similar
   project is better.
8. If the close met the objective — or showed it will not be met — an
   **objectives draft** follows (2.3a).

### 2.9 Bring someone in as a member

Join is **people only**. Orgs do not request to join as members. The card
has no Recipient row.

1. Open **Decisions → Join** (or **My Work → Needs your answer** — the
   same join card).
2. Click **Agree** (or **Decline**).
3. They appear on Overview; nothing lands on them until they accept work.
   In the preview: River has Rafi open and Priya passed.

### 2.10 Ask the org anything

1. Open **Personal Assistant**.
2. Type: “Have we dealt with the council before?”
3. Read the answer and follow the **receipts** — threads, proposals, projects.

---

## 3. Member — not a DRI yet

Joined; holds nothing until they accept a piece.
In the prototype: **You**, first login.

### 3.1 Join the space

1. First login → talk to the assistant.
2. Profile cards fill as you speak; **Confirm** each one.
3. Pick a suggested space, or land in the one you were invited to.
4. Assistant: “I’ll let the others know your skills and that you’re available.”
5. You are a **member**. No work queue on day one.

### 3.2 Look around

1. Open **Overview** — mission, vision, objectives, strategy, who shapes, who holds what.
2. Open **Projects** — every project, its DRI, what is open.
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

1. Open **Group chats → a room**.
2. Type a need: “Who signs the hall licence?”
3. Agent drafts a ticket or project from it — with whoever holds the piece
   above, not with you.
4. Nothing lands on you unless someone offers it and you accept.

### 3.7 Ask the org anything

1. Open **Personal Assistant**.
2. Ask: “What is this org for?” / “Who holds the stall?” / “Have we done this before?”
3. Follow the receipts.

### 3.8 Check your profile

1. Open **My Profile**.
2. Read: who you are, what you hold (nothing yet), **Paid to you** (empty).
3. After your first accepted piece is done and paid, the payment shows here.

---

## 4. The org agent

Not a role, not a chatbot. One agent per org, running the loop
_hear → remember → offer → watch → revise_ server-side. It has no door of
its own: it shows up as cards on My Work, drafts in rooms, replies in the
Personal Assistant, and lines on Overview and Decisions.

Two rules it never breaks: it **drafts, never decides**; it **offers, never
assigns**. Every flow below ends with a human tap or with nothing.

### 4.1 Hear

1. A message lands in a room, a DM with the assistant, or a call transcript
   is ingested. The agent stores it as an event with author, room, time.
2. A cheap rule decides whether the batch is worth thinking about: a
   mention of open work or its DRI, a question to the room, a commitment
   verb, a spike of activity, a transcript, or anything a Shaper says in
   the Shapers room or their own assistant chat.
3. Most batches stop here. No model call, no card. Cost scales with
   candidates, not with everything everyone says.
4. A candidate goes to **Think** with the latest direction (4.2), the open
   work tree, the recent room window, and relevant past outcomes.

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
   _none_, flagged), a suggested holder if one was named, an exact date.
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
   project. Same kickers. Naming them still needs an offer they accept
   (2.5) or a **project DRI** vote (2.5a).
6. **After a ticket is held** — pieces the ticket names that nobody covers
   → **subticket drafts** on the holder's **Needs your answer** (kicker
   **Drafted by the agent**). Same card as a talk-derived split (1.6).
   Preview: keys under Saturday setup; cash-box teach for Tom; rota for
   Jun; Energy sandbox charts.

**Discipline:**

7. One open suggestion per objective and per project at a time. Never two
   drafts for the same gap.
8. A dismissed suggestion is not raised again until something changed — a
   new direction version, a project closed, a new person with the skills.
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

### 4.8 Nudge the person above

1. A ticket goes done.
2. Agent posts a **“work finished”** card to the project DRI's My Work
   (and, for a root project done, to the Shapers): what was done, by whom,
   the receipt.
3. Offers the next step in one line: “Ask me for the pay proposal when you
   are ready.” No sum guessed.

### 4.9 Draft the pay proposal on request

1. Someone involved tells the assistant: “draft a proposal for the Shapers
   for my work — 150 USDC” (or for someone under them).
2. Agent checks the ticket is done. If not, it says so and stops.
3. Drafts the **payment** card: the sum, the ticket, the done receipt, who
   asked. Future (1.8): if a sum was agreed in a room, it carries that
   line; if the named sum differs, it shows both.
4. On **Open as a proposal**, the proposal enters the existing governance
   path. **In the MVP every payment goes to the Shapers** — any sum, any
   level of the tree, whoever asked. No thresholds, no delegated approvers.
   The agent never moves money.

### 4.10 Watch the end date

1. A date rule — not a model — fires when a project enters the last fifth
   of its run (20% of approval → end date, floor two days). Proportional, so
   a two-week project gets its brief three days out, not before it starts.
2. Agent assembles the **brief** from the ledger: held, done, not done, paid
   through proposals, and whether the objective it cites moved.
3. Writes one **recommendation**: a **follow-up project** draft (title,
   suggested DRI, description, exact end date, same objective) when the
   objective is live or the brief shows a clear next step; or **no further
   work** with the reasoning. One line of why, with receipts.
4. Routes the review card to the Shapers (My Work, and the Shapers room if
   several).
5. On the date, closes the project — the same scheduled rule — unless a
   Shaper set a new end date.
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

1. Anyone asks the assistant: “Have we dealt with the council before?” /
   “What is this org for?” / “Who holds the stall?”
2. Agent answers from the confirmed direction and the live work tree, then
   searches the history for the threads, decisions, and outcomes behind
   the answer.
3. Every claim links to its receipt — a message, a proposal, a project.
   Live numbers (treasury) are fetched at question time, never remembered.

### 4.13 Learn from outcomes

1. Every decision and what followed it is kept: offer → accepted or
   declined; recommendation → taken, amended, or rejected; unprompted
   suggestion (4.4) → confirmed or dismissed; project → done on time,
   extended, or closed with work open.
2. Next time it drafts something similar — a holder to suggest, a follow-up
   to propose, an end date to set — it reads those outcomes first.
3. It never rewrites a belief from an outcome by itself; it drafts (4.11)
   and a Shaper confirms.

### 4.14 Welcome a newcomer

1. First login: talks the person through their profile; each card is
   confirmed by them.
2. No space in mind → suggests spaces from the profile and the spaces' own
   direction. Invited → lands them in that space as a member.
3. Tells the space: skills, availability. Adds them to the pool it draws
   suggested holders from (4.3 step 4).
4. Puts nothing on their My Work on day one. Offers come later — from a
   person, or from the agent — when there is a fit.

### 4.15 What it never does

- Decide money, membership, or direction.
- Draft a **money-in** proposal. Pay and reimburse go out; incoming money
  is not a vote.
- Assign work, or promote its own draft.
- Suggest on every message, or raise a dismissed suggestion again with
  nothing new behind it.
- Close, extend, or open a project without a Shaper's tap — except the
  scheduled close on an end date a Shaper already set.
- Mark anything done on someone else's word.
- Flip a state without a receipt.

---

## How the four meet — one loop

1. **Member (You)** accepts Sam’s setup ticket (3.3) — or an **AI is
   asking you** card for Autumn harvest fair — and becomes a ticket DRI.
   After setup is held, the agent drafts **keys** under it (1.6).
2. **Ticket DRI (Lea)** sees a rota draft the agent wrote once covers was
   assigned; or asks for the split herself (1.6). Says done in the room
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
- [The Intelligent Organization — Design](../architecture/intelligent-org-design.md) — work tree, confirm rules, money
- [The Intelligent Organization — Current State](../architecture/intelligent-org-current-state.md) — what is shipped vs designed
- Clickable preview: [hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app)
