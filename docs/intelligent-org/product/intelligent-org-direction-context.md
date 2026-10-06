---
title: 'The Intelligent Organization — What Shapers Give the Agent to Suggest Projects'
date: 2026-10-02
status: draft
tags: [product, intelligent-org, direction, shapers, agent, buzz]
parent: docs/intelligent-org/README.md
---

# What Shapers give the org agent so it can suggest projects

The org agent suggests a project when it sees a **gap**: an objective, or a
strategy line that names an action, that no live project serves. Whether that
suggestion is one the Shapers would have written themselves depends almost
entirely on what the Shapers have put down before the agent looks. This
document says what that is: what goes in each of the five direction texts,
what lives outside them, where each thing a Shaper wants the agent to know
belongs, and how to tell when the org is ready.

It is written for Shapers (and for whoever designs the chat that helps them
write direction). Companion to
[What it is § 1](./intelligent-org-features.md#1-direction-stays-current),
[AI evaluation § 1](../plans/intelligent-org-ai-evaluation.md#1-direction--projects),
[Org agent § 8](../architecture/intelligent-org-agent.md#8-think), and
[Organizational Intelligence § 1–3](../architecture/organizational-intelligence.md).
When this document disagrees with those, they win.

---

## The short answer

Seven things, in order of weight:

| #   | What                    | Who writes it                         | What it does for a project suggestion                                       |
| --- | ----------------------- | ------------------------------------- | --------------------------------------------------------------------------- |
| 1   | **Objectives**          | Shapers (confirmed version)           | The thing projects are drafted from. No objectives, no suggestions.         |
| 2   | **Strategy**            | Shapers                               | Bets that can become projects; refusals every draft is checked against.     |
| 3   | **Mission**             | Shapers                               | Scope: is this ours, and for whom.                                          |
| 4   | **Vision**              | Shapers                               | Direction of travel and scale: how big a step is the right step.            |
| 5   | **Member profiles**     | each member, for themselves           | Who can hold the project. No profile and no history means never suggested.  |
| 6   | **The work tree**       | Shapers propose projects; DRIs split  | What is already served. Work missing from it gets suggested again.          |
| 7   | **Answers on drafts**   | Shapers, by agree / amend / decline   | What this org did and did not want last time, so it is not asked twice.     |

One rule sits over all seven:

> **When the agent suggests projects, it reads nothing anyone said in chat.**
> If it is not in the five texts, a profile, the work tree, or a past decision
> on a draft, it does not exist for this move.

That is a design choice, not a gap (Org agent § 8.1: _nothing from L1 enters a
gap move_). Project suggestions are about the distance between what the org
committed to and what it is doing — not about whatever was said last. Talk
turns into work through a different path (Org agent J7: a project or ticket
drafted from a message in `#shapers` or a project room). So anything the
Shapers have agreed in conversation, and want reflected in suggestions, has to
be confirmed into direction.

---

## 1. What the agent reads, and what it makes from it

### When it runs

- A new version of **objectives** or **strategy** is confirmed.
- The **Monday scan**: any objective whose date is getting close with nothing
  under it.
- **Not** when mission or vision change alone — those are direction, not a
  to-do.

### What it reads (Org agent § 8.1, column J1)

| Slice                                                      | Comes from                                    |
| ---------------------------------------------------------- | --------------------------------------------- |
| The five direction texts, in full, with line ids           | the latest confirmed `39100` of each          |
| Community facts: Shapers, decision rules, language, today  | `39103` and the relay                         |
| Every live project, one line each, with the objective it serves and its holder | the work tree (`39101` roots) |
| The last two closed projects, one line each                | the work tree                                 |
| Earlier suggestions on the same gap and how they were answered | the decision record (`39104`, L4)         |
| Up to ten candidate holders: skills, about, open count against their limit, up to three items they held | `39105` profiles and the tree |

### What it does not read

Chat in any room or DM. Threads, calls, transcripts. Documents or links dropped
in chat (they can _inform_ a direction draft, but only the confirmed text
counts). The conversation in which the direction was written. Treasury or any
other live number. Profiles of members who are not among the candidates.

### What it makes

First a **gap list**: every objective line and every action-naming strategy
line marked _served_, _partly_, or _not_, with the projects that serve it. Then
at most **one project draft per line** marked _not_ or _partly_:

| Draft field        | Built from                                                                          |
| ------------------ | ----------------------------------------------------------------------------------- |
| `title`, `brief` (≤ 60 words) | the objective line, read through mission scope and strategy               |
| `objective_ref`    | the line's id — the draft must cite a real line                                      |
| `due_at`           | an exact date **before** the line's date                                             |
| which step         | the first step when the line says something is not known yet (a survey before the build, a permit before the programme) |
| `suggested_dri`    | one candidate, with the skills, about span, or past items that make them the fit — or nobody, with what the org lacks |
| `why`              | one line naming the gap                                                              |
| silence            | when every line is served, a line is a refusal, or it was declined with nothing changed since |

Every field on the right traces back to something a Shaper or member wrote.
That is the whole of this document in one table.

---

## 2. The five texts, written for project suggestions

Each text is a **statement** (one sentence) and, where it helps, **a paragraph
or two behind it** (Protocol §4.1: `body` is _"the statement and the paragraph
or two behind it"_). The situation is the exception: one paragraph, no
separate statement. Objectives and strategy are also **numbered lines** —
those lines are what a project cites.

The examples are River Commons from the
[prototype](../../../prototypes/org-preview/src/lib/data.ts), a neighbourhood
food hub that is also the evaluation seed.

### Mission — is this ours, and for whom

**What the agent uses it for.** A scope check on every draft, and the
_who it is for_ that a good brief names.

**It must say:**

- **What we do.** The activity, in plain words.
- **For whom.** Named, concrete people or groups — not "communities" or "users".
- **Where**, if the work is bounded by a place, a language, or a jurisdiction.
- **What we are not.** Two or three forms the org refuses to take. These stop
  the agent drafting plausible projects that belong to a different
  organisation.
- In the paragraph: **who we already work with** (partners, suppliers, the
  people the org exists for, by name), and **why we exist** — the problem in
  one or two sentences.

**Example.**

> A street food hub from people we know, not a supermarket — for neighbours,
> and the three growers we already buy from.
>
> We already buy from three growers — Ana, Tomasz and the Ferreira family. The
> hub exists so the whole street can do the same without a supermarket in
> between, and so the growers see the money the same week. It is a stall, then
> a hall, run by people who live here. Not a restaurant, not a marketplace app,
> not a brand's community programme.

**Fails:** "We empower communities through sustainable food." No _for whom_, no
boundary, nothing a project could be checked against.

### Vision — where this is going, and how far

**What the agent uses it for.** Whether a project moves toward the end state,
and how ambitious a single step should be.

**It must say:**

- **A picture you would recognise when it is true.** Something a stranger could
  walk into and confirm.
- **The two to four measures** that make it true, in words — _five growers_,
  _two days a week_, _10,000 hubs_.
- **A horizon** — a year, or "in three years".
- In the paragraph: **what each measure means** and why that number.

**Example.**

> A neighbourhood that feeds itself two days a week: five growers, a hall paid
> without a whip-round, every grower paid the week they sell.
>
> Two days a week means the Saturday stall and a weekday hall. Five growers is
> what a hall can carry without a van. "Paid the week they sell" is the whole
> point — no invoices, no waiting, no one fronting the money.

A target number is fine here: it is a commitment, an interpretation. What does
not belong is a **reading** — how many members, how much money, how many
stalls happened. Those change weekly and are fetched live (Organizational
Intelligence § 1: _L3 holds interpretation, never readings_).

**Fails:** a slogan ("A better food future for all"), or a status report.

### Situation — where we stand today

**What the agent uses it for.** Where every objective starts. The same
objective means a different first project for an idea nobody has tested, a
service that is running, and an org in trouble: a test of demand, the number
that matters most, or the repair. Without it the agent drafts from the end
state.

**It must say, in one paragraph of three to six sentences:**

- **The stage.** Only an idea, just started, running a service or product,
  growing, or in trouble.
- **What exists.** The service, who uses it, the partners — by name.
- **What is proven and what is only assumed.** The line between them is where
  the next project starts.
- **What is stuck**, if anything, and **the one thing the org must learn
  next.**
- **Capacity and money, in words.** _Three volunteers, a few hours a week;
  money for the season, none for a hall._

**Example.**

> Running one season: the Saturday stall every week since March, three growers
> selling, about forty regulars. Demand on Saturdays is proven; whether anyone
> comes on a weekday night is not — we have never run one, and the hall has no
> evening licence yet. Three volunteers carry it on a few hours a week, and the
> season pays for itself but not for a hall. The next thing we must learn is
> whether a weekday night fills.

_About forty regulars_ is a reading the Shapers chose to state, in words; it
dates as the org grows, which is why the situation is redrafted when the stage
changes. Exact counts stay out (§ 7).

**Fails:** a status report (_this week: 112 sales, €640_), a wish (_we are
growing fast_), or the mission again. If it would read the same next year, it
is not a situation.

### Objectives — the text projects are drafted from

**What the agent uses it for.** Every project draft cites one objective line.
The line's date bounds the project's end date. The Monday scan watches the
dates. An org with no confirmed objectives gets no project suggestions.

**Three to seven lines.** Near-term — this season, this year. When one is met
or dropped, the list is redrawn, not appended.

**Each line must:**

1. **Be one outcome, not an activity.** _A weekday hall is open before August_
   — not _Work on the hall_. An activity is already a project; the agent has
   nothing to add.
2. **Be checkable.** Someone could say yes or no on the date.
3. **Carry its date in the sentence.** _before August_, _by spring_, _all
   season_. (Today the chat stores lines as text only — see § 8 — so the words
   are where the date lives.)
4. **Name the starting point when it decides the first step.** _A second island
   by December — no site chosen._ _A weekday hall before August — we have never
   run a weekday night._ This is what lets the agent draft the survey before
   the installation, the licence before the opening, instead of the end state.
   Say it in words, not as a count the work tree already knows.
5. **Name the outside reason for the date, when there is one.** _Six Iberian
   communities ready for EECF round 2 by spring._ A funding round, a season, a
   permit window, a partner's deadline — the agent cannot know it otherwise.
6. **Not overlap another line.** If two lines share ground, say where one ends.

**In the paragraph:** what "soon" means for this org, and anything
deliberately left off the list this round, so the agent does not draft it from
the vision.

**Example.**

> This season: Saturday every week, a weekday hall by August, and five growers
> paid the week they sell.
>
> 1. The Saturday stall runs every week, all season.
> 2. A weekday hall is open before August.
> 3. Five growers by autumn, each paid the week they sell.

**Fails:**

- _Be more visible._ Too vague to serve — the right answer is a draft that says
  so, or nothing (AI evaluation § 1, adversarial cases).
- A list of projects dressed as objectives — _Build the website_, _Hire a
  coordinator_. Write the outcome the website is for.
- Undated lines. The agent cannot date the project, and the scan cannot tell
  the line is getting late.

### Strategy — the bets, and what we say no to

**What the agent uses it for, two ways.** A line that **names an action**
(_publish the Ameland report before any marketing_) can become a project. A
line that **refuses** (_we do not take brand money_, _no restaurant_) never
becomes one — and every draft is checked against it.

**It must say:**

- **Two to six bets** — how the org intends to reach its objectives. _Prove it
  in real towns first. The stall funds the hall._
- **The refusals** — what the org will not do, so the agent never drafts it.
- **Order between objectives**, when it matters. _Saturday first; the hall only
  once the stall pays for it._ Do not rely on the order of the objective lines
  to carry priority; say it here.
- **How work gets done** — volunteers or paid, a few hours a week or full-time,
  local, open-source, through partners. This sets the size and form of every
  brief.
- **The money posture, in words** — _small money, many hands — not a grant
  round_. Never a balance.
- In the paragraph: **why these bets**, and what the org learned that produced
  each one.

**Example.**

> Small money and many neighbours — the stall pays for the hall, not a grant or
> a restaurant.
>
> 1. Small money, many hands — the stall funds the hall, not a grant round.
> 2. No restaurant. Two days a week, not a fleet.
> 3. RIVER vouchers keep value with the growers, not the middle.

Line 2 was written the week after the Shapers rejected a second-hand van. A
refusal learned the hard way belongs here: it is the only way the agent learns
it.

**Fails:** values with no consequence (_we value transparency_), or a line that
is really an objective (_reach 500 members_).

---

## 3. What you want the agent to know, and where it goes

Shapers often have more to say than five texts seem to hold. Almost all of it
has a home. Put it there; do not invent a fifth document.

| You want the agent to know…                                  | Put it in                                                       | Because                                                              |
| ------------------------------------------------------------ | --------------------------------------------------------------- | -------------------------------------------------------------------- |
| Who we serve; who we are not for                             | Mission                                                         | The scope check reads it on every draft.                             |
| Forms we refuse to take (not an app, not a restaurant)       | Mission paragraph; repeat as a strategy refusal if it is a live temptation | Refusals in strategy are what drafts are filtered against.  |
| Partners, suppliers, the people we already work with         | Mission paragraph (named)                                       | Briefs can name them; the agent will not invent others.              |
| Place, language, jurisdiction, legal form — when it bounds the work | Mission                                                  | Stops projects in the wrong place or form.                           |
| What success looks like, and how big                         | Vision, with its measures                                       | Sets the size of a step.                                             |
| Where we are now                                             | In the objective line it affects (_— no site chosen_)          | Decides the first step.                                              |
| Outside deadlines (funding rounds, seasons, permits)         | The objective line's date and reason                            | The only way a project gets dated before it.                         |
| What matters most right now; what waits                      | Strategy line on order                                          | Line order is not read as priority.                                  |
| How we fund work; money we refuse                            | Strategy, in words                                              | Shapes every brief; never a balance.                                 |
| Volunteers or paid; how much time people have                | Strategy (how work gets done) and each member's `open_limit`    | Sizes the project; the agent skips anyone at their limit.            |
| Skills we have                                               | Members' own profiles                                           | The holder suggestion only quotes what a member wrote.               |
| Skills we lack and will not build                            | Strategy (_we contract electrical work_)                        | Otherwise the agent drafts it and says nobody fits.                  |
| Things we tried and what we learned                          | Closed projects and their reviews; the strategy line it produced | The last two closed roots and past decisions are read; the lesson as a line is read every time. |
| Work already under way, inside or outside Buzz               | The work tree, as a project citing its objective line           | Otherwise the line looks unserved and gets a duplicate.              |
| An idea for a project nobody has agreed                      | Not direction. Say it in `#shapers`, or propose the project     | Talk becomes work through J7 or a proposal, not through direction.   |
| Balances, sales, member counts, attendance                   | Nowhere in direction                                            | Fetched live; a written number goes stale and gets recited.          |
| A strategy deck, a grant application, a founding document    | Drop it in the chat with the agent while writing direction      | It can help draft the texts. Only what is confirmed counts.          |

---

## 4. Outside direction: the three things that make a suggestion land

### People — profiles

A project draft without a credible holder is half a draft. The holder
suggestion is **evidence-based**: the agent picks from up to ten candidates —
members whose skills or about share terms with the brief, plus members who held
work under the same objective — and must quote what made them the fit
(Org agent § 8.2). It skips anyone at their `open_limit`. It never guesses at a
member with **no profile and no history**. In a new org the founder is the
fallback. "Nobody here fits; this needs someone who can X" is a correct answer.

Shapers cannot write anyone else's profile (Organizational Intelligence § 2:
_written by nobody but its subject_). What they can do:

- **Write their own on day one** — about, skills, a limit.
- **Ask every member to write theirs** before the first objectives are
  confirmed. A member can do it by talking to the agent in their DM; it drafts
  the profile and they confirm it.
- **Skills as specific slugs** — `grant-writing`, `electrical-certification`,
  `spanish` — not `helping` or `community`. The match is on terms.
- **About in plain sentences** about what the person has done and wants to do.
  It is quoted as evidence.

Limits: about up to 1,000 characters, up to 20 skills, `open_limit` 1–50 or
none.

### The work tree

The agent decides a line is served by reading the live projects and the
objective each one cites. So:

- **Every project that already exists gets entered**, including work that
  started before the org was on Buzz, as a project proposal naming the
  objective it serves.
- **Every project a Shaper proposes names its objective.** A project with no
  objective cannot serve a line, and the line will look open.
- A project that serves a line **partly** should say in its brief what it does
  not cover; that is what the agent drafts next.

### Answers on drafts

- **Decline with a reason.** A declined gap is not raised again until the
  direction version or the work under it changes. The reason is what the next
  suggestion learns from.
- **Amend rather than decline** when a draft is nearly right. The edit is the
  most useful thing the org teaches the agent (Organizational Intelligence § 2).
- **If the declines keep saying the same thing, the direction is wrong.**
  Change the objective or add the strategy refusal; do not keep declining.

---

## 5. Ready to start

### The minimum for the first suggestion

- [ ] Objectives confirmed, with **at least one line** that has a date in it.
- [ ] Mission confirmed, saying for whom and what the org is not.
- [ ] The founder's profile written.

With only this, the agent can draft — but expect the holder to be the founder
and the briefs to be generic.

### What a good first run needs

- [ ] All five texts confirmed.
- [ ] The situation names the stage and what is proven versus assumed.
- [ ] Three to seven objective lines; each an outcome, checkable, dated, with
      its starting point where it decides the first step, and its outside
      reason where there is one.
- [ ] Strategy has the bets, at least one refusal, the order between
      objectives if it matters, and how work gets done.
- [ ] Vision has its measures and a horizon.
- [ ] Every member has a profile with specific skills.
- [ ] Every piece of work already under way is a project citing its objective.
- [ ] No balance, count, or other live number appears in any text.
- [ ] All five texts together read in a few minutes. If they take an
      afternoon, they are too long (Organizational Intelligence § 2: _L3 must
      stay small enough that a person could read all of it in an afternoon_ —
      and the whole of it goes into every call, up to ~6,000 tokens).
- [ ] Written in the language the org works in. Drafts come back in the
      language the direction is written in.

---

## 6. A brief to start the conversation with the agent

The founder (in their DM with the agent) or the Shapers (in `#shapers`) can
paste this, fill it in, and let the agent walk it with them one piece at a
time. Everything settled becomes a draft the Shapers confirm.

**The order is mission, vision, situation, objectives, strategy.** The
answers under _Where we stand today_ become the situation draft. A piece is
finished when it is confirmed, not when it is drafted: the reply that drafts
one ends there, and the agent starts the next piece only once the Overview
shows a version, or when someone asks to move on. When a draft it made is
confirmed, the agent says so in the room the draft came from and asks what
the next piece needs, so nobody has to ask what comes next. It asks one
question per message, the most decisive first, often with an example answer,
and builds the next question on the reply. Objectives
wait for the situation because it decides what the first objective
can be: an untested idea gets a test of whether anyone wants it before
anything is built, a running service gets the one or two numbers that matter
most, and an org in trouble gets the repair first. The agent reads the
overview first (live and closed work, people, profiles, Shapers) and asks only
what it does not show.

**The agent judges every text it is given**, against the bars in § 2: strong,
close, or weak, and what exactly is missing. When a text is not strong it ends
with a _Sharper draft:_ built only from what the Shapers said and what the
overview shows. When the missing piece is a fact only they know, such as who
the org is for, it asks instead of guessing. The draft card still carries
their own words; when they take the sharper version, the agent drafts that
instead. A statement in the wrong text is called out: a _who-for_ is mission,
an end state is vision, an activity is a project, a refusal is strategy. The
prompt is `DIRECTION_COACHING` in `crates/buzz-org-agent/src/chat_act.rs`.

```markdown
## Who we are
- What we do:
- For whom (name them):
- Where / in which language / under which legal form, if it limits us:
- What we are not (2–3):
- Who we already work with (names):
- Why we exist (the problem, in two sentences):

## Where we are going
- When it is true, someone walking in would see:
- The 2–4 measures that make it true:
- By when:

## Where we stand today
- Stage (only an idea / just started / running a service or product / growing / in trouble):
- What exists today (the service, its users or customers, partners):
- What is proven, and what is only assumed:
- What is working, and what is stuck:
- Who does the work, and how many hours a week:
- Money, in words (no balances):
- Outside dates (funding rounds, seasons, permits, a partner's deadline):
- The one thing we must learn next:

## What we want done soon (3–7)
For each: the outcome · the date · where we are today · the outside reason for the date, if any
1.
2.
3.

Deliberately not on the list this round:

## How we get there
- Our bets (2–6):
- What we will not do:
- What comes first, and what waits:
- How work gets done (volunteers / paid, hours, partners, open-source):
- How we fund work, and money we refuse (in words, no balances):
- What we learned that made us decide these:

## Work already under way
For each: what it is · who holds it · which "done soon" line it serves
```

The last section is not direction. It becomes project proposals, so the tree
is true before the agent first looks.

---

## 7. What not to put in direction

- **Live numbers** — balances, sales, member counts, attendance. Fetched live.
- **Tasks** — anything one person could finish in a week. That is a ticket under
  a project.
- **Names of who should hold what.** Holders come from profiles and offers;
  naming a person in direction does not make them a candidate.
- **Instructions to the agent** ("always suggest grants first"). Direction is
  quoted as data, never followed as instructions (Org agent § 8.5). If it is a
  real preference of the org, write it as a strategy line: _grants before
  sponsorship_.
- **Values with no consequence.** If it would not change a single project,
  leave it out.

---

## 8. Today in the app, and open questions

What exists now, checked against the code on 2026-10-02, and where it falls
short of this document. Each item is a decision or a slice to argue for, not a
change made here.

1. **The move that drafts projects from direction is not built yet.** It is
   slice A-2 in the [Development plan](../plans/intelligent-org-development-plan.md)
   (job J1). Today the agent drafts a project only when someone describes one
   to it in chat. Direction written the way this document asks is what
   A-2 reads on its first run, so it is worth writing it this way now.
2. **The Overview direction form sends no lines.** It has one _Body_ box and
   publishes `io_direction_propose` without `lines`
   (`desktop/src/features/org/ui/overview/DirectionFormDialog.tsx`). Objectives
   or strategy confirmed through it have no line ids, so no project can cite
   them and J1 has no line to mark served or not. Until the form edits lines,
   write objectives and strategy through the chat with the agent, which does
   send them (`directionFromChat.ts`, `chatDraft.ts`). Open: give the form a
   line editor, or have it split the body into lines.
3. **Lines from chat carry no date.** The relay accepts a `date` on each line
   (Protocol §4.1), but the chat path sends text only. Hence § 2's rule that the
   date lives in the sentence. Open: have the agent read the date out of the
   line and send it, so the judge's _due before the objective's date_ check has
   a number to check.
4. **Mission and vision from chat are one sentence.** The chat prompt keeps them
   to a single sentence (`crates/buzz-org-agent/src/chat_act.rs`), so the
   paragraph behind them — partners, boundaries, what each measure means — is
   not stored. Open: let the chat settle the statement and the paragraph
   together.
5. **No slot for org-held assets and standing commitments** — a hall key on
   Tuesdays, a licence, a signed partnership. Today the choice is the mission
   or strategy paragraph, where they are interpretation rather than facts. If
   orgs keep needing this, it is an argument for a protocol change
   (Organizational Intelligence § 2: _a sixth artifact is a protocol change to
   argue for, not a slot to fill_) — not something to stretch the five texts
   for. The situation may say _we hold the hall on Tuesdays_ as part of where
   the org stands, but it is the Shapers' reading, not a register.
6. **Priority between objectives is prose.** The agent reads a strategy line on
   order, but nothing structural says which objective comes first. Open:
   whether line order should carry priority, stated once in the prompt.
7. **The situation is the fifth text.** Where the org stands is a confirmed
   `39100` with `d = situation` (Protocol §4.1), between vision and objectives
   on the Overview, full width. The agent asks for it before objectives (§ 6),
   drafts it as one paragraph, and the Shapers confirm it like any other text,
   so a later conversation and the J1 gap move both read it. It closes
   category 11 (_current state_) in
   [Full project context](./intelligent-org-project-context.md) for the part
   that is interpretation; live counts stay in the tree. Open: the agent does
   not yet notice when the situation has gone stale — a passed project or a
   met objective that changes the stage — and offer a redraft unprompted;
   today it redrafts when told something changed.
