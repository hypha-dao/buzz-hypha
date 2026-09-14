# The Intelligent Organization — Hypha on Buzz

This folder is the product and architecture record for the **intelligent
organization**: a way of running a group where the AI drafts and people decide,
work is offered rather than assigned, and every belief the organization holds
traces back to a receipt.

It was designed and prototyped in [`hypha-dao/hypha-web`](https://github.com/hypha-dao/hypha-web)
and moved here so the features can be built on the Buzz substrate — one relay,
one identity model, one signed event log for people and agents alike.

A clickable prototype of the five doors lives in
[`prototypes/org-preview`](../../prototypes/org-preview/README.md) and at
[hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app).

## Read in this order

| #   | Document                                                                              | What it answers                                                                                                                  |
| --- | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| 1   | [What it is](./product/intelligent-org-features.md)                                   | Source of truth. The five doors, the two rules, what users can do.                                                               |
| 2   | [Journeys](./product/intelligent-org-journeys.md)                                     | Four people, one month, all five doors. What each person sees and does.                                                          |
| 3   | [Design](./architecture/intelligent-org-design.md)                                    | How to build it: tables, the agent pipeline (SENSE → THINK → ROUTE → PUBLISH), surfaces, guardrails.                             |
| 4   | [Organizational Intelligence](./architecture/organizational-intelligence.md)          | The memory model underneath: L1 substrate, L2 ledger, L3 beliefs, L4 outcomes.                                                   |
| 5   | [Buzz additions](./product/intelligent-org-buzz-additions.md)                         | What the Buzz fork needs on top of upstream: event kinds, doors, the agent as a member.                                          |
| 6   | [Phase 0](./plans/intelligent-org-phase-0.md)                                         | Dogfooding plan: run the build of the product inside a minimal real app. Includes the technical architecture.                    |
| 7   | [AI evaluation](./plans/intelligent-org-ai-evaluation.md)                             | Pass/fail bars and the test harness for the four agent moves: direction → projects, project → tickets, completion → next, health. |
| 8   | [Current state](./architecture/intelligent-org-current-state.md)                      | What existed on `hypha-web` when the design was frozen, and the gap to close.                                                    |
| —   | [Exploration](./product/intelligent-org-exploration.md)                               | Early notes that fed the features doc. Historical.                                                                               |

## Archive

Earlier drafts kept because later documents cite them and one holds material
that was not carried forward:

- [`archive/user-journeys.md`](./archive/user-journeys.md) — first journeys draft; superseded by
  the Journeys document above.
- [`archive/user-journeys-ux.md`](./archive/user-journeys-ux.md) — who-sees-what tables,
  including the **North Workshop** example.

## Provenance

Copied from two branches of `hypha-dao/hypha-web`:

- `docs/intelligent-org` — everything under `product/`, `architecture/`, `plans/`.
- `docs/intelligent-org-architecture` — the two archive files.
- `feat/org-preview` — `apps/org-preview`, now `prototypes/org-preview`.

Paths inside the documents that say `apps/org-preview` refer to the hypha-web
layout; in this repo the same code is at `prototypes/org-preview`. Two links
point back to hypha-web documents that were not moved because they describe
hypha-web-only surfaces (the Space Memory panel and the documents/media store).
