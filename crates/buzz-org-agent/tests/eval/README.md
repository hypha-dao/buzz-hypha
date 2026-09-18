# Evaluation fixtures (Development plan E-1)

The synthetic organizations the org agent's evaluation harness and
`OrgState::apply` tests read. Everything under `fixtures/` except
`generate.mjs`, `lib/`, and this file is **generated** — from the
prototype's [`data.ts`](../../../../prototypes/org-preview/src/lib/data.ts)
through the mapping table in
[Prototype map § 3](../../../../docs/intelligent-org/product/intelligent-org-prototype-map.md)
— and checked in. Edit the generator or `data.ts`, never the JSON.

```bash
just org-fixtures          # regenerate in place (~30 s)
just org-fixtures-check    # CI: regenerate in memory and diff against the checked-in files
cargo test -p buzz-org-agent   # every fixture loads, verifies, decodes, round-trips
```

The generator is plain Node (≥ 23.6, for type stripping of `data.ts`) with
no dependencies; `prototypes/org-preview` stays outside the pnpm workspace.
Its output is a pure function of `data.ts`, `lib/constants.mjs`, and the
locale tables: pubkeys derive from `sha256("io-fixture:" + name)`, object ids
from stable keys, event ids follow, and Schnorr signatures use a fixed
nonce — so a regeneration with no input change is byte-identical.

## Layout

```
fixtures/
  orgs/<org>/seed.json            River Commons, Hypha Energy, cold start — signed
                                  events in relay order, one per line
  orgs/<org>/seed.<locale>.json   pt (River) / es (Energy): text, brief, title,
                                  and messages translated; ids, pubkeys,
                                  timestamps, and every non-event-id tag shared
  orgs/<org>/manifest.json        the ids a case needs by prototype name: people,
                                  Shapers, rooms, direction heads, items,
                                  proposals, per-kind counts
  orgs/<org>/health-gold.json     one 50101 per project with `health` — the gold
                                  for move 4's judge, never state
  sequences/<name>/sequence.json  a multi-step brief, its gold plan (order,
                                  `after`, the gate, `held`), `why_gold`, and two
                                  gate outcomes with the next wave each calls for
  sequences/<name>/*.json         snapshot deltas layered on the org's seed:
                                  before → gate → outcome-a | outcome-b
  who-is-needed/<org>.json        per skill: who in the seed has it, how many open
                                  pieces they hold, who is at `open_limit`, and the
                                  answer a holder suggestion must give
```

The loader is `buzz_org_agent::fixtures` (`load_org`, `load_sequence`,
`load_who_is_needed`, `roundtrip`). `tests/eval_fixtures.rs` is the proof
that the checked-in files are what the documents ask for.

## What the seed is, and is not

- **State events are relay-signed** (`39100–39105`, NIP-29 `39000`/`39002`,
  NIP-43 `13534`); commands are person-signed; `50100` drafts and `50101`
  gold are agent-signed. The `Org` class in `lib/emit.mjs` plays the relay:
  it keeps the projections a relay keeps and writes the `39xxx` an accepted
  command produces, so the seed reads as a relay would have stored it.
- **"Assigned" is offered-then-accepted.** Every held row in `data.ts` seeds
  a `50006 io_offer` from the parent's holder and the `50007 io_accept` that
  said yes. Agent-suggested rows add the `50100 t=ticket` and its `39104
  accepted`.
- **Money and join are dropped** (Prototype map § 4). The list of what was
  dropped is at the top of `lib/constants.mjs`.
- **The trail is derived, not seeded**; `read` and `proofs` on direction
  lines are agent output the harness must produce; `Health.pct` becomes a
  band. Nothing in the seed is a draft the agent is later expected to make —
  the sequence outcome deltas seed the gate closing and *no* next-wave
  drafts, because those are the gold.
- **Dates**: base clock `2026-03-01T09:00Z` plus row order, one minute
  apart; `when`/`approved`/`due` dates resolve to 09:00Z of the day (period
  start for decisions, period end for deadlines); the fixture's present is
  `manifest.now` = `2026-05-31T09:00Z`, and sequence deltas start the next
  day.

## Sequences

| Name            | Org    | Brief                                      | Gate                        | Outcome A / B                                                      |
| --------------- | ------ | ------------------------------------------ | --------------------------- | ------------------------------------------------------------------ |
| `weekday-hall`  | River  | licence, insurance, rota, opening night    | the weekday licence         | granted until 22:00 → rota + opening / refused → a daytime re-application gates the rest |
| `hall-electrics`| River  | certify, rewire, council inspection        | the electrician's report    | kitchen circuit only / whole hall fails → a structural check gates the rewire |
| `iberia-pilot`  | Energy | site, landowner, inverters, install, commission | the site survey        | school roof, municipal owner → landowner agreement via the council / no roof passes → a ground-mount site search replaces the gate |
| `andalusia`     | Energy | translation (live), Junta reading, annex, merge | the Junta's reading    | two paragraphs / a chapter plus a §3 change that belongs elsewhere |

Each `plan[]` piece is a phrase of the brief, `after` names earlier pieces,
exactly one piece is the `gate`, and a piece is `held` iff a predecessor is
neither live nor done. `draft_now` is what the agent drafts before the gate;
`gate.json` seeds those drafts accepted and the gate held; each outcome
closes the gate with an `io_done` citing the holder's message and names the
`next_wave` (pieces, `after` item ids, `must_mention` words from what was
said), what stays `held`, what is `not_redrafted`, and what belongs
`elsewhere`.

## Who is needed

`lib/constants.mjs` tunes the `39105` profiles so each org has one skill met
by exactly one member, one by two (one of them at `open_limit`), and one by
nobody. The generator checks that against the seed it just built and writes
the answer: the free candidate, or `unfilled` naming the gap. Rowan is the one
Energy member at `open_limit` in the seed (Prototype map § 3); Diego and Pedro
sit one below theirs so the sequences can hand each of them one more piece.

## Adding or changing

- A new org, room, profile, or date rule is a constant in `lib/constants.mjs`.
- A new locale is a table under `lib/locales/`; the generator fails on any
  string the table misses or no longer uses.
- A new sequence is an entry in `lib/sequences.mjs`; the Rust tests check its
  plan against the brief and its snapshots against the seed.
- Then `just org-fixtures`, `cargo test -p buzz-org-agent`, and commit the
  regenerated JSON with the change that caused it.
