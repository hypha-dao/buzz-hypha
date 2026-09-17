---
title: 'The Intelligent Organization — Development Plan'
date: 2026-09-16
status: current
tags: [plan, intelligent-org, development, buzz]
parent: docs/intelligent-org/README.md
---

# The Intelligent Organization — Development Plan

One plan across the four places the code goes — relay, CLI and SDK,
desktop, org agent — sliced into pull requests with their dependencies,
tests, and gates. It consolidates the four build orders that already agree
with each other (Design § Build order, Phase 0 § First things, Org agent
§ 20, AI evaluation § Order of work) and adds what they leave out: the
slice boundaries, what each PR must prove, and what blocks what.

It does not restate the design. Every slice names the sections that are
its spec. Where a slice depends on a code fact, it names the item in the
[Codebase verification](../architecture/intelligent-org-codebase-verification.md)
(V-n) and the decision in the [Readiness review](./intelligent-org-readiness.md)
(D-n) it needs.

**Scope.** Waves 1–4 are [Phase 0](./intelligent-org-phase-0.md) — Design
build steps 1–3, the dogfood community, the four moves. Waves 5–8 are
Design steps 4–8 as epics, sliced when their wave opens.

---

## Conventions every slice obeys

From [AGENTS.md](../../../AGENTS.md); listed because each has bitten a
past PR:

- **Kinds first.** New integers land in `crates/buzz-core/src/kind.rs`
  (constants, `ALL_KINDS`, the predicate arms) before any handler, and are
  mirrored into `desktop/src/shared/constants/kinds.ts` and
  `mobile/lib/shared/relay/nostr_models.dart` in the same PR.
- **Schema in three places.** A migration in `migrations/`, the
  desired-state `schema/schema.sql`, and — for anything `pgschema` cannot
  express (seed rows, storage parameters) —
  `scripts/reconcile-schema-after-pgschema.sql` with a live assertion.
- **Events over endpoints.** No new HTTP route. Every write is a command
  kind through the existing `EVENT` / `POST /events` path; every read is a
  REQ filter.
- **Every caught failure leaves a durable record or propagates** (rule 1);
  **fence async results by generation** (rule 2); **regression tests bind
  the production seam** (rule 3); **bound every loop and resource**
  (rule 4); **one user action, one atomic persist** (rule 5).
- No `unsafe`; no new `unwrap()`/`expect()` in production paths; doc
  comments on public API; `just ci` green; `git commit -s`.
- Desktop text in rem tokens; every new component audited for assistive
  semantics (rule 7); every input modality tested (rule 8).

---

## Workstreams

| Prefix | Where                                                   | Owner skill      |
| ------ | ------------------------------------------------------- | ---------------- |
| **R**  | relay: `buzz-core`, `buzz-db`, `buzz-relay`, migrations | Rust, Postgres   |
| **C**  | `buzz-sdk` builders, `buzz-cli` `org`, `buzz-test-client` E2E | Rust       |
| **D**  | desktop `features/org/`, profile, agents door, Playwright | React, Tauri   |
| **A**  | `crates/buzz-org-agent`                                 | Rust, LLM        |
| **E**  | evaluation fixtures and cases (`buzz-org-agent/tests/eval/`) | product + Rust |
| **O**  | provisioning, staging deploy, dogfood operations        | ops              |

---

## Slices

Each slice is one reviewable PR (or two if it exceeds ~800 changed lines
of non-test code). **Spec** names the sections that define it. **Proves**
is the test that must exist and fail if the slice is reverted. **Needs**
is what must merge first.

### Wave 1 — the spine (relay, CLI, no UI, no model)

| #    | Slice                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Spec                                                          | Proves                                                                                                                                                                                                                                                                                                                                                | Needs                     |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| R-1  | **Kinds and payload types.** Constants `39100–39105`, `50001–50021`, `50100–50103`; `ALL_KINDS`; `is_command_kind` and `is_relay_only_kind` in `kind.rs`, `is_global_only_kind` in `handlers/ingest.rs` (V3 note); `crates/buzz-core/src/intelligent_org.rs` serde types for every content schema in Protocol §4 (WorkItem, Proposal, Shapers, Direction, Draft payloads, Health, Progress, AgentNote, Profile) with `schemars` derives; mirror into `kinds.ts` and `nostr_models.dart`. Applies D4, D5, and D11 (single-letter filter tags). No behaviour. | Protocol §3, §4; V1                                            | `kind.rs` unit tests: every new kind in `ALL_KINDS`; predicate arms; `39100–39105` parameterized-replaceable const asserts. Serde round-trip per type. A new org-range parity script over `kind.rs`, `kinds.ts`, `nostr_models.dart` (V1 found none exists), which also greps `docs/intelligent-org/`, `desktop/src/features/org/`, `crates/buzz-cli`, and the E-1 generator for the retired long tag names (`#needs`, `#item`, `#parent`, `#status`, `#skill`, `["kind"`, `["note"`) and fails on any hit (risk 1).                                                                                                  | — (D4, D5, D11 decided)   |
| R-2  | **Migration and projections.** `migrations/0045_intelligent_org.sql` (or the next free number): `io_shapers`, `io_direction`, `io_work_items`, `io_proposals`, `io_votes`, `io_drafts`, `io_progress`, `io_health`, `io_health_ratings`, `io_profiles` (`skills text[]` GIN), `io_ledger`, `io_hosted_agents`; all with `community_id`; `schema.sql`; reconcile script if needed. `buzz-db/src/store/intelligent_org.rs`: typed reads and writes, transaction-scoped. **Generic tag pushdown** (V2): `EventQuery.custom_tags` rendered as JSONB containment per tag on the existing GIN index, filled from `filter_to_query_params` for every single-letter tag not already pushed — no new index. | Protocol §6.2, §6.5; V2                                        | Migration applies on a fresh DB and on the desired-state path; store round-trips per table; a REQ `{kinds:[50100], "#n":[me]}` over a page of 600 drafts where only the last needs `me` returns it (bind to the production query path, not a helper).                                                                                                  | R-1                       |
| R-3  | **Executor spine and Shapers.** `handlers/intelligent_org/{mod,apply,authorize,state}.rs`; `apply(tx, ledger_row, projection_change, state_event)` as the only write path; `state.rs` builds and relay-signs `39100–39105`; rejection of client `EVENT` for `39100–39105`; bootstrap `io_shapers_propose op=add` by the owner (private `#shapers` created, `io_hosted_agents` read, `39103` written with `agent`/`agent_hosted`); `io_shaper_accept`, `io_shaper_step_down`; `#shapers` roster updated in the same transaction. All writes go on the `tx` that `persist_command_event` returns — not on the pool as today's DM/workflow handlers do (V3). | Protocol §3.2, §4.5, §6.1, §6.4; Design § Shapers; V3, V4      | E2E: bootstrap emits `39103` with the hosted pubkey and `agent_hosted=true`; a client `EVENT` of `39103` is rejected `restricted`; last Shaper cannot step down; a non-owner cannot bootstrap; the room roster equals `shapers ∪ {agent}` after each change; a handler failing after the projection write leaves no event, no ledger row, and no `39103`. | R-2                       |
| R-4  | **Proposals and votes** — ships as two PRs: **R-4a** Shapers ops (`50001` add / remove / rules / agent, `50003` on them, the `shapers` execution rows) and **R-4b** `50002`, `50004`-opening, `50015`, `50003` on them, `direction`/`dri` execution. `50001` (add / remove / rules / agent), `50002`, `50003`, `50015`; `needed` resolved at opening from `39103.rules`, `eligible` minus subject, frozen; pass / reject / `all` for rules and agent; execution table §5.3 for `direction`, `dri`, and every `shapers` op (the `project` execution lands in R-5, `agent` membership move in R-8); `39102` emission; D1's optional opener-vote tag. Money and join kinds rejected with the fixed reasons. | Protocol §4.4, §5.2, §5.3; Features 1a; D1                     | E2E for every line of Phase 0 § Keeping the agent honest — relay that concerns proposals: non-eligible vote rejected; subject cannot vote; `needed` fixed at opening while a Shaper is added mid-vote; rules and agent need `all`; agent `p` must be a non-Shaper member; `50013`/`50016` rejected; `stale base` on direction; opener-vote tag records `vote_cast` atomically. | R-3                       |
| R-5  | **Work tree** — ships as two PRs: **R-5a** `project` execution + `50005`–`50008` (create / offer / accept / decline) and **R-5b** `50009`–`50011`, `50018` (done / release / set-due / reopen) with the invariants that need them. `project` execution (root in `open` or `offered`; `approved_at`); `50005`–`50011`, `50018`; the seven invariants of §5.1; `39101` emission with the tag set of §4.2; children counters; `objective_ref` validation against the live `39100` line; `after` on `50005` validated as live-or-done siblings under the same parent, stored on `39101`, never a lock.                                                                                                                                                                                                              | Protocol §4.2, §5.1; Design § Work objects                     | E2E per invariant: only holder creates children; only `offered_to` accepts/declines; done refused with open children; release returns children; money fields rejected; one event + one `39101` + one ledger row per command; `io_reopen` within 7 days only; `after` naming a non-sibling rejected, a done sibling accepted, and an `io_accept` on an item whose `after` is still open succeeds.                                                                                          | R-4                       |
| R-6  | **Scheduler.** `io_scheduler`: offers renotified at half-window (D6) and returned after; `in_review` at the last fifth (floor two days); close on `due_at` with `orphaned_by_close`; draft `expiration` → `39104 expired`; proposal `expires_at` → `expired` with notice to `opened_by`; seat lapse. Idempotent; one transaction per transition; every transition is a DB claim as `admin_action_worker` and `claim_scheduled_workflow_fire` do, never a process-local decision (V9, multi-pod); `actor="relay"` ledger rows. | Protocol §5.1 rule 5, §6.3; V9; D6                             | E2E with a clock hook (or short windows in test config): each transition once and only once, including with two sweeps running concurrently; a root with a moved `due_at` is not closed; open children of a rule-closed root are `orphaned_by_close`.                                                                                                                                                 | R-5                       |
| R-7  | **Drafts, reads, and settlement.** `50100` ingest (shape, `needs`, receipts resolve community-wide, one open draft per `gap`, holder receipt and `skill` slug rules, `open_limit`) → `39104 open`; `50101` (item exists, rows resolve); `50103` (author is `39103.agent`, no receipt check, ledger row); draft settlement on any command carrying `["e", id, "", "draft"]` (`accepted` vs `amended`); `50012`; `50017`.                                                       | Protocol §3.2 settlement, §3.3, §4.3, §4.6–4.7c, §5.4, §5.4a, §6.1 | E2E: unresolved receipt rejected; second open draft per `gap` rejected; `50103` from a non-agent rejected; a command with a draft tag from the wrong `needs` party rejected and the draft untouched; equal payload → `accepted`, different → `amended`; `50101` from a non-agent rejected.                                                                 | R-5                       |
| R-8  | **The agent everywhere.** `39103.agent` added on channel create (`9007`) and on every relay-created room, and to the participant set on `41010`; backfill at bootstrap; the `shapers/agent` execution moves every membership row; **DM identity excludes the agent** at the relay seams V5 names (`41011` member-set hashing, DM `39000`/`39002` `p` tags, the `41010` system message's `participants`) and the 2–9 participant cap counts humans only; the `participant_hash` itself is untouched by membership and needs no change; D2's agent-DM shape accepted by `handle_dm_open`. Relay-only — no desktop or mobile change.                     | Protocol §6.8; Design § Where it runs — Everywhere; V5; D2      | E2E in `e2e_nostr_interop.rs` and `e2e_relay.rs`: two members opening "our DM" find one DM; its `39000`/`39002` list two `p` while `channel_members` holds three; the `41010` system message names two participants; a nine-human DM opens with the agent as a tenth row; a `41011` on that DM then a second `41010` by the humans finds the same channel; a channel created by a member has the agent in `39002`; after `shapers/agent` the old key is in no room and the new key is in all; a `[member]`-only `41010` opens the agent DM. | R-4                       |
| R-9  | **Project home** — **Phase 0 ships the room half only (R-9a)**; the repository half (R-9b: `30617`/`30621`, manifest seed, quota exemption, push-hook record) lands in wave 6 with Work sync, which is the first thing that reads it (risk 3). Full scope: on `project` pass, in the same transaction: room (`create_channel` + members + `emit_group_discovery_events`), relay-signed `30617` stored through `replace_parameterized_event` with `buzz-channel`, `maintainers`, and `["buzz-protect","refs/heads/main","push:admin","no-force-push","no-delete"]`, `30621`, `home` on `39101`; after commit, `handle_git_repo_announcement_inner` seeds the manifest pointer (an S3 write — on failure a durable retry row, rule 1). The relay pubkey is exempt from `BUZZ_GIT_MAX_REPOS_PER_PUBKEY` (V8). Roster sync on accept / release / `dri` (root holder → admin + `maintainers`, child holder → member, attested agents → bot); archive-on-done scheduled. **Push-hook record** for D12: on an allowed fast-forward to the default branch, store `(repo, ref, old, new)` for the `merged_into` fill. Sovereign relays without object storage skip repo and project. | Protocol §6.7; Design § Work objects; V8, V15; D12              | E2E: a passed project leaves room + `39000–39003` + `30617` + `30621` + `home`, none after a rolled-back tx; the 101st project still gets a repo; after root `io_accept` the holder is admin and the only maintainer; after child accept, member; after release, member and empty `maintainers`; a push to `main` by a child holder is refused, to `refs/heads/io/*` allowed; a pointer-seed failure leaves a retry row and the next tick seeds it. | R-5, R-8 (R-9b: wave 6)   |
| R-10 | **Receipt read.** `{ids:[…]}` REQ from a community member served for an event in a room they are not in iff the id is cited as a receipt on a stored community-global `io` event; one event, never a window. **May slip to wave 5 (HEAR) without blocking Phase 0** — no Phase 0 draft cites a message.                                                                                                                                                                       | Protocol §6.8 Receipt read; V6                                 | E2E: a non-member's `{ids}` REQ for a cited message returns it; for an uncited message in the same room returns nothing; a `#h` REQ for that room still returns nothing.                                                                                                                                                                              | R-7                       |
| R-11 | **Org profile and membership stream.** `50021` → `39105` + `io_profiles`; the limits of §4.7a; self-only; `profile` draft rules of §5.4a; inactive on NIP-43 removal. `claim_invite` already publishes `8000`/`13535` (V7), so `MemberJoined` is observable; the relay's `member_joined` ledger row is written by `claim_invite` itself, not derived from the best-effort publish.                                                                                                | Protocol §4.7a, §5.4a, §6.6; V7                                | E2E: `50021` for another pubkey rejected; a `profile` draft not addressed to its subject rejected; limits enforced; `{kinds:[39105], "#k":[slug]}` answers "who can do X"; a claim writes the ledger row even when the `8000` publish is made to fail.                                                                                                      | R-7                       |
| R-12 | **Invites.** `mint_invite` accepts a pubkey in `39103.shapers` (read from `io_shapers` at request time); ledger `member_joined` with `minted_by`; the transparency notice on `/invite/<code>` per D7.                                                                                                                                                                                                                                                                       | Protocol §6.6; Features 6a; V7; D7                             | E2E: a Shaper who is not owner/admin mints; a plain member cannot; the landing page carries the notice for an org community.                                                                                                                                                                                                                          | R-3                       |
| R-13 | **Inbox sources.** `query_needs_action` extends its `kind IN (…)` list with `39101` offered-to-me, `50100` needs-me (or `shaper`), open `39102` where I am eligible or the offered seat. The query is joined on `event_mentions`, so each of those events must carry a `p` tag for the party (V10; `39101` and `39102` already do, `50100` gains `["p", <pubkey>, "", "needs"]`); `needs = shaper` drafts join the reader against `io_shapers` instead.                                                                                                                                                                                                                                                                                                                | Protocol §6.5 last paragraph; V10                              | Store test on the feed query with seeded `io_*` rows; a `50100` whose party is in `needs` but not in a `p` tag does not appear; the desktop Inbox spec in D-2 renders them.                                                                                                                                                                                                                                                     | R-7                       |
| C-1  | **SDK builders.** `build_io_*` for every command and for `50100`/`50101`/`50102`/`50103`, typed over the R-1 payload types; tag layout exactly Protocol §4.8.                                                                                                                                                                                                                                                                                                              | Protocol §4.8; Design § Surfaces CLI                           | Builder tests: tag sets per command; a builder cannot produce a `39100–39105`.                                                                                                                                                                                                                                                                        | R-1                       |
| C-2  | **`buzz org` CLI.** The surface in § CLI surface below, JSON output, exit codes, `--format compact`; `org bootstrap` for the dogfood seed.                                                                                                                                                                                                                                                                                                                                 | Design § Surfaces; AGENTS.md § Agent CLI                       | `crates/buzz-cli/TESTING.md` runbook entries; unit tests on argument → event.                                                                                                                                                                                                                                                                         | C-1, R-3                  |
| C-3  | **E2E suite.** `crates/buzz-test-client/tests/e2e_intelligent_org.rs`: every "Proves" line of R-3…R-13 driven through C-1/C-2, plus the Protocol §9 worked example end to end.                                                                                                                                                                                                                                                                                              | Phase 0 § Keeping the agent honest — relay; Protocol §9         | It is the proof for the wave.                                                                                                                                                                                                                                                                                                                         | grows with each R slice   |

### Wave 2 — the doors and the ruler (desktop, agent skeleton, fixtures)

| #    | Slice                                                                                                                                                                                                                                                                                                                                             | Spec                                                                         | Proves                                                                                                                                                                                                                                | Needs             |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| D-0  | **Feature gate and skeleton.** `org` gate (V14 mechanism), routes `/org`, `/org/work`, `/org/work/$itemId`, `/org/my-work`; sidebar group; `features/org/hooks/` — one live REQ hook per door (Protocol §6.5) with backfill/live overlap; `features/org/commands.ts` — build, sign (`sign_event`), publish each command, with the draft `e` tag when settling a card. | Design § Surfaces; Phase 0 § The app; Prototype map § Routes                 | Playwright (mock bridge): gate off hides the group; each route renders its empty state _Nothing needs you._ / _Not set yet_; a command hook produces the right kind and tags (assert on the mock bridge's captured event).             | R-1, R-4          |
| D-1  | **Overview.** Four direction cards from `39100` with version and confirmer; empty slots; direction form → `io_direction_propose`; Shapers card from `39103` — members, rules, agent host — with **Add a Shaper**, **Step down**, **Change the rules**; who holds what from root `39101`s. Tally card (Shapers only) from `{kinds:[50103], "#t":["tally"]}`. | Phase 0 § Overview; Prototype map § Overview, § Direction                     | Playwright: renders from seeded state; the form emits `50002` with `base` = current version; Add a Shaper emits `50001 op=add`.                                                                                                        | D-0               |
| D-2  | **My Work and the card set.** Three columns; card components — draft, offer, decision, done, review — with kickers (_AI is asking you_, _AI is suggesting for (name)_, _Drafted by the agent_), receipts, _n of needed_; decline reason chips → `io_draft_decide`; Inbox `needs_action` renders the same cards. Accessibility audit per rule 7.                  | Phase 0 § My Work, § Cards by move; Design § Surfaces cards; Prototype map § Cards | Playwright: each card type from a seeded `50100`/`39101`/`39102`; Agree carries the draft tag; Decline emits `50012` with the chosen reason; keyboard path for every tap (rule 8).                                                     | D-0               |
| D-3  | **Work and the item page.** Tree from `39101` (roots + one level; deeper on the item page); item page — brief, holder, dates, breadcrumb, children with state chips, trail from `{kinds:[50001–50021], "#i":[id]}`, **Open room** from `home.channel`, health card from the latest `50101` with rows on hover. Mark done / Release / Set due.                | Phase 0 § Work, § Project / ticket page; Prototype map § Work, § Ticket      | Playwright: tree renders depth 3 from fixtures; item page shows trail newest first; Mark done emits `50009`; health sentences expose rows.                                                                                             | D-0               |
| D-4  | **About & skills.** Section on the existing profile view: editable for me → `io_profile_set` (whole profile); read-only on others; skills as chips; `open_limit`. A `profile` card in D-2 settles through the same command with the draft tag.                                                                                                                | Phase 0 § The principle (My Profile row); Protocol §4.7a, §7                 | Playwright: Save emits `50021` with the full profile; another member's profile has no form.                                                                                                                                            | D-0               |
| D-5  | **Agents door — Hypha defaults.** Stop seeding `BUILT_IN_PERSONAS` (V13); the door starts empty; the **Work sync** template entry is present but disabled until wave 6; the org agent never appears. DMs list the org agent first.                                                                                                                              | Design § Where it runs — Members' own agents stay; Current State             | Tauri Rust test: no built-in persona in a fresh store; Playwright: the door's empty state; DM list order.                                                                                                                              | —                 |
| E-1  | **Fixtures from the prototype.** `tests/eval/fixtures/orgs/{river,energy,cold}/seed.json` as event lists (Protocol kinds) generated from `prototypes/org-preview/src/lib/data.ts` by a checked-in script; `en` plus `pt`/`es`. Money fields dropped; "assigned" holders re-expressed as accepted offers; the mapping table in the Prototype map is the spec. Plus the sequence and who-is-needed fixtures Eval § Test data names: multi-step briefs with `why_gold` order, gate-outcome snapshots (before / outcome A / outcome B), and profiles tuned so `requires` is met by one, two, or nobody.        | AI evaluation § Test data; Prototype map § Fixtures                          | The loader (A-1) applies each seed through `OrgState::apply` without error; counts match the map's tables.                                                                                                                             | R-1               |
| A-0  | **`buzz-agent` deltas.** `llm` public (or `pub mod llm`), `temperature: Option<f32>`, `tool_choice: Option<String>` on the request path; no behaviour change for `buzz-agent` itself.                                                                                                                                                                          | Org agent § 3.1, § 21; V12                                                   | Existing `buzz-agent` tests unchanged and green; a new test that `tool_choice` reaches the provider request.                                                                                                                           | —                 |
| A-1  | **Skeleton and ruler.** `crates/buzz-org-agent`: `OrgState` + `apply` + the § 5.2 transition table; `RelayLink` (reconnect, watermarks, REQ set — V12 confirms none exists to reuse); outbox; `ModelClient` with `BuzzAgentModel` and `Recorded`; `judge.rs` (all 15 gates, incl. `sequence`); `route.rs`; `publish.rs` with the `Permitted` kinds chokepoint and `tests/allow_list.rs`; `FakeRelay`; harness loader over E-1; `run`, `dry-run`, `replay`, `doctor` modes. Nothing drafts. | Org agent § 2, § 4, § 5, § 9, § 10, § 18, § 19; Phase 0 § The agent pipeline | Unit: one test per judge gate; transitions per row of § 5.2; allow-list fails on a `50009` outside its module. Pipeline: outbox drains after a simulated disconnect; a newer generation mid-THINK → `stale`.                              | R-1, A-0, E-1     |

### Wave 3 — the first drafts (dogfood goes live)

| #    | Slice                                                                                                                                                                                                                                                                                                         | Spec                                                       | Proves                                                                                                                                                                                                                                                                                       | Needs                          |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| O-1  | **Provisioning script.** `scripts/org-agent-provision.sh <community>`: mint key, `kind:0` "Org agent", NIP-43 add, `io_hosted_agents` row, launch `buzz-org-agent run` with env; secrets from the operator's store.                                                                                          | Org agent § 15.2; Phase 0 § Deploy                         | Runs against a local relay in CI (`just test` lane) and leaves the row and the member.                                                                                                                                                                                                       | R-3, A-1                       |
| O-2  | **Staging relay deploy** with the R-wave migration; the desktop build with the `org` gate on for the team.                                                                                                                                                                                                    | Phase 0 § Deploy                                           | Smoke: bootstrap on the staging community via `buzz org bootstrap`; `39103` visible in the desktop Overview.                                                                                                                                                                                 | wave 1, D-1                    |
| O-3  | **Dogfood bootstrap.** D9's community and second Shaper: owner bootstraps, proposes the second seat, the seat is accepted; members invited by a Shaper's link; the four direction artifacts written and confirmed in the app (Phase 0 § Direction — written on day one). First real confirm = first trigger. | Phase 0 § The first org; Journeys 2.2, 2.11                | The record itself; the four `39100` v1 heads exist and were confirmed under `majority` with two Shapers.                                                                                                                                                                                     | O-2, D-1, D-2                  |
| A-2  | **J1, J1b, J13 (expiry notice), J12 (greeting, per D3).** Prompt `1-direction-to-projects.md` v1 with pinned model (D10); context recipe; candidate list; first step first (a validation before what depends on it); `suggested_dri: null` with the missing capability named when nobody fits; shadow on the dogfood community first, cards when the offline suite is at target.                                                                     | Org agent § 1 rows J1/J1b/J12/J13, § 8, § 11.2; Eval § 1     | Eval suite `direction-to-projects` green at target on recorded responses (positives, negatives, adversarial as listed); pipeline test from `DirectionConfirmed` to a captured `50100`; notice dedupe.                                                                                          | A-1, O-3 (for shadow)          |
| E-2  | **Case sets.** Gold cases for all four suites over River, Energy, cold — written by one person, reviewed by another, each with `why_gold` domain reasoning; the sequence cases (gate first, next wave, outcome changes the plan, no invented order) and who-is-needed cases (`requires`, `unfilled`, skill over availability) in the first cut, not a later one; the § 5 case sets (J1b, J3d, J6, J7, J8/J8b, J9, J10, J11) landing before each job leaves shadow; the vacuous-title list; the model judge prompt v1 with the seven-question rubric; κ check procedure documented in the suite README.                                                                                                             | AI evaluation § The bar is expertise, § 1–5, § Three judges | Each case loads and its gold parses; negatives are ≥ half of each suite; every job in Org agent § 1 with model output has ≥ 1 case set.                                                                                                                                                                                                                     | E-1                            |

### Wave 4 — the remaining moves and the Friday ritual

| #    | Slice                                                                                                                                                                                                                                                                                | Spec                                                     | Proves                                                                                                                                                                                                                             | Needs         |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- |
| A-3  | **J4 + J5.** `health_formula.rs` with `health-weights.json@1`; prose with rows; Friday tick; tally `50103`; blind bands via `io_health_rate` from the Overview tally card (D-1 already renders it; the rate buttons land here).                                                        | Org agent § 11.4, § 6.2; Eval § 4; Phase 0 § Tally        | Monotonicity per factor; five runs same band; grounding gate; tally payload shape (Protocol §4.7c).                                                                                                                                 | A-2           |
| A-4  | **J2, J3a/b/c; Monday scan; catch-up; `--dry-run` on the live org.** Coverage list first as an **ordered plan** — `order`, `after`, `held`, `gate`; draft what can start now, hold the rest; re-run on "child done unblocks a held piece" with the outcome in context; `requires` before the candidate list, `unfilled` when nobody fits; one level down, ≤ 7; done card; review brief from the ledger view; redraw as operations.                                                                                                     | Org agent § 8.6, § 6.2; Eval § 2–3                        | Suites `projects-to-tickets` and `completion-to-direction` at target, including the sequence-fit and who-is-needed rows; pipeline test: a gate ticket's `ItemDone` re-runs J2 on the parent exactly once and the next wave carries `after`; catch-up runs a missed Monday once.                                                                                                                          | A-3           |
| D-6  | **Playwright coverage of the loop** with the mock bridge: a seeded direction confirm → a project card → agree → proposal card with _1 of 2_ → pass → root on Work → offer → accept → child card → done card. Screenshots posted per AGENTS.md.                                          | Phase 0 § Keeping the agent honest — desktop             | The spec exists and is registered in `playwright.config.ts`.                                                                                                                                                                       | D-1…D-4       |
| O-4  | **Friday ritual and the record.** The fifteen-minute routine running; after four green Fridays, export the community's `io_*` rows as the fourth fixture (`tests/eval/fixtures/orgs/dogfood/`).                                                                                       | Phase 0 § The Friday ritual, § Exit criteria; Eval § Test data | Exit criteria met for four consecutive Fridays.                                                                                                                                                                                    | A-4           |

**Phase 0 ends at O-4.** The exit criteria are Phase 0's, unchanged.

### Waves 5–8 — epics, sliced when their wave opens

| Wave | Epic (Design step)                              | Slices to open then                                                                                                                                                                                                                                                                     | Needs                          |
| ---- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| 5    | **Decisions** (step 4)                          | D-7 Decisions door with filters Work / Direction / Shapers and _n of needed_; D-8 My Profile sections (current, earlier, recent decisions); Overview glance numbers and timeline (Readiness F12 — define the reads first).                                                                | Phase 0 exit                   |
| 5    | **HEAR** (step 5)                               | R-10 if slipped; R-14 transcript tag in huddle STT (V11); A-5 batcher, pre-filter, classify, THINK-R menu, fencing; J7 in `#shapers` and project rooms, then J6, J9, J10, J11, remaining J13; **J8 last** behind `IO_DONE_FROM_TALK_ENABLED` — R-15 the §5.5 executor check.                | wave 4                         |
| 6    | **Work sync** (step 6)                          | R-9b the repository half of the project home; R-16 `50102` ingest per D12 (`30618` ref check, `head_verified`, push-hook `merged_into`); `last_progress` on `39101`; D-9 work log + _last moved_ + **Open in editor**; D-10 heartbeat-only respond mode in agent creation (V13) and the Work sync persona; C-4 `buzz org progress note`; A-6 J3a on `ready`/merge, J14, `stalled`. | wave 5 HEAR not required       |
| 7    | **Hosted at scale + money** (steps 3-ops, 7)    | A-7 `supervise`, registry, key store, per-community budgets, provisioning in the binary; then the treasury: chain and contract chosen, R-17 bridge worker and key, `50013`/`50014` enabled, `rules.money`, D-11 Money filter and _Paid to you_, A-8 J15.                                    | wave 6                         |
| 8    | **Later** (step 8)                              | Join proposals (§5.7), pay-agreed-in-chat (J16), document text extraction, post-commit hook for Work sync, directory, mobile cards.                                                                                                                                                       | as needed                      |

---

## Order and parallelism

```
wave 1   R-1 ─► R-2 ─► R-3 ─► R-4a ─► R-4b ─► R-5a ─► R-5b ─► R-6
                              │                  ├────► R-7 ─► R-10? ─► R-11 ─► R-13
                              │                  └────► R-8 ─► R-9a   (R-9b: wave 6)
                              └────► R-12
         C-1 (after R-1) ─► C-2 (after R-3) ─► C-3 (grows with every R)
wave 2   D-0 (after R-4) ─► D-1, D-2, D-3, D-4 in parallel;  D-5 any time
         E-1 (after R-1) ─► A-0 ─► A-1
wave 3   O-1 ─► O-2 ─► O-3 ─► A-2 (shadow → cards);  E-2 alongside
wave 4   A-3 ─► A-4;  D-6;  O-4
```

Three people can run wave 1 (R), wave 2 (D), and A-0/A-1/E-1 concurrently
from the day R-1 merges; nothing in D or A waits on R past R-4 except the
live relay for shadow mode. One person alone follows the arrows.

**Serialise on R-1.** Every other slice imports its kinds and types; two
PRs that both add kinds will conflict in `kind.rs`, `kinds.ts`, and
`nostr_models.dart`. Merge R-1 first and alone.

---

## Definition of done, per slice

- The **Proves** test exists, binds the production seam, and fails when the
  slice's guard is removed (TESTING.md § Review-Proven Test Standards).
- `just ci` green; `just test` green for any R slice.
- Protocol, Design, or Phase 0 edited in the same PR if the slice changed a
  rule or a name — the documents are the contract, not a description.
- For D slices: screenshots posted with `scripts/post-screenshots.sh`;
  assistive semantics audited; keyboard path tested.
- For A slices touching `src/prompts/`, `schemas.rs`, `context.rs`,
  `judge.rs`, or `health-weights.json`: the eval suites re-run and the
  metric diff is in the PR description.
- No slice widens a move from shadow to cards below its offline target
  (AI evaluation § Rollout gates).

---

## CLI surface

`buzz org …` in `buzz-cli`. Every verb maps to one command kind or one REQ
filter; the org agent and the E2E suite use the same builders.

| Group        | Verbs                                                                                                                                     | Kind / filter                                                         |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `bootstrap`  | `bootstrap` (owner self-add)                                                                                                              | `50001 op=add`                                                        |
| `shapers`    | `list`, `add <p>`, `remove <p>`, `rules <json>`, `agent [<p>]`, `accept <proposal>`, `step-down`                                          | `39103`; `50001`; `50019`; `50020`                                    |
| `direction`  | `show [slug]`, `history <slug>`, `propose <slug> --base <n> --body … [--lines …] [--draft <id>]`                                          | `39100`; `50002`                                                      |
| `proposals`  | `list [--kind] [--status]`, `show <id>`, `vote <id> agree\|decline [--reason]`, `propose-project`, `propose-dri <item> <p>`               | `39102`; `50003`; `50004`; `50015`                                    |
| `work`       | `tree [--root]`, `show <item>`, `create <parent> …`, `offer <item> <p>`, `accept`, `decline`, `done`, `release`, `set-due`, `reopen`, `my-work` | `39101`; `50005`–`50011`, `50018`; `{kinds:[39101], "#p":[me]}`   |
| `drafts`     | `list [--needs me\|shaper]`, `show <id>`, `decide <id> accept\|decline [--reason]`, `publish <json>` (person-signed `50100`)              | `50100` + `39104`; `50012`                                            |
| `health`     | `show <item>`, `rate <item> <week> <band>`                                                                                                | `50101`; `50017`                                                      |
| `profile`    | `show [<p>]`, `set --about … --skill … [--limit n]`, `who-can <skill>`                                                                    | `39105`; `50021`; `{kinds:[39105], "#k":[…]}`                     |
| `progress`   | `note <item> …` (wave 6)                                                                                                                  | `50102`                                                               |
| `ledger`     | `list [--item] [--since]`, `note <json>` (agent notes)                                                                                    | commands by `#i` (item); `50103`                                          |
| `tally`      | `tally`                                                                                                                                   | `{kinds:[50103], "#t":["tally"]}`                                  |

Not a CLI verb, ever: anything that emits `39100–39105` (relay-only), or
`50014` (bridge-only).

---

## Test matrix

Where each test layer of Phase 0 § Keeping the agent honest lives:

| Layer                     | Location                                                        | Slices                     |
| ------------------------- | --------------------------------------------------------------- | -------------------------- |
| Kind registry             | `crates/buzz-core/src/kind.rs` tests                            | R-1                        |
| Store / migration         | `crates/buzz-db` tests (Postgres)                               | R-2, R-13                  |
| Relay invariants          | `crates/buzz-test-client/tests/e2e_intelligent_org.rs`          | R-3…R-12, C-3              |
| DM identity               | `e2e_nostr_interop.rs`, `e2e_relay.rs`                          | R-8                        |
| Git home                  | `e2e_intelligent_org.rs` + the git policy tests in `api/git/`   | R-9                        |
| Agent unit / pipeline     | `crates/buzz-org-agent/tests/{judge,pipeline,allow_list.rs}`    | A-1…A-4                    |
| Eval                      | `crates/buzz-org-agent/tests/eval/`                             | A-2…A-4, E-1, E-2          |
| Agent ↔ relay             | `e2e_intelligent_org.rs` (agent binary against a live relay)    | A-2, R-7                   |
| Desktop                   | `desktop/tests/e2e/org-*.spec.ts` (mock bridge)                 | D-0…D-6                    |
| Tauri Rust                | `desktop/src-tauri` tests                                       | D-5                        |

---

## Risks to the plan (not to the product)

Product risks are Design § Known risks. These are delivery risks:

1. **V2 was refuted** — multi-letter tag filters are not expressible in
   `nostr::Filter` at all. The fix is D11 (single-letter tags) in the
   Protocol before R-1, plus generic tag pushdown in R-2. No index
   migration; the live matcher already handles any single letter. The risk
   that remains is a fixture or a CLI filter written with the old long
   names — the R-1 parity script and C-1 builder tests are the guard.
2. **V5 is relay-only** (DM identity lives in `participant_hash`, which
   membership does not touch). R-8 is one PR. The residual risk is a
   fourth seam nobody listed; the R-8 proof asserts on `39000`, `39002`,
   the `41010` system message, the participant cap, and a repeat `41010`
   (`40901` is defined but never emitted by the relay — V5).
3. **V15 was refuted** — there is no git store at ingest; repos hydrate
   from object storage per request. D12 moves `merged_into` to a
   relay-derived fill from the push hook. R-9's repository half is not
   needed until wave 6; if it is late, ship the room-only home
   (sovereign-relay shape) and land the repository with Work sync.
4. **Model choice** (D10, decided) drifts. Every prompt file's frontmatter
   pins it; the harness re-records on change; the tally splits by `model`
   tag.
5. **Review rounds.** AGENTS.md reports ~5 rounds per PR. Keep slices at
   or under ~800 non-test lines. R-4 and R-5 are pre-split above (R-4a/b,
   R-5a/b) rather than split when they balloon.
6. **One community, two Shapers** is the whole online sample in Phase 0.
   The metrics in Phase 0 § What we measure are read as direction, not
   statistics, until the fourth fixture exists. If a second team community
   is available on the staging relay, add it in wave 4; it doubles the
   sample for no code.
7. **Eval case authoring is the long pole of wave 3.** E-2 is four core
   suites plus eight secondary case sets over three seeds, each case
   written by one person and reviewed by another with domain reasoning.
   Start River's cases the day E-1's fixture format is stable (wave 2),
   not when A-2 is ready for them; A-2 goes to cards only when the suite
   is at target, so late cases hold the whole of wave 3 in shadow.
8. **Staging secrets and access gate wave 3.** O-1/O-2 are small in code
   but need an operator key for the agent, a provider account (D10), and
   deploy access to the staging relay. Name who holds each when O-1 opens;
   without them A-2 cannot run in shadow and the dogfood does not start.

---

## Related

- [Progress](./intelligent-org-progress.md) — which slices are merged, follow-ups, the local check recipe; every slice PR updates it
- [Readiness review](./intelligent-org-readiness.md) — the D-items and the gate
- [Codebase verification](../architecture/intelligent-org-codebase-verification.md) — the V-items
- [Prototype map](../product/intelligent-org-prototype-map.md) — what D-1…D-3 grow from and E-1 extracts
- [Phase 0](./intelligent-org-phase-0.md) — the intent this plan slices
- [Protocol](../architecture/intelligent-org-protocol.md), [Design](../architecture/intelligent-org-design.md), [Org agent](../architecture/intelligent-org-agent.md), [AI evaluation](./intelligent-org-ai-evaluation.md) — the specs each slice cites
