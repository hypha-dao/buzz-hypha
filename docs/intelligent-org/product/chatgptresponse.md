Yes. I’d think of this less as “give the AI a big context” and more as **build an organizational state model** that the AI can reason over.

The AI needs to answer two different questions:

1. **What should we do?** → recommend projects.
2. **How do we actually execute it?** → turn an approved project into tickets.

A useful architecture would look like:

**Organization state → AI identifies opportunities → Project proposal → Project decomposition → Tickets → Execution feedback → updated organization state**

### 1. Organization context

This is the relatively stable stuff:

* **Mission / objectives**

  * What the organization exists to achieve
  * Current strategic goals
  * Long-term vision
* **Strategy**

  * Current strategic priorities
  * Things explicitly *not* being pursued
  * Competitive positioning
* **Capabilities**

  * What the organization is good at
  * Technical capabilities
  * People/roles
  * Existing infrastructure
  * Access to capital, users, distribution, partnerships, etc.
* **Constraints**

  * Budget
  * Available people/hours
  * Deadlines
  * Regulatory/legal constraints
  * Technical limitations
  * Dependencies
* **Resources**

  * Money
  * People
  * Compute
  * Existing assets
  * Data
  * Intellectual property
  * Customer relationships

The important thing is that the AI shouldn't just know **what resources exist**, but their approximate availability.

> Engineering: 3 people, ~60% available
> Marketing: 1 person, ~30% available
> Budget: €50k
> Deadline: launch before December

---

### 2. Current organizational state

This is probably the most important part.

The AI should know what is **currently happening**.

For example:

* Active projects
* Project status
* Current objectives
* Completed projects
* Failed/abandoned projects
* Current tickets
* Blocked tickets
* Upcoming deadlines
* Current experiments
* Recent decisions
* Known problems

And importantly:

**Why are things in their current state?**

Instead of:

> Project X — 65% complete

give it:

> Project X — 65% complete
> Goal: acquire first 100 customers
> Current result: 43 customers
> Main blocker: onboarding conversion 4%
> Hypothesis: users don't understand the product value proposition
> Next experiment: test new onboarding flow

That lets the AI reason about **what should happen next**, rather than merely reading project names.

---

### 3. Goals + measurable outcomes

I'd make goals structured.

For example:

```text
Goal:
Increase monthly revenue

Target:
€100k MRR

Current:
€63k MRR

Deadline:
2027-01-01

Priority:
High

Strategy:
- Improve retention
- Increase conversion
- Expand enterprise sales

Metrics:
MRR
churn
activation rate
ARPU
```

Then projects can explicitly connect to goals:

```text
Project:
Enterprise onboarding redesign

Contributes to:
Increase MRR

Expected impact:
+€15k–€30k MRR

Confidence:
0.65
```

This is extremely useful because otherwise the AI will generate lots of **interesting but strategically irrelevant projects**.

---

### 4. Problems / opportunities

I'd actually make this a first-class object.

The organization should have a constantly evolving list of:

**Problems**

* Customer churn is increasing
* Deployment takes 3 days
* Sales pipeline is weak
* Engineers spend too much time on support

**Opportunities**

* Existing customers asking for feature X
* New API becoming available
* Competitor weakness
* New market opening
* Existing technology could enable a new product

Each should have:

```text
Problem
Description
Evidence
Severity
Frequency
Affected area
Root cause hypothesis
Known solutions
Unknowns
```

This gives the AI something much better to generate projects *from*.

---

### 5. Knowledge / evidence

This is where I'd avoid dumping the entire company's knowledge base into every prompt.

Instead, have the organization context reference:

* Customer feedback
* Analytics
* Research
* Market intelligence
* Competitor information
* Internal documents
* Meeting decisions
* Experiments
* Technical documentation
* Historical project results

And attach **evidence to claims**.

For example:

> Customer onboarding is the primary growth bottleneck.

Evidence:

* Analytics: 72% drop-off during onboarding
* 14 customer interviews
* Support tickets #381, #392, #401
* Experiment E-17

This makes the AI's recommendations much more defensible.

---

## 6. People / agents

This becomes particularly important if your AI is actually going to create tickets.

It should know:

```text
Person:
Alice

Role:
Backend engineer

Skills:
Python
Postgres
AWS

Capacity:
60%

Current work:
Project A
Project B

Preferences:
Backend > frontend

Authority:
Can approve technical changes
Cannot approve spending > €5k
```

Potentially also:

* Who can make decisions
* Who owns what
* Who reviews what
* Who is overloaded
* Who has particular expertise

This allows the AI to generate **assignable work**, rather than generic Jira-style tickets.

---

# 7. Projects

I'd give projects their own structured schema.

Something like:

```text
Project
├── Objective
├── Problem
├── Expected outcome
├── Strategic goals
├── Owner
├── Participants
├── Required capabilities
├── Required resources
├── Dependencies
├── Deadline
├── Estimated effort
├── Expected value
├── Risk
├── Confidence
├── Evidence
├── Status
└── Tasks
```

But there's one particularly important field:

### Why this project?

The AI should be able to explain:

> We recommend this project because X problem currently limits goal Y, and this project has a high expected impact relative to its estimated cost.

That makes the organization understandable to itself.

---

# 8. Decision history

This is easy to overlook but **very important**.

The AI needs to know:

```text
Decision:
Do not build mobile app

Date:
2026-09-12

Decision maker:
Vlad

Reason:
Mobile users represent <3% of usage and engineering capacity is constrained.

Reconsider when:
Mobile traffic >15%
```

Otherwise six months later the AI will happily recommend:

> Build a mobile app 🚀

even though humans explicitly decided against it.

So I'd maintain:

**Decisions + rationale + conditions for reconsideration.**

---

# 9. Execution history

The AI should learn from what the organization actually does.

For every project:

```text
Expected:
€20k impact
2 weeks

Actual:
€7k impact
5 weeks

Why:
Integration complexity underestimated
```

This allows the AI to develop an implicit model of:

> “This organization consistently underestimates backend integration projects by ~2×.”

That's incredibly valuable for future project recommendations.

---

# 10. Project → ticket generation

Once a project is approved, I wouldn't simply ask the AI:

> "Create tickets."

Give it an explicit decomposition model.

For example:

```text
Project
    ↓
Milestones
    ↓
Deliverables
    ↓
Tasks
    ↓
Subtasks
```

A ticket should contain enough information to actually execute it:

```text
Ticket

Title:
Implement onboarding event tracking

Objective:
Measure where users drop out during onboarding.

Context:
...

Acceptance criteria:
- signup_started event exists
- signup_completed event exists
- events contain user_id
- dashboard displays conversion

Dependencies:
- Analytics infrastructure

Estimated effort:
4h

Owner:
Alice

Priority:
High

Parent:
Enterprise onboarding redesign
```

The key is that **tickets inherit context from the project**.

So you don't need to repeat the entire organizational context inside every ticket.

---

# The context hierarchy I'd use

I'd probably structure your system roughly like this:

```text
ORGANIZATION
│
├── Mission
├── Strategy
├── Goals
├── Constraints
├── Resources
├── Capabilities
│
├── PEOPLE
│
├── KNOWLEDGE
│   ├── Research
│   ├── Customer feedback
│   ├── Market data
│   └── Internal documents
│
├── PROBLEMS
│
├── OPPORTUNITIES
│
├── DECISIONS
│
├── PROJECTS
│   ├── Active
│   ├── Planned
│   ├── Completed
│   └── Failed
│
└── EXECUTION
    ├── Tickets
    ├── Results
    └── Metrics
```

Then your AI operates something like:

```text
                  ORGANIZATION STATE
                         │
          ┌──────────────┼──────────────┐
          ↓              ↓              ↓
       Problems      Opportunities    Goals
          │              │              │
          └──────────────┼──────────────┘
                         ↓
                 AI PROJECT ENGINE
                         │
             ┌───────────┴───────────┐
             ↓                       ↓
       Project proposals        Rejected ideas
             │
             ↓
        Human / AI decision
             │
             ↓
          PROJECT
             │
       ┌─────┴─────┐
       ↓           ↓
   Milestones   Dependencies
       │
       ↓
     Tickets
       │
       ↓
    Execution
       │
       ↓
     Results
       │
       └──────────→ ORGANIZATION STATE
```

### One important design principle

Don't make the AI's context just a **knowledge dump**.

Make it a **state representation**.

The distinction is:

> **Knowledge:** “We have 3 engineers.”

vs.

> **State:** “We have 3 engineers; 2 are committed to Project A until November 15, one has ~40% capacity, and backend capacity is currently the bottleneck.”

The second one allows an AI to make decisions.

And ultimately I think your core object should be something like:

**`Project = proposed change to the organization's state`**

Then the AI can evaluate projects according to:

**Expected impact × strategic alignment × confidence / cost × risk**

and, after selecting one, derive the concrete work required to make that state transition happen.

That gives you a pretty clean foundation for an **AI organization/project manager**, rather than just an AI that generates Jira tickets.
