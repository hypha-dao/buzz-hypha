# The Intelligent Organization — on Buzz

This folder is the product and architecture record for the **intelligent
organization**: a way of running a group where the AI drafts and people decide,
work is offered rather than assigned, and every belief the organization holds
traces back to a receipt.

It is being built **into Buzz**. One organization is one Buzz community; the
relay's signed event log is its substrate; the org agent is a member with its
own key; the five doors are a feature of the desktop. The design was first
worked out for [`hypha-dao/hypha-web`](https://github.com/hypha-dao/hypha-web)
and retargeted here on 2026-09-14.

A clickable prototype of the five doors lives in
[`prototypes/org-preview`](../../prototypes/org-preview/README.md) and at
[hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app). It calls
the board door _Projects_; on Buzz it is **Work** (the desktop already has a
git Projects surface).

## The model, in one paragraph

Direction is four short, versioned texts — **mission, vision, objectives,
strategy** — each confirmed by Shapers. Work is **one recursive tree**: a
project is a ticket with nothing above it; whoever holds a piece of work can
split it and offer the pieces; only the named person accepts; only the holder
marks done; done cascades up, never down. **No money lives on work**: pay is
an out-only proposal the Shapers decide and settle outside Buzz. Exactly **five
things are proposals** — project, DRI, money, direction, join. The **org agent**
listens, drafts, and routes each draft to the one person who can make it real;
it has no command that changes state. Every state change is a signed command
the relay executes; every claim carries a receipt.

## Read in this order

| #   | Document                                                                      | Status                    | What it answers                                                                                                                       |
| --- | ----------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | [What it is](./product/intelligent-org-features.md)                           | **source of truth**       | The features. The two rules, three roles, five doors, what users can do. When any document disagrees with this one, this one wins.   |
| 2   | [Journeys](./product/intelligent-org-journeys.md)                             | current                   | Step-by-step through the desktop, by role — DRI, Shaper, member, and the org agent's own loop.                                        |
| 3   | [Protocol](./architecture/intelligent-org-protocol.md)                        | current — **start here to build** | The event kinds, tags, content schemas, state machines, and relay rules. The contract between relay, desktop, CLI, and agent. |
| 4   | [Design](./architecture/intelligent-org-design.md)                            | current                   | Why the protocol is shaped that way: the four memory layers on Buzz, the work tree, the org agent (HEAR → THINK → ROUTE), surfaces, build order, risks. |
| 5   | [Organizational Intelligence](./architecture/organizational-intelligence.md)  | current (revised)         | The memory model underneath: L1 substrate, L2 ledger, L3 beliefs, L4 outcomes; context budget; decision rights.                       |
| 6   | [Phase 0](./plans/intelligent-org-phase-0.md)                                 | current — **the plan**    | Dogfood: run the build of the product inside Buzz with three doors and the agent. Where the code goes, in what order, and what we measure. |
| 7   | [AI evaluation](./plans/intelligent-org-ai-evaluation.md)                     | current                   | Pass/fail bars and the harness for the four agent moves: direction → projects, project → tickets, completion → next, health.         |
| 8   | [Current state](./architecture/intelligent-org-current-state.md)              | current                   | What Buzz already has that the model needs, what is designed and not built, and what to verify in code first.                        |
| —   | [Exploration](./product/intelligent-org-exploration.md)                       | historical                | The note that chose Buzz as the foundation. The model it sketches was replaced on 11 September.                                        |

Documents 1–2 are substrate-neutral product. Documents 3–8 are Buzz-specific.

## For development

Start with the **Protocol**, then **Phase 0** § Technical architecture, then
**Design** § Build order. The first slice is the relay: kinds in `buzz-core`,
command handlers in the command executor, relay-signed state, `io_*`
projections, the `io_scheduler` job, and `buzz org` in the CLI — all testable
from `buzz-test-client` before any UI or model exists. The
[Current state](./architecture/intelligent-org-current-state.md) lists five
facts to verify in code before that slice starts.

Terminology that differs from earlier drafts, so grep does not mislead you:

| Earlier drafts                       | Now                                                              |
| ------------------------------------ | ---------------------------------------------------------------- |
| mandate, pot, steward, envelope      | project (a root of the tree), no money on it, its DRI            |
| org brief                            | four direction artifacts, versioned independently                |
| Projects door, All Work, Org door    | **Work** door, **Overview** door                                 |
| Space                                | community                                                        |
| Personal Assistant                   | your DM with the org agent (the name is still used for the flow) |
| assigned                             | held / accepted (work is offered; only a `dri` vote names)       |
| SENSE → THINK → ROUTE → PUBLISH      | HEAR → THINK → ROUTE (confirm is a human command, not a pass)    |
| `gap_key`, `dismissed`               | `gap` tag, `declined`                                            |

## Archive

Earlier drafts kept because they hold reasoning that fed the current model.
None is a spec.

- [`archive/intelligent-org-buzz-additions.md`](./archive/intelligent-org-buzz-additions.md) —
  the first Buzz sketch (2026-08-26): six features as event kinds, on the
  mandate–pot model. Superseded by Protocol and Design.
- [`archive/user-journeys.md`](./archive/user-journeys.md) — first journeys
  draft (2026-08-20): roles, decision tiers, funding. Superseded by Journeys.
- [`archive/user-journeys-ux.md`](./archive/user-journeys-ux.md) — who-sees-what
  tables (2026-08-22), including the **North Workshop** example.

## Provenance

Copied from two branches of `hypha-dao/hypha-web` and retargeted to Buzz, both
on 2026-09-14:

- `docs/intelligent-org` — everything under `product/`, `architecture/`, `plans/`.
- `docs/intelligent-org-architecture` — the two archive journey files.
- `feat/org-preview` — `apps/org-preview`, now `prototypes/org-preview`.

Paths inside archive documents that say `apps/org-preview` refer to the
hypha-web layout; in this repo the same code is at `prototypes/org-preview`.
The hypha-web-only references (Space Memory panel, documents/media store,
Drizzle tables, Privy, Vercel crons) have been removed from the live documents
and survive only in the archive.
