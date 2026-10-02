---
title: 'The Intelligent Organization — Full Context for Project Suggestions'
date: 2026-10-02
status: draft
tags: [product, intelligent-org, context, shapers, agent, buzz]
parent: docs/intelligent-org/README.md
---

# Full context for project suggestions — every category

What an AI needs to know about an organisation to suggest the projects an
experienced operator in that organisation would suggest: the right project,
for the right reason, at the right size, at the right time, held by the right
person, and nothing it should not suggest.

This document is deliberately **not** bounded by the current model, where
direction is four texts (mission, vision, objectives, strategy). It lists every
category of context that changes which projects are worth suggesting, says
what each one prevents, and what to capture. The last section maps each
category onto what Buzz holds today and names the tension with the current
design. For writing the four texts well within today's model, see
[Direction context](./intelligent-org-direction-context.md).

---

## Contents

- [How to read a category](#how-to-read-a-category)
- [The reasoning the context has to support](#the-reasoning-the-context-has-to-support)
- [Properties every piece of context needs](#properties-every-piece-of-context-needs)
- [The categories](#the-categories) — thirty, in nine groups
- [Tiers: what to have when](#tiers-what-to-have-when)
- [Written or fetched live](#written-or-fetched-live)
- [Intake template](#intake-template)
- [How this maps to Buzz today](#how-this-maps-to-buzz-today)

---

## How to read a category

Each category has the same five parts:

- **Answers** — the question it lets the AI answer.
- **Without it** — the wrong suggestion the AI makes when it is missing. This
  is the reason the category exists; if no wrong suggestion follows from its
  absence, it does not belong here.
- **Capture** — the fields worth writing down.
- **Example** — River Commons, the neighbourhood food hub from the
  [prototype](../../../prototypes/org-preview/src/lib/data.ts) and the
  evaluation seed.
- **Owner · changes · tier** — who keeps it true, how often it moves, and
  whether it is needed before the first suggestion (Tier 1), within the first
  month (Tier 2), or as the org matures (Tier 3).

---

## The reasoning the context has to support

A good project suggestion is the end of seven steps. Every category feeds at
least one; a step with thin context is where suggestions go wrong.

| Step                 | The question                                               | Fed mainly by                                                                                   |
| -------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| 1. **Find the gap**  | Where is the org short of where it said it would be?       | Objectives & targets (7), current state (11), live portfolio (12), commitments (19)             |
| 2. **Generate**      | What projects could close it?                              | Theory of change (4), strategy (9), beneficiaries (5), ideas pool (13), domain knowledge (26), open questions (24) |
| 3. **Filter**        | Which of those must never be suggested?                    | Anti-goals & red lines (10), values (3), constraints (18), history & rejected ideas (23), feedback (30) |
| 4. **Order**         | Which one comes first?                                     | Open questions (24), dependencies (21), calendar (20), priorities (8)                           |
| 5. **Size**          | How big a step, and how long?                              | People & capacity (14), money (15), assets (16), operating norms (27), risk appetite (22)       |
| 6. **Staff**         | Who could hold it — or who is missing?                     | People & capacity (14), relationships (17), live portfolio (12)                                 |
| 7. **Justify**       | Why this, why now, and what backs each claim?              | Every category's provenance; stakeholders (6), governance (28)                                  |

---

## Properties every piece of context needs

Whatever the category, a fact is only useful to the AI if it has these:

| Property        | Meaning                                                                                              | Why it matters for suggestions                                              |
| --------------- | ---------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| **Specific**    | Named people, places, numbers, dates — not "stakeholders", "soon", "growth".                         | A generic input produces a generic project.                                 |
| **Kind**        | One of: **fact** (true, checkable), **belief** (the org's agreed view), **assumption** (not yet tested), **reading** (a live number). | The AI treats an assumption as something to validate, not to build on.      |
| **Provenance**  | Who said or confirmed it, when, from what decision or conversation.                                  | Every claim in a suggestion has to point at something.                      |
| **Owner**       | The person or role who keeps it true.                                                                | Stale context is worse than none; somebody must notice.                     |
| **Freshness**   | **stable** (years), **slow** (quarters), **fast** (weeks), or **live** (fetched at the moment of use). | Live readings written into a document get recited after they are wrong.    |
| **Negatives**   | What is *not* true, *not* wanted, *not* on the list.                                                  | The cheapest way to stop a confident wrong suggestion.                      |
| **Version**     | Changes are new versions, old ones readable.                                                         | "What did we believe when we declined this?" decides whether to raise it again. |

---

## The categories

### A. Why we exist and where we are going

#### 1. Purpose

**Answers:** What do we do, for whom, and why does it matter?
**Without it:** suggestions that are good projects for a *different*
organisation — a marketplace app for a food hub that exists to keep the
supermarket out.
**Capture:**
- What we do, in plain words.
- For whom, named and concrete.
- The problem we exist to solve, in two sentences.
- The forms we refuse to take (not a restaurant, not an app, not a brand
  programme).
- Origin: why the org was started, by whom, and what it was reacting to.

**Example:** _A street food hub from people we know, not a supermarket — for
neighbours, and the three growers we already buy from. Started by Maya after
the corner shop closed; the growers were selling at a loss to a wholesaler._
**Owner · changes · tier:** Shapers · stable · **Tier 1**

#### 2. Vision and horizon

**Answers:** What does success look like, how big, and by when?
**Without it:** steps of the wrong size — a city-wide scheme for a street
project, or a one-off event for an org aiming at ten thousand hubs.
**Capture:**
- A picture someone could walk into and recognise as true.
- The two to four measures that make it true.
- The horizon (a year, or "in three years").
- What each measure means and why that number.

**Example:** _A neighbourhood that feeds itself two days a week: five growers,
a hall paid without a whip-round, every grower paid the week they sell — by the
end of next year._
**Owner · changes · tier:** Shapers · stable · **Tier 1**

#### 3. Values and principles

**Answers:** How do we behave, and what would we rather lose than compromise?
**Without it:** suggestions that are effective and wrong — a sponsorship drive
for an org that will not take brand money, a paid-ads campaign for one that
grows only by word of mouth.
**Capture:**
- Three to six principles, each with **what it rules out** in practice. A value
  that rules nothing out is not worth recording.
- Ethical red lines (also in category 10).

**Example:** _Growers are paid the week they sell — we never ask a grower to
wait for money we owe. Everything is decided in the open._
**Owner · changes · tier:** Shapers · stable · Tier 2

#### 4. Theory of change

**Answers:** Why do we believe our activities lead to the vision?
**Without it:** projects that are busy but do not move anything — the AI can
see the goal but not the mechanism, so it suggests activity near the goal
instead of activity that causes it.
**Capture:**
- The causal chain, one link per line: _if we do A, then B happens, which
  leads to C._
- Which links are proven and which are assumptions (feeds category 24).

**Example:** _A weekly stall builds the habit → regular buyers make the
growers' income predictable → predictable income lets two more growers join →
the stall's takings pay for a weekday hall._ The second link is proven; the
fourth is an assumption.
**Owner · changes · tier:** Shapers · slow · Tier 2

---

### B. Who we serve

#### 5. Beneficiaries and their needs

**Answers:** Who are the people we exist for, what do they need, and what do
they keep asking for?
**Without it:** projects that serve the org's idea of its people rather than
the people — and no projects at all for needs the Shapers have stopped
noticing.
**Capture:**
- Segments, each with a size in words, a need, and what they do today instead.
- Top needs, ranked, with the evidence for each (a survey, repeated requests,
  attendance).
- Unmet requests: what people asked for that the org does not do.
- Who is under-served or excluded today, and why.

**Example:** _Neighbours who work weekdays and cannot come on Saturday (asked
at the stall most weeks); older residents who cannot carry produce home; the
three growers, who need payment in the same week._
**Owner · changes · tier:** Shapers, with input from members · slow · **Tier 1**

#### 6. Stakeholders

**Answers:** Who else has a stake, what do they want, and what can they block?
**Without it:** projects that ignore the council that licenses the hall, or a
funder whose grant comes with reporting the project forgets.
**Capture:**
- Each stakeholder: who, what they want from us, what we need from them, what
  they can approve or block, and the relationship today (warm, cold, tense).
- Funders, regulators, landlords, partner orgs, the wider neighbourhood.

**Example:** _The council (licences the hall for evening use; slow, formal);
the church that owns the hall (wants it used, worried about noise after 22:00);
the grant funder (wants a short report each quarter)._
**Owner · changes · tier:** Shapers · slow · Tier 2

---

### C. What we are aiming at

#### 7. Objectives and targets

**Answers:** What do we mean to have done soon, and how will we know?
**Without it:** nothing to suggest against. This is where projects come from.
**Capture:** three to seven objectives, each with:
- The outcome, not the activity.
- How it is measured — the metric, its source, and the target.
- The date, and the outside reason for that date if there is one.
- The baseline in words (the number itself is a live reading; category 11).
- Which vision measure it moves.

**Example:** _A weekday hall is open before August — measured as one
weekday session a week for four consecutive weeks; before August because the
church offers the room at a summer rate until then; moves "two days a week"._
**Owner · changes · tier:** Shapers · fast · **Tier 1**

#### 8. Priorities and trade-offs

**Answers:** When two good things compete for the same people, which wins?
**Without it:** a balanced spread of suggestions across every objective, when
the org needs everything on one.
**Capture:**
- The objectives in priority order, with the reason.
- Explicit trade-offs: _we accept X to get Y._
- What is deliberately parked this period.

**Example:** _Saturday first — if the stall slips, nothing else matters. The
hall before more growers, because the hall is where the growers would sell.
Parked: delivery for older residents, until the hall is open._
**Owner · changes · tier:** Shapers · fast · **Tier 1**

#### 9. Strategy and bets

**Answers:** How do we intend to reach the objectives?
**Without it:** suggestions that reach the objective a way the org has
already decided against — a grant application when the bet is that the stall
pays for the hall.
**Capture:**
- Two to six bets, each with why the org believes it.
- Positioning: what makes this org the one to do it.
- What would make us drop each bet.

**Example:** _Small money, many hands — the stall funds the hall, not a grant
round. We would drop it if the stall's takings stay flat for two months._
**Owner · changes · tier:** Shapers · slow · **Tier 1**

#### 10. Anti-goals and red lines

**Answers:** What must never be suggested, however well it would work?
**Without it:** the most expensive kind of wrong suggestion — one that is
sensible on paper and that the Shapers have already said no to.
**Capture:**
- Anti-goals: outcomes the org does not want (_we do not want to become a
  restaurant_).
- Red lines: actions it will not take (_no brand money; no paid advertising_).
- Each with where it came from.

**Example:** _No van, no fleet — rejected in April at 3,500 USDC. No restaurant.
No money that comes with a logo._
**Owner · changes · tier:** Shapers · slow · **Tier 1**

---

### D. Where we are now

#### 11. Current state and baselines

**Answers:** How close is each objective, what is working, what is not?
**Without it:** projects for things already done, or the end state suggested
when the org is still at the beginning.
**Capture:**
- Per objective: where it stands in words, and the live metric it is read from.
- What is working and should be protected.
- What is failing or stuck, and the org's best guess why.
- The org's stage (just started, running, scaling, in trouble).
- Recent changes that matter (a key person left, a partner arrived).

**Example:** _Stall: every Saturday since March, none missed. Growers: three
selling, two visits booked. Hall: never run a weekday evening; no licence for
after 18:00. Stage: running, one season in._
**Owner · changes · tier:** written by Shapers, metrics fetched live · fast ·
**Tier 1**

#### 12. Live work portfolio

**Answers:** What is already being done, by whom, and how is it going?
**Without it:** duplicates; suggestions for people who are already full;
suggestions that ignore a struggling project that needs help more than a new
one is needed.
**Capture:**
- Every live project: what it serves, who holds it, its due date, its health.
- Work happening outside the system — it counts just the same.
- Open proposals not yet decided.
- How much of the org's capacity is already committed.

**Example:** _Saturday stall (Sam, healthy); grower onboarding (Jun, ends 1 Oct,
wobbly — the welcome sheet is late); RIVER vouchers (Maya, ends 1 Sep)._
**Owner · changes · tier:** the system, from the work tree · live · **Tier 1**

#### 13. Ideas and requests pool

**Answers:** What has already been suggested, by whom, and what happened to it?
**Without it:** the AI reinvents ideas members already raised, without credit,
and misses the ones that fit now but did not then.
**Capture:**
- Each idea: what, who raised it, when, which objective it might serve, and its
  status (open, parked until X, declined because Y).
- Requests from beneficiaries and partners (links to category 5).

**Example:** _Lea: a soup night in the hall once it opens (parked until the
hall). Rafi: apply for the city's roof grant (declined — strategy line 1)._
**Owner · changes · tier:** anyone adds; Shapers triage · fast · Tier 2

---

### E. What we have

#### 14. People and capacity

**Answers:** Who can do what, how much, when, and what do they want to do?
**Without it:** projects nobody here can hold; the same two people suggested
for everything; newcomers never given anything.
**Capture, per person:**
- Skills, specific (_grant-writing_, _food-hygiene-certificate_, _Portuguese_).
- What they have done (also visible from history).
- What they **want** to do or learn — aspirations are how routing stops
  ossifying.
- Availability: hours a week, days, time zone, planned absences.
- Their own limit on how much they hold at once.
- Paid, volunteer, or contracted.

**Capture, for the org:**
- Skills the org lacks, and whether it will build, borrow, or buy them.
- Total capacity in words (_about forty volunteer hours a week, most on
  Saturdays_).

**Example:** _Priya: design, Portuguese; wants to run an event; Saturdays only.
Rafi: grant-writing; two evenings a week. Nobody holds a food-hygiene
certificate — the hall will need one._
**Owner · changes · tier:** each person for themselves; Shapers for the org
view · fast · **Tier 1**

#### 15. Money

**Answers:** What can the org afford, how does it fund work, and what money
comes with conditions?
**Without it:** projects the org cannot pay for, or paid for out of money that
is restricted to something else.
**Capture:**
- Funding model: where money comes from and how reliable each source is.
- Money posture in words (_small and frequent, never a big bet_).
- Restricted money: what each restricted source may and may not pay for, and
  what it requires in return.
- Typical spend per project, as a range.
- Who approves spend, and the threshold.
- **Balances and runway are live readings** — fetched at the moment of use,
  never written down (see [Written or fetched live](#written-or-fetched-live)).

**Example:** _Income: stall takings, weekly. One grant (restricted to the hall,
quarterly report). A project normally costs under 200 USDC. Over 500 needs all
Shapers._
**Owner · changes · tier:** Shapers; balances live · slow (posture) / live
(balances) · Tier 2

#### 16. Assets and tools

**Answers:** What does the org already own or have access to?
**Without it:** projects to acquire what the org already has; projects that
ignore a cheaper path through an existing asset.
**Capture:**
- Spaces (with when they are available and on what terms).
- Equipment, vehicles, stock.
- Digital: repositories, websites, domains, accounts, data sets, mailing lists
  and their size band.
- Licences, permits, certifications, insurance — with expiry dates.
- Brand, content, intellectual property.

**Example:** _Two market gazebos and a card reader. The church hall, Tuesdays
and Thursdays from 17:00, at a summer rate until August. Stall trading permit,
renews in March. A mailing list of about 300 neighbours._
**Owner · changes · tier:** Shapers · slow · Tier 2

#### 17. Relationships and network

**Answers:** Who outside the org can we call on, and for what?
**Without it:** projects that build from scratch what one phone call to a
friendly org would provide; suggestions that miss the obvious partner.
**Capture:**
- Partners, suppliers, allies, advisors, champions — each with what they can
  offer, who holds the relationship, and its state.
- Peer organisations doing similar work, and what they have tried.

**Example:** _The Ferreira family (grower; also has a refrigerated van they
sometimes lend). The food bank two streets over (ran a weekday hall last year —
ask them about the licence). Tomasz knows the council's licensing officer._
**Owner · changes · tier:** Shapers and members · slow · Tier 2

---

### F. What bounds us

#### 18. Constraints

**Answers:** What rules, laws, and limits apply, whether we like them or not?
**Without it:** projects that cannot legally or practically happen — an evening
food event with no licence and no hygiene certificate.
**Capture:**
- Legal and regulatory (licensing, food safety, data protection, employment,
  tax, the org's legal form).
- Policy and contract terms (the hall's noise curfew, the grant's conditions).
- Safety and accessibility requirements.
- Practical limits (no storage, no vehicle, no one free on weekdays before
  17:00).

**Example:** _Selling cooked food needs a hygiene certificate per session. The
hall closes at 22:00. The org is an unincorporated association — it cannot sign
a lease._
**Owner · changes · tier:** Shapers · slow · Tier 2

#### 19. Commitments and obligations

**Answers:** What has the org already promised, to whom, by when?
**Without it:** suggestions that look free but collide with a promise; missed
deliverables that should have been projects in their own right.
**Capture:**
- Each commitment: to whom, what, by when, what happens if it is missed.
- Recurring obligations (reports, renewals, annual meetings, filings).

**Example:** _Quarterly report to the grant funder, next due 30 June. Promised
the church a deep clean after each session. Annual members' meeting in
November._
**Owner · changes · tier:** Shapers · fast · Tier 2

#### 20. Calendar and deadlines

**Answers:** What dates shape when work can or must happen?
**Without it:** projects dated into the dead season, or dated after the window
they depend on has closed.
**Capture:**
- Seasons and their effect (growing season, holidays, school terms).
- External windows: funding rounds, permit cycles, events, price changes.
- The org's own rhythms: meetings, review cycles, busy weeks.

**Example:** _Growing season April–October; the stall slows in winter. The
council's licensing committee meets monthly — applications close the first
Monday. The summer hall rate ends in August._
**Owner · changes · tier:** Shapers · slow · Tier 2

#### 21. External dependencies

**Answers:** What do our plans wait on that we do not control?
**Without it:** projects suggested in the wrong order, or projects that stall
for months on someone else's yes.
**Capture:**
- Each dependency: on whom, for what, expected when, how certain, and what
  happens if it does not come.

**Example:** _The weekday hall waits on the council's licence decision (applied
for; decision expected within six weeks; if refused, a daytime-only
re-application)._
**Owner · changes · tier:** project holders and Shapers · fast · Tier 2

#### 22. Risks and risk appetite

**Answers:** What could go badly wrong, and how much risk will the org take on?
**Without it:** bold projects in a cautious org, timid ones in a bold one, and
no projects at all for the risk that would sink everything.
**Capture:**
- Top risks: what, how likely, how bad, what reduces it.
- Single points of failure (one person, one supplier, one source of money).
- Risk appetite in words, by area (money, reputation, legal, people).

**Example:** _Sam is the only person who can run the stall alone. If the
Ferreiras stop, half the produce goes. Appetite: low on money, high on trying
new formats if they cost little._
**Owner · changes · tier:** Shapers · slow · Tier 3

---

### G. What we know

#### 23. History and lessons

**Answers:** What has the org tried, what happened, and what did it learn?
**Without it:** the AI suggests last spring's failure again, confidently.
**Capture:**
- Past projects: what, who, outcome, and the lesson in one line.
- Experiments that failed, and why.
- Decisions taken and their reasons, especially rejections.
- Ideas declined, with whether and when they may be revisited.

**Example:** _A pop-up evening in February drew eight people — too cold, no
publicity. The van was rejected (cost, and "two days a week, not a fleet").
The voucher design shipped on time and was paid the same week._
**Owner · changes · tier:** the system, from closed work and decisions;
lessons written by holders · fast · **Tier 1** once anything has closed

#### 24. Open questions and assumptions

**Answers:** What does the org not know yet that decides what it should do?
**Without it:** the AI suggests the build when the right project is the test —
the installation before the site survey, the rollout before the pilot.
**Capture:**
- Each question, the decision it unlocks, and how it could be answered.
- Each assumption the plan rests on, how confident the org is, and what would
  disprove it.

**Example:** _Will weekday buyers come? (decides whether the hall runs one
evening or two; a four-week trial answers it). Assumption: the stall's takings
can cover the hall rent — untested past August._
**Owner · changes · tier:** Shapers · fast · **Tier 1**

#### 25. Environment and landscape

**Answers:** What is happening around the org that changes what is worth doing?
**Without it:** projects that duplicate what someone else already offers, or
that miss a change everyone outside the org can see.
**Capture:**
- Alternatives people use instead of us, and how we differ.
- Trends, policy changes, new entrants, closures.
- Opportunities (a new fund, an empty shop) and threats.

**Example:** _A discount supermarket opens on the high street in September.
The city announced a fund for neighbourhood food projects. The food bank stops
its weekday hall in July._
**Owner · changes · tier:** Shapers and members · fast · Tier 3

#### 26. Domain knowledge and playbooks

**Answers:** How is this kind of work done well, by people who have done it?
**Without it:** generic projects ("make a plan", "do research") instead of the
licence before the opening night and the hygiene course before the first
cooked session.
**Capture:**
- The org's own playbooks: how it runs a stall day, onboards a grower, opens a
  new session.
- Standards and checklists that apply in the domain.
- External references the org trusts.
- A glossary of the org's own terms (category 29).

**Example:** _Our grower onboarding checklist: visit, sample, price agreement,
first stall, first payment the same week. The council's guide to temporary
food events._
**Owner · changes · tier:** members who did the work · slow · Tier 3

---

### H. How we work

#### 27. Operating model and project norms

**Answers:** What does a project look like here?
**Without it:** suggestions the right shape for another org — a six-month
programme where projects are four weeks, a team project where one person holds
each.
**Capture:**
- Typical project length and size (one holder, a few helpers).
- What "done" means here and how work is reviewed.
- Preferred patterns (pilot first, small and reversible, document as you go).
- Where work lives (rooms, repositories, shared documents).
- Working language(s) and time zone.

**Example:** _A project is four to eight weeks and one holder. Everything new
starts as a four-week trial. Work is reviewed at the monthly Shapers call.
English and Portuguese; Europe/Lisbon._
**Owner · changes · tier:** Shapers · slow · Tier 2

#### 28. Governance and decision rights

**Answers:** Who decides what, by what rule, and how fast?
**Without it:** suggestions routed to the wrong person, or sized beyond what
can be decided before the date that matters.
**Capture:**
- Who can approve a project, name a holder, change direction, spend money.
- The rule for each (majority, all, a number) and the decision window.
- Escalation: what goes to all members, what to an outside body.

**Example:** _Two Shapers; projects need majority (so both); seven-day
decision window; spending over 500 USDC needs all._
**Owner · changes · tier:** Shapers · slow · **Tier 1** (the system holds it)

#### 29. Culture, language, and glossary

**Answers:** How does this org talk, and what do its words mean?
**Without it:** suggestions that read as foreign — the wrong register, the
wrong language, a term the org uses differently.
**Capture:**
- Language(s) for drafts.
- Tone (plain, formal, playful).
- What members find energising and what drains them.
- Glossary: the org's own names for things (_RIVER_ = the voucher currency;
  _the hall_ = the church hall on Elm Street).

**Owner · changes · tier:** Shapers · stable · Tier 2

---

### I. Feedback on suggestions

#### 30. What the org did with past suggestions

**Answers:** Which suggestions did the org take, change, or refuse — and why?
**Without it:** the AI asks the same thing twice and never learns this org's
taste.
**Capture:**
- Every suggestion's outcome: accepted, amended (with the edit), declined
  (with the reason), ignored.
- Patterns: what kinds of suggestions this org values, and which it never
  takes.

**Example:** _Grant suggestions: three declined, same reason (strategy line
1). Holder suggestions for Priya: accepted when Saturday-only._
**Owner · changes · tier:** the system, from decisions · live · **Tier 1** (it
starts filling with the first suggestion)

---

## Tiers: what to have when

### Tier 1 — before the first suggestion

Purpose (1), vision (2), beneficiaries and needs (5), objectives and targets
(7), priorities (8), strategy (9), anti-goals and red lines (10), current state
(11), live portfolio (12), people and capacity (14), open questions (24),
governance (28). History (23) and feedback (30) join as soon as anything closes
or is decided.

With Tier 1 the AI can find the gap, generate, filter the worst mistakes,
choose the first step, and name a holder.

### Tier 2 — within the first month

Values (3), theory of change (4), stakeholders (6), ideas pool (13), money
(15), assets (16), relationships (17), constraints (18), commitments (19),
calendar (20), dependencies (21), operating norms (27), culture and glossary
(29).

With Tier 2 the suggestions are sized, dated, legal, affordable, and use what
the org already has.

### Tier 3 — as the org matures

Risks and appetite (22), environment (25), domain playbooks (26).

With Tier 3 the suggestions anticipate — the risk before it lands, the
opportunity before it closes, the playbook step nobody remembered.

---

## Written or fetched live

The same rule holds for every category: **write down what the org believes and
knows; fetch what can be counted at the moment it is used.** A number written
into context is recited after it stops being true.

| Freshness  | Categories                                                                                         | How it reaches the AI                       |
| ---------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| **stable** | 1 purpose, 2 vision, 3 values, 29 culture                                                          | written; reviewed yearly                    |
| **slow**   | 4 theory of change, 5 beneficiaries, 6 stakeholders, 9 strategy, 10 anti-goals, 15 money posture, 16 assets, 17 relationships, 18 constraints, 20 calendar, 22 risks, 26 playbooks, 27 norms, 28 governance | written; reviewed each quarter or when something changes |
| **fast**   | 7 objectives, 8 priorities, 11 current state (prose), 13 ideas, 14 people, 19 commitments, 21 dependencies, 23 lessons, 24 open questions, 25 environment | written; reviewed monthly, or on the event that changes it |
| **live**   | 11 metrics, 12 portfolio, 15 balances, 30 feedback                                                 | fetched from the system at the moment of use |

Not everything goes into every call. **Always send the map; send the contents
on demand** (Organizational Intelligence § 3): a one-line index of every
category, the Tier 1 categories in full, and the rest loaded by name when a
suggestion needs them — the money category when sizing, constraints when
filtering, relationships when staffing.

---

## Intake template

One structure for the founder or Shapers to fill in, in conversation with the
agent or directly. Every field is optional except Tier 1; every entry records
who wrote it and when.

```yaml
purpose:            # 1 · Tier 1
  what_we_do:
  for_whom: []
  problem:
  not_this: []
  origin:
vision:             # 2 · Tier 1
  picture:
  measures: []
  horizon:
values:             # 3
  - principle:
    rules_out:
theory_of_change:   # 4
  - link:
    status: proven | assumption
beneficiaries:      # 5 · Tier 1
  - segment:
    need:
    today_they:
    evidence:
  unmet_requests: []
stakeholders:       # 6
  - who:
    wants:
    we_need:
    can_block:
    relationship:
objectives:         # 7 · Tier 1
  - outcome:
    metric:
    target:
    by:
    why_this_date:
    baseline_in_words:
    moves_vision_measure:
priorities:         # 8 · Tier 1
  order: []
  trade_offs: []
  parked: []
strategy:           # 9 · Tier 1
  - bet:
    why:
    drop_if:
anti_goals:         # 10 · Tier 1
  - never:
    because:
current_state:      # 11 · Tier 1
  per_objective: {}
  working: []
  stuck: []
  stage:
  recent_changes: []
ideas:              # 13
  - idea:
    raised_by:
    serves:
    status: open | parked | declined
    because:
people:             # 14 · Tier 1 — each person writes their own
  - who:
    skills: []
    wants_to: []
    hours_per_week:
    days: []
    timezone:
    limit:
    basis: volunteer | paid | contracted
  org_lacks: []
money:              # 15 — no balances
  sources: []
  posture:
  restricted: []
  typical_project_cost:
  approval:
assets: []          # 16 — with availability, terms, expiry
relationships: []   # 17 — who, offers, held_by, state
constraints: []     # 18
commitments: []     # 19 — to_whom, what, by, if_missed
calendar: []        # 20
dependencies: []    # 21 — on, for, expected, certainty, if_not
risks:              # 22
  top: []
  single_points_of_failure: []
  appetite: {}
lessons: []         # 23 — what, outcome, lesson, revisit
open_questions:     # 24 · Tier 1
  - question:
    unlocks:
    answered_by:
assumptions: []
environment: []     # 25
playbooks: []       # 26
norms:              # 27
  project_length:
  holders:
  done_means:
  patterns: []
  language: []
  timezone:
glossary: {}        # 29
```

Categories 12, 28, and 30 are not filled in; the system already knows them.

---

## How this maps to Buzz today

The current model holds direction as four short, versioned texts and treats
that limit as an invariant (Organizational Intelligence § 2: _L3 must stay
small enough that a person could read all of it in an afternoon … a fifth
artifact is a protocol change to argue for_). This document argues for that
change. The tension is real and worth stating plainly:

- **What the invariant protects** — every belief is confirmed by Shapers,
  versioned, and auditable; nothing grows into a corpus nobody reads; nothing
  stale is recited. Those properties should survive any expansion: each new
  category should be confirmed, versioned, owned, and small.
- **What it costs** — most of the categories above have no home, so the agent
  suggests projects without knowing the org's constraints, commitments,
  assets, calendar, or open questions. The AI evaluation's own bar — _drafts
  that read like the work of someone who has run this kind of organisation
  before_ — needs them.
- **The path the design already names** — Organizational Intelligence § 3:
  _if L3 ever outgrows four short texts, the index-then-select pattern
  returns: send one line per artifact, load bodies by name. Nothing else in
  the design changes._

| #  | Category                    | Home in Buzz today                                                   | Status   |
| -- | --------------------------- | -------------------------------------------------------------------- | -------- |
| 1  | Purpose                     | `39100` mission (chat keeps it to one sentence)                      | partial  |
| 2  | Vision and horizon          | `39100` vision (one sentence from chat)                              | partial  |
| 3  | Values and principles       | —                                                                    | missing  |
| 4  | Theory of change            | —                                                                    | missing  |
| 5  | Beneficiaries and needs     | mission text, if written there                                       | partial  |
| 6  | Stakeholders                | —                                                                    | missing  |
| 7  | Objectives and targets      | `39100` objectives lines; `date` field unset from chat; no metric or target fields | partial |
| 8  | Priorities and trade-offs   | strategy prose only                                                  | partial  |
| 9  | Strategy and bets           | `39100` strategy lines                                               | exists   |
| 10 | Anti-goals and red lines    | strategy lines that refuse                                           | partial  |
| 11 | Current state and baselines | per-project health read (`50101`); no org-level metrics              | partial  |
| 12 | Live work portfolio         | work tree (`39101`), open proposals (`39102`)                        | exists   |
| 13 | Ideas and requests pool     | chat; drafts from talk (J7)                                          | partial  |
| 14 | People and capacity         | `39105` profile: about, skills, open limit; no availability, aspirations, time zone | partial |
| 15 | Money                       | none in v1 by design; treasury later                                 | missing  |
| 16 | Assets and tools            | project homes (room, repository); nothing else                       | partial  |
| 17 | Relationships and network   | —                                                                    | missing  |
| 18 | Constraints                 | —                                                                    | missing  |
| 19 | Commitments and obligations | —                                                                    | missing  |
| 20 | Calendar and deadlines      | objective dates only                                                 | partial  |
| 21 | External dependencies       | `after` between sibling tickets; nothing external                    | partial  |
| 22 | Risks and risk appetite     | —                                                                    | missing  |
| 23 | History and lessons         | closed roots, review briefs (J3b), proposal history; agent reads the last two closed | partial |
| 24 | Open questions and assumptions | —                                                                 | missing  |
| 25 | Environment and landscape   | —                                                                    | missing  |
| 26 | Domain knowledge            | the model's own knowledge                                            | missing  |
| 27 | Operating model and norms   | decision and offer windows in `39103`                                | partial  |
| 28 | Governance                  | Shapers and rules in `39103`                                         | exists   |
| 29 | Culture, language, glossary | language detected from direction                                     | partial  |
| 30 | Feedback on suggestions     | draft outcomes (`39104`, L4)                                         | exists   |

Four categories exist, fifteen are partial, eleven are missing. The largest
gains for the least change, in order: open questions (24) and priorities (8),
which decide the first step and the focus; constraints (18) and commitments
(19), which stop impossible and colliding suggestions; and the missing person
fields in 14 — availability and what each person wants to do.
