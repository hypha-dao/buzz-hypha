# The Intelligent Organization — Progress

Where the [development plan](./intelligent-org-development-plan.md) stands,
what each merged slice left behind, and how to run the checks on this fork.
The plan is the schedule; this file is the log. **Every slice PR updates this
file in the same PR** — the row, any follow-ups it discovered, and any change
to the local-check recipe — so a fresh agent (or person) can start the next
slice from `main` alone.

Read this after [AGENTS.md](../../../AGENTS.md) and the
[README](../README.md) reading order, and before opening a slice.

---

## Slice status

Status is one of: `merged`, `open` (PR exists), `in progress` (branch, no
PR), `blocked`, or blank (not started). Waves and slice ids are the plan's.

### Wave 1 — the spine (relay, CLI, no UI, no model)

| Slice | Status | PR | Merged as | Notes |
| ----- | ------ | -- | --------- | ----- |
| R-1a  | merged | [#2](https://github.com/hypha-dao/buzz-hypha/pull/2) | `16cd79f29` | Kinds `39100–39105`, `50001–50021`, `50100–50103`; predicates; `kinds.ts` / `nostr_models.dart` mirrors; `just org-kinds-check` parity + retired-long-tag grep. |
| R-1b  | merged | [#5](https://github.com/hypha-dao/buzz-hypha/pull/5) | `6c8501abf` | `buzz-core/src/intelligent_org.rs`: serde + schemars types for every Protocol §4 schema. ~1360 non-test lines, kept whole on purpose (see PR). |
| R-2a  | merged | [#6](https://github.com/hypha-dao/buzz-hypha/pull/6) | `88aad25b9` | Migration `0045_intelligent_org`, twelve `io_*` tables, `schema.sql`, deletion catalog, typed transaction-scoped store `buzz-db/src/store/intelligent_org.rs`, desired-state/migration parity test per table. |
| R-2b  | merged | [#7](https://github.com/hypha-dao/buzz-hypha/pull/7) | `1e935fab0` | `EventQuery.custom_tags`: every single-letter tag filter without a dedicated column is pushed as JSONB containment before `LIMIT` (V2). 600-draft `#n` proof through the production seam. |
| R-3   | merged | [#9](https://github.com/hypha-dao/buzz-hypha/pull/9) | `1f6767a49` | `handlers/intelligent_org/{mod,apply,authorize,state,shapers}.rs`; `apply` is the one write path (projection row, relay-signed `39xxx`, ledger, `#shapers` roster) on the `persist_command_event` transaction; `50001` bootstrap, `50019`, `50020`; ingest scope + executor routing for `50001–50021`; `buzz-db/src/store/relay_rooms.rs` (transaction-scoped room + roster sync). Six Postgres-lane proofs through `ingest_event` and six E2E through `POST /events`. |
| R-4a  | merged | [#13](https://github.com/hypha-dao/buzz-hypha/pull/13) | `2b33bd6af` | `handlers/intelligent_org/proposals.rs` (open with frozen `eligible`/`needed`, D1 opener vote, `50003`, tally, execution dispatch, one `apply` per settle); `shapers.rs` opens add / remove / rules / agent and executes them against the live `39103` (`shaper_offered`, `shaper_removed`, `rules_changed`, `agent_changed`); `authorize::{open_shapers, agent_candidate, rules_content, vote}`; `apply` writes `io_votes` from the `39102` projection; `store::get_proposal_opening_receipt`. Money and join kinds refused with the fixed reasons. Seven Postgres-lane proofs through `ingest_event`, five E2E through `POST /events`; the R-3 offered-seat SQL seed now goes through a real passed add. `direction`/`dri`/`project` votes that would pass are refused `invalid: execution of … not implemented yet` until R-4b/R-5a. |
| R-4b  | merged | [#23](https://github.com/hypha-dao/buzz-hypha/pull/23) | `761f01393` | `50002` / `50004`-opening / `50015` and `50003` on those kinds; `direction` / `dri` execution through `apply` (`39100` v`base+1` + `prev`, `39101` accepted + offer withdrawn). Project execution still refused until R-5a. Three Postgres-lane proofs through `ingest_event`, three E2E through `POST /events`. |
| R-5a  | merged | [#28](https://github.com/hypha-dao/buzz-hypha/pull/28) | `db1019300` | `project` execution + `50005`–`50008` (create / offer / accept / decline). Root opens `open` or `offered` to `suggested_dri`; `approved_at` set; no home (R-9a). §5.1 create/offer/accept/decline invariants: holder-only children, only `offered_to` accepts/declines, money fields refused, `after` as live-or-done siblings, `objective_ref` against the live `39100` line, children counters (parent `39101` rewritten). `50009`–`50011` / `50018` still refused. R-4b DRI proofs now open roots through a passed `project`. |
| R-5b  | merged | [#37](https://github.com/hypha-dao/buzz-hypha/pull/37) | `b0259fed0` | `50009`–`50011` / `50018` (done / release / set-due / reopen). Holder-only done; done refused with open children; release clears the holder and offers live children to the parent holder (or `open` for a root); set-due is a Shaper on a root / the parent holder on a child; `io_reopen` within 7 days of `io_work_items.done_at`. Parent `39101` rewritten so `children` stays on the live event. One command + one ledger row; live `39101` count does not grow (replace, not add). Two Postgres-lane proofs through `ingest_event` and two E2E through `POST /events`: `only_the_holder_marks_done_and_open_children_are_refused`, `release_returns_children_and_set_due_follows_authority`. |
| R-6   |        |    |           | |
| R-7   |        |    |           | |
| R-8   | open | [#29](https://github.com/hypha-dao/buzz-hypha/pull/29) |           | `39103.agent` is a real `channel_members` row on 9007 / `create_channel` / `create_channel_with_id` / `create_room` / `41010`, backfilled in `shapers::bootstrap` in the same transaction (`agent_membership_synced why=bootstrap`), and moved by `shapers/agent` (`why=agent_changed`). DM identity excludes the agent at the V5 seams (41011 hash, DM 39000/39002 `p`, 41010 `participants`); the 2–9 cap counts humans; `participant_hash` is untouched; `[member]`-only 41010 is the agent DM. `AGENT_ROOM_ROLE` stays `member`. Protocol §6.8 matches: `channel_members` is membership truth; DM `39000`/`39002` are identity; only a channel `39002` lists the agent. Migration `0046` teaches the 0032 roster fence the V5 exception. Four Postgres-lane proofs, three `e2e_nostr_interop` / `e2e_relay` identity proofs, one `e2e_intelligent_org` backfill+move proof. |
| R-9a  |        |    |           | R-9b is wave 6. |
| R-10  |        |    |           | |
| R-11  |        |    |           | |
| R-12  | merged | [#12](https://github.com/hypha-dao/buzz-hypha/pull/12) | `bfa3de73a` | `POST /api/invites` admits a pubkey in the live `39103.shapers` (`Db::is_org_shaper`, one `io_shapers` read per request, no cache); `claim_relay_invite` appends `member_joined` (`actor` = claimant, `detail.via = "invite"`, `detail.minted_by` = the minter) on the claim's own transaction; `GET /api/join-policy` carries `org.transparency_notice` (`api::invites::ORG_TRANSPARENCY_NOTICE`, `Db::is_org_community`: an `io_hosted_agents` row or an `io_shapers` row) and `web/` renders it on `/invite/<code>` above the join controls, gating nothing. Two Postgres-lane proofs in `api::invites`, two in `buzz-db`, two E2E in `e2e_intelligent_org.rs`, one web smoke spec. `shapers.rs`/`apply.rs` untouched. |
| R-13  |        |    |           | |
| C-1   | merged | [#11](https://github.com/hypha-dao/buzz-hypha/pull/11) | `46ef9be4a` | `buzz-sdk/src/intelligent_org.rs`: `build_io_*` for every command (`50001–50021`) and read (`50100–50103`), typed over `buzz-core::intelligent_org`, tags per Protocol §4.8 / §4.3 / §4.7–4.7c; every builder sets `allow_self_tagging`; no builder yields `39100–39105` (`io_state_kinds_have_no_builder`). `buzz-sdk --lib` added to `just test-unit`; `crates/buzz-sdk` added to the retired-tag scan. |
| C-2   | merged | [#22](https://github.com/hypha-dao/buzz-hypha/pull/22) | `cbb3ce363` | `buzz org` in `buzz-cli`: every verb → one `build_io_*` or one REQ; `org bootstrap` is `build_io_shapers_propose` self-add (`bootstrap_args_build_owner_self_add`); `50003`/`50019` `e` is read with `uuid_tag`; `progress note` refuses `not implemented`. Work / drafts / health have the CLI + unit test; relay execution is R-5/R-7 (runbook says so). |
| C-3   |        |    |           | Grows with every R. |

### Waves 2–4

Not opened as waves. D-5 (Agents door defaults) and E-1 (fixtures) need only
R-1 and can start any time; D-0 waits on R-4; A-0/A-1 wait on E-1; O-1 onward
waits on the relay being live. Rows appear here as those early slices land.

| Slice | Status | PR | Merged as | Notes |
| ----- | ------ | -- | --------- | ----- |
| D-0   | merged | [#27](https://github.com/hypha-dao/buzz-hypha/pull/27) | `f010ffc32` | Desktop only. `org` preview feature (V14); routes `/org`, `/org/work`, `/org/work/$itemId`, `/org/my-work` render empty states; sidebar group; live REQ hooks per Protocol §6.5 with backfill/live overlap; `commands.ts` + `useOrgCommands` (C-1 tags, `sign_event` → EVENT). Reuses D-5 `orgAgent` / `useOrgAgent`. Bodies are D-1 / D-2 / D-3. |
| D-5   | merged | [#15](https://github.com/hypha-dao/buzz-hypha/pull/15) | `6163bad20` | Desktop only. `BUILT_IN_PERSONAS` and `BUILT_IN_TEAMS` are empty; Fizz/Honey/Pollen live on as `SAMPLE_PERSONAS` (migration lookups only) and a carried-over Block store demotes them to custom personas on load; the Welcome Team is retired. `features/org/{orgAgent,useOrgAgent}.ts` read `39103.agent` (live REQ + reconnect invalidation): the door hides it, the sidebar pins its DM first in every sort mode. Work sync is a disabled `Templates` card. Welcome kickoff degrades to a plain channel when the starter personas are absent. Mock bridge: `mock.org` serves `39103` and seeds the agent DM. |
| E-1   | merged | [#16](https://github.com/hypha-dao/buzz-hypha/pull/16) | `85b597c66` | `crates/buzz-org-agent` (stub: `fixtures` loader + decoder, no agent yet) and `tests/eval/fixtures/`: `orgs/{river,energy,cold}/seed.json` (+ `seed.pt.json`, `seed.es.json`, `manifest.json`, `health-gold.json`), four sequences (`weekday-hall`, `hall-electrics`, `iberia-pilot`, `andalusia`: before / gate / outcome-a / outcome-b deltas + `sequence.json`), `who-is-needed/{river,energy}.json`; all generated from `data.ts` by `tests/eval/fixtures/generate.mjs` (Node, no deps; `prototypes/org-preview` stays outside pnpm). `just org-fixtures-check` + CI job `Intelligent-Org Fixtures` prove the checked-in files match; `cargo test -p buzz-org-agent` (in `just test-unit`) verifies, decodes, and round-trips every event through `buzz-core::intelligent_org` with the Protocol §4 tag set. |
| A-0   | merged | [#21](https://github.com/hypha-dao/buzz-hypha/pull/21) | `c3847668e` | `pub mod llm`; `CompleteOverrides { temperature: Option<f32>, tool_choice: Option<String> }` on `Llm::complete_with`. `Llm::complete` is that path with both `None` — today's request (no `temperature`; OpenAI-family `tool_choice: "auto"` when tools are present). A `Some` is written onto the JSON body as a number / string. |
| A-1   | merged | [#30](https://github.com/hypha-dao/buzz-hypha/pull/30) | `6251d748f` | `crates/buzz-org-agent` skeleton and ruler: `OrgState` + `apply` + the § 5.2 table; Protocol §4 tag checks live in `inbound` (the E-1 decoder calls through); fixture loader is `#[cfg(any(test, feature = "fixtures"))]`; `RelayLink` / outbox / `ModelClient` (`BuzzAgentModel` via `Llm::complete_with`, `Recorded`); `judge` (15 gates incl. `sequence`); `route`; `publish` `Permitted` chokepoint (`50009` only from `jobs_impl::done_from_talk`); `FakeRelay`; harness loader over E-1; `run` / `dry-run` / `replay` / `doctor`. Nothing drafts. |
| E-2   | open | [#39](https://github.com/hypha-dao/buzz-hypha/pull/39) | | Gold cases for the four move suites and Eval § 5 (J1b, J3d, J6, J7, J8/J8b, J9, J10, J11) over River, Energy, cold; each with `why_gold`; sequence (gate first, next wave, outcome changes the plan, no invented order) and who-is-needed (`requires`, `unfilled`, skill over availability) in the first cut; vacuous-title list; model-judge prompt v1; κ procedure in `tests/eval/README.md`. Human-authored under `tests/eval/cases/` — `generate.mjs` does not emit them. 155 cases; negatives ≥ half per suite. Ready for human review of gold cases. |

### Waves 5–8

Sliced when their wave opens (plan § Waves 5–8).

---

## Follow-ups discovered

Things a slice found that were out of its scope. Each is either fixed
(strike it and cite the PR) or still open. Add to this list; do not silently
absorb an item into an unrelated slice.

- **`pgschema` silently drops table-level `CHECK` constraints whose text
  contains `IS NOT NULL`.** Found in R-2a (probed in isolation: named or
  unnamed, `CHECK (a IS NULL OR b IS NOT NULL)` vanishes from the
  desired-state database; `CHECK (NOT (a AND b IS NULL))` survives).
  `io_drafts` is written in `IS NULL`-only form and
  `intelligent_org_schema_parity_between_desired_state_and_migrations`
  compares `CHECK` sets across both bootstrap paths. **Still exposed today:**
  `push_leases` — its multi-column active/inactive `CHECK` is absent from the
  desired-state DB (`relay_invites`' table `CHECK` has no `IS NOT NULL` and
  survives). A relay bootstrapped from `schema.sql` therefore accepts
  `push_leases` rows a migrated relay rejects. Fix: rewrite the constraint in
  a representable form, or put it in
  `scripts/reconcile-schema-after-pgschema.sql` with an assertion, and extend
  the parity test's table list beyond `io_*`. (R-2a's PR body over-states
  this as both tables; this entry is the correction.)
- **Fork CI has a standing red set** unrelated to the org work: the Docker
  image builds (`Build (linux/*)`, `Build public push gateway (linux/*)`,
  `Qualify relay image source`), `Desktop Domain / Desktop Smoke E2E (1,2,4)`,
  and `Relay and PostgreSQL / Desktop E2E Integration`. They fail identically
  on `main`. Until they are fixed (secrets, runners, or disabling them on the
  fork), judge a PR by the **Rust lanes** — `Rust / Rust Lint`,
  `Rust / Unit Tests`, both `Server Cross-Compile (*-linux-musl)`,
  `Windows Rust` — plus the local Postgres lane below. GitHub CI becomes the
  gate when the fork goes to production (owner's decision, 17 Sep).
- **`PostgreSQL Tests` flakes** on
  `buzz-db runtime::replica_fence::postgres_tests::cluster_global_probe_commits_tokens_and_sessions_prove_coverage`
  (`first probe: MaskedActivity { masked: 1 }`). The probe reads
  `pg_stat_activity` cluster-wide and fails closed when any *other* client
  backend is non-idle with no `xact_start` yet — which, with nextest running
  ~400 Postgres tests in parallel against one CI cluster, is simply a
  neighbouring test between statements. Seen once on R-12
  ([#12](https://github.com/hypha-dao/buzz-hypha/pull/12), rebased head;
  the same lane was green on the previous head and the test passes 3/3
  locally in isolation). Not an org-work regression. A deflake would run
  the `replica_fence` probes in their own nextest test group
  (`test-groups` with `max-threads = 1`, or serialising against the whole
  lane) so no sibling backend is in flight during the sample.
- **`Rust / Unit Tests` flakes** on
  `buzz-agent::fake_llm::cancelled_turn_with_usage_emits_notification_before_response`
  (asserts `stopReason: cancelled`, sees `null`). nextest fail-fast then
  cancels ~100 other tests. Seen on `main` and on R-2a's first run; a rerun
  passes. Not an org-work regression; worth a deflake in `buzz-agent`.
- **The `PostgreSQL Tests` lane runs in PR CI only when the paths filter
  selects it.** `Relay Artifact Producer / PostgreSQL Tests` was skipped on
  R-2a/R-2b; on R-3 ([#9](https://github.com/hypha-dao/buzz-hypha/pull/9))
  `PostgreSQL Domain / PostgreSQL Tests` ran and passed. Do not rely on it:
  every slice that touches `buzz-db` or `buzz-relay` handlers must run the
  lane locally (recipe below) and say so in the PR, and should check which
  Postgres job CI actually ran.
- **`mesh_demo::demo_join_forwarded_arm_round_trips_echo`** fails locally on
  clean `main` (HTTP 504 from an environment dependency). Ignore locally;
  unrelated.
- **`io_hosted_agents` has no writer yet.** Design § Where it runs says the
  operator's supervisor records `community → pubkey`; nothing in the repo
  does (no `buzz-admin` subcommand, no bootstrap hook). R-3 reads the table
  at `39103` bootstrap and its tests seed the row by SQL. A community
  bootstrapped on a relay with no row gets `agent: null, agent_hosted:
  false` — valid `39103`, but not the hosted default the Design promises.
  Owner: the operator/agent-hosting slice (wave 3 `A-*`/`O-*`), or a small
  `buzz-admin org hosted-agent set` sooner.
- **nostr's `EventBuilder` drops a `p` tag that names the signer** unless
  `.allow_self_tagging()` is set. The bootstrap `io_shapers_propose` *is*
  that self-tag, so a client built on the default builder sends a command
  with no `p` and is refused `invalid: op=add and op=remove need a p tag`.
  Found in R-3 (its test harness and `state::sign` both set the flag).
  **C-1 must set `allow_self_tagging()` in `build_io_*`** for every command
  whose `p` may be the sender (`50001`, `50006`, `50021`, …), and C-2's
  `org bootstrap` inherits it.
- **`#shapers` roles are R-3's call, not the Protocol's.** §6.4 says "adds
  the owner and the org agent" without roles. R-3 gives every Shaper
  `admin` (manage members and settings; cannot delete the room — the relay
  is its only authority, and a deleted room would strand every later Shaper
  change) and the agent `member` (`apply::SHAPER_ROOM_ROLE`,
  `apply::AGENT_ROOM_ROLE`). R-8 settles the agent's role in every other
  room; if it chooses differently, change the constant and the Design's
  Shapers section together.
- **Bootstrap does not yet backfill the agent into existing channels and
  DMs** (§6.4 "adds the org agent to every channel and DM that already
  exists", ledger `agent_membership_synced why=bootstrap`). By plan that is
  R-8; until it lands, a community bootstrapped with existing rooms has the
  agent only in `#shapers`. R-8 should add the backfill to
  `shapers::bootstrap` inside the same transaction, not as a post-commit
  sweep.
- **`buzz-db runtime::postgres_tests::writer_pool_rejects_non_read_committed_database_default`
  hangs forever** on a native Postgres 16 (no Docker) host — `Db::new` never
  returns the expected `requires READ COMMITTED` / `pool timed out` error
  after the scratch database's default isolation is set to `repeatable
  read`. Reproduced on clean `main` (`b4924027`) and on R-3; not org work.
  The `postgres-ci` nextest profile has `slow-timeout = "60s"` but no
  `terminate-after`, so the whole lane waits on it. Until fixed, run the
  lane with `-E 'not test(/writer_pool_rejects_non_read_committed/)'` or
  kill that one test process; the other 401 tests pass (R-3 run). Fix
  candidates: a `terminate-after` on the profile, and a bounded connect in
  the test.
- **The relay refuses to start without MinIO/S3** — the git object-store
  conformance probe runs at boot and is fatal. `BUZZ_GIT_CONFORMANCE_PROBE=false`
  skips it for local org work that never touches media (recipe below).
- ~~**No lane ran `buzz-sdk`'s tests.** `just test-unit` enumerates crates
  by hand and `buzz-sdk` was not among them, so the C-1 "Proves" tests (and
  the crate's 306 existing builder tests) would have run nowhere in CI.~~
  Fixed in C-1 ([#11](https://github.com/hypha-dao/buzz-hypha/pull/11)):
  `cargo nextest run -p buzz-sdk --lib` in `just test-unit` and the
  plain-cargo fallback in `scripts/run-tests.sh`. Locally:
  `cargo test -p buzz-sdk --lib intelligent_org`.
- ~~**The retired-tag guard did not scan `crates/buzz-sdk`.**
  `RETIRED_TAG_SCAN_ROOTS` in `scripts/check-org-kinds-parity.mjs` listed
  `buzz-cli` and the desktop/agent roots but not the SDK the CLI builds
  on.~~ Fixed in C-1: `crates/buzz-sdk` added; `just org-kinds-check` fails
  on a retired name there.
- ~~**The SDK half of the `allow_self_tagging` follow-up above is done**
  (C-1: `intelligent_org::io_event` sets it on every `build_io_*`;
  `self_add_bootstrap_keeps_its_p_tag` proves the bootstrap keeps its `p`
  and shows nostr's default builder dropping it). Strike that bullet when
  C-2's `org bootstrap` is built on `build_io_shapers_propose` and its
  argument → event test covers the owner's self-add.~~ Struck in C-2:
  `org bootstrap` is `build_io_shapers_propose` (`ShapersProposal::Add`
  naming the signer, `vote_agree=false`); `bootstrap_args_build_owner_self_add`
  covers the self-`p`.
- **`50003`, `50014`, and `50019` carry a proposal UUID in an `e` tag**
  (Protocol §4.8). nostr's `Tag::parse` accepts it (standardization is
  lazy) and R-3 reads it with `uuid_tag`, but nostr's typed accessors
  (`Tags::event_ids`, `as_standardized`) return nothing for it. C-2 and C-3
  must read that tag as a plain value, as `handlers/intelligent_org` does,
  never through the `EventId` helpers; the C-1 test
  `vote_accept_and_step_down_tag_sets` pins the wire form.
- **Protocol §4.8 gives `50021` no `p` tag** ("`pubkey` is the signer,
  never a tag"), so `build_io_profile_set` has no self-`p` to preserve; the
  flag is set uniformly anyway. The self-`p` builders are `50001`
  (add/remove/agent), `50005` (`offer_to`), `50006`, `50013`, `50015`,
  `50016`, `50100` (`needs` = author), and `50102` (`dri` = signer) —
  `every_builder_whose_p_may_be_the_signer_keeps_it`.
- **The Prototype map's inventory paragraph does not match `data.ts`.**
  Found in E-1. §1 Inventory › Data says "116 titled rows; ticket states 23
  done, 21 doing, 11 open, 3 waiting". A walk of `data.ts`
  (`projectsData`/`energyOrg.projects` `tickets[]` with nested `children`,
  plus `ticketsData`/`energyOrg.tickets` and `pricesChildren`) gives **22
  done, 20 doing, 5 open, 2 waiting**, plus the 5 live `ticketsData` rows
  that carry `dri`/`due` and no `state` (River 20 rows, Energy 34). The
  mapping table (§3) is unambiguous and E-1 followed it — a live row is
  `doing` when it has a `dri`, else `open` — so the fixtures and
  `river_seed_counts_match_the_prototype_map` /
  `energy_seed_counts_match_the_prototype_map` bind to the `data.ts` counts,
  not to the paragraph. E-1 changed no document; the paragraph is the doc
  owner's to reconcile (or to drop, since §3 is the spec).
- **`data.ts` names people outside `space.members`.** Rafi (`HOLDERS`,
  `TICKET_SUGGESTED`), Eli (the investor persona), and You (the reader) hold
  or are offered rows in River but are not in `space.members` (7), and
  `energyOrg.space` has no `members` at all — only `founder` and
  `shapers`, while its rows name seventeen people. Every command is sent by
  "any member" (Protocol §3) behind Buzz's NIP-43 gate, so E-1 seeds
  everyone a row names as a NIP-43 member (River: 11 = 7 + Rafi + Eli + You
  + the agent, `river_seed_counts_match_the_prototype_map`; Energy: the
  list in `lib/constants.mjs`). The Prototype map §3 row for
  `space.members` therefore under-describes the fixture. Either the
  prototype should list them or the map should say holders are added — the
  doc owner's call.
- ~~**`buzz-org-agent` is a stub.**~~ A-1: loader is
  `#[cfg(any(test, feature = "fixtures"))]`; Protocol §4 tag checks live in
  `inbound` and run from `OrgState::apply`; `fixtures/decode.rs` wraps
  that path. `tests/eval_fixtures.rs` is compiled as a lib unit test via
  `#[path]` so `cargo test -p buzz-org-agent` stays one command.
- **The fixture generator takes ~25 s.** It signs ~3 000 events with a
  pure-JS BigInt secp256k1 (no dependencies, so `prototypes/org-preview`
  stays outside pnpm). Fine for the `Intelligent-Org Fixtures` CI job
  (10-minute timeout) and `just check`; if it grows past a minute, cache
  signatures by `(id, signer)` in a checked-in side file or move signing to
  a tiny Rust binary under `buzz-org-agent` — do not add an npm dependency.
  The generator needs Node ≥ 23.6 (type stripping of `data.ts`); Hermit pins
  24.
- **The agent DM is not yet named from the agent's `kind:0`** once the relay
  excludes `39103.agent` from the DM identity (Protocol §6.8 "The agent
  DM": `p = [member]`, "the sidebar names it from the agent's `kind:0`").
  D-5's `isOrgAgentDm` already recognises that `{member}` shape and pins it,
  but `useDmSidebarMetadata` / `resolveChannelDisplayLabel` still label a DM
  from its listed participants, so a self-identity DM would read as the
  member's own name and carry their avatar and presence. Nothing produces
  that shape today — the relay still lists the agent in the `p` tags, and
  the mock bridge seeds `{member, agent}` — so this waits on the relay slice
  that ships the §6.8 identity exclusion (R-8) and should land with it, or
  as a small D follow-up: label, avatar, and presence from `39103.agent`'s
  profile when `isOrgAgentDm` is true.
- **Block's Welcome-channel onboarding is dormant, not removed.** With no
  starter personas the Welcome Team cannot be provisioned; D-5 makes
  `ensureWelcomeTeam` throw `WelcomeTeamUnavailableError`, which the seeding
  path treats as "no team" (canvas still seeds, channel still counts as
  ensured), and holds the kickoff stage back so Welcome reads as an ordinary
  empty channel. The team-provisioning code, the kickoff stage, and the
  Block-era welcome copy all remain in the tree. Decide whether the org
  agent's DM (the Personal Assistant surface, Design § Surfaces) replaces the
  Welcome kickoff outright, then delete the dormant code — a wave-3 `O-*`
  concern once the agent is live.
- **The Agents door still shows Block-era "Agent teams".** D-5 empties
  `BUILT_IN_TEAMS`, so a fresh store has none, but the section and its
  catalog UI stay. Whether teams of members' own agents belong on the Hypha
  door at all is a product call the Design does not make; leave the section
  until it does.
- **`scripts/post-screenshots.sh` is hardwired to `block/buzz`.** `REPO`,
  the raw-URL base, and the `gh pr comment` target all name the upstream
  repo, and `GH_USER` comes from `gh api user`, which an app-token `gh`
  cannot call. Run as-is on this fork it would comment on the wrong
  repository and link images to a commit that repo does not have. D-5
  posted its screenshots by running the same blob-tree / `--force-with-lease`
  mechanics with `REPO=hypha-dao/buzz-hypha` and the branch
  `agent-screenshots/cursor-agent`, then posting the comment separately.
  Fix: derive `REPO` from `gh repo view --json nameWithOwner` (or accept an
  env override) and fall back to a fixed branch name when `gh api user`
  fails. Same for the `gh pr view … --repo block/buzz` cleanup commands in
  AGENTS.md § PR Screenshots.
- **Adding a smoke spec re-cuts the Playwright shards.** `--shard=N/4`
  splits the smoke project by test count, so `org-agent-defaults.spec.ts`
  moves the boundaries of `Desktop Smoke E2E (1–4)`; a spec that was in the
  green shard 3 may now run in a standing-red one and vice versa. Judge the
  shards by which specs failed, not by shard number, until the standing red
  set is fixed.
- **`e2e_intelligent_org.rs` runs in no CI lane.** `Relay E2E`
  (`.github/workflows/_ci-relay.yml`) selects `e2e_relay invite`, the persona,
  team-catalog, interop, and project suites — not this file. R-3 and R-12 ran
  it locally against `just relay` (recipe below). Adding `--test
  e2e_intelligent_org` to that step is a one-line change once someone checks
  that the CI relay's `DATABASE_URL` is reachable from the test process (the
  suite seeds `communities`, `relay_members`, `io_hosted_agents` by SQL) and
  that its `REQUIRE_RELAY_MEMBERSHIP` posture matches the dev relay. Until
  then the E2E proof for every R slice is local-only; say so in each PR.
- **`web/tests/e2e/smoke.spec.ts` runs in no CI lane either.** The `Web`
  job runs `just web-check` and `just web-build`; Playwright is wired for
  the desktop only. R-12's landing-page spec ran locally with `cd web &&
  pnpm test:e2e:smoke` (7 specs). A `Web Smoke E2E` step mirroring the
  desktop one would close this; it needs `playwright install chromium` in
  the job.
- **`member_joined` is written by the invite claim only.** Owners and
  admins keep Buzz's direct add (`kind:9030`) and the founder is a member by
  creating the community; Protocol §6.6 calls those relay administration,
  outside the org's decisions, and R-12 left them without a ledger row. The
  Overview's "who invited whom" (§6.6) therefore starts at the first invite;
  founding and directly-added members have no `member_joined`. If R-9a or
  the Overview wants a complete membership history, decide whether the
  direct-add handler writes `member_joined` with `detail.via = "admin"` (a
  Protocol §6.2 verb-list addition) or the Overview derives founders from
  `relay_members.added_by IS NULL`. R-12 also writes the row for every
  community on the relay, org or not — the ledger is a fact table and the
  claim does not know whether a `39103` will be bootstrapped later.
- **`member_joined.detail.minted_by` is `relay_invites.created_by` as
  stored** — a pubkey hex string the mint handler wrote from the NIP-98
  signer, never re-validated as 32 bytes at claim time. The `io_ledger.actor`
  column is `text`, so this matches the rest of the ledger; a reader that
  wants bytes must parse and may see a legacy or malformed value from an
  invite minted before R-12 (none exist on the staging relay today).
- **The org agent's greeting DM on join** (Protocol §6.6: "the org agent
  greets the new member in a DM") is agent work — wave 3, on the agent's
  `member_joined` ledger read — not part of R-12.
- **`Db::is_org_community` treats a retired hosted agent as still making
  the community an org.** It tests `EXISTS (io_hosted_agents WHERE
  community_id = $1)` with no `retired_at IS NULL`, exactly D7's wording
  ("has an `io_hosted_agents` row or a `39103`"). A community whose hosted
  key was retired *and* whose `39103` was never bootstrapped — an operator
  provisioned, then withdrew — keeps showing the notice. That is the
  conservative failure (a notice nobody needed) and matches the document;
  if the operator slice wants withdrawal to also withdraw the notice, add
  the predicate there and update D7 together.- **Time-driven proposal and seat transitions are R-6's; R-4a only refuses
  at the edge.** A vote at or after `39102.expires_at` is refused `invalid:
  proposal has expired`, and an `io_shaper_accept` after
  `at + offer_window_secs` is refused `invalid: the offer has lapsed`, but
  the `39102` stays `open` and the seat stays in `39103.offered` until the
  scheduler writes `expired` / `shaper_offer_lapsed`. A new `shapers/add`
  for a `p` whose seat has lapsed is accepted (`authorize::has_live_seat`
  ignores lapsed seats), so R-6's lapse sweep must tolerate a `p` that
  already has a fresh seat — drop by `(p, proposal)`, not by `p`.
- **`shapers/agent` execution does not yet move the agent's memberships.**
  §5.3 says a passed `agent` op moves membership in every channel and DM
  from the old agent to the new one (ledger `agent_membership_synced`). R-4a
  rewrites `39103.agent`/`agent_hosted` and the `#shapers` roster (the R-3
  sync swaps the agent row there) and nothing else — by plan that is R-8,
  which should add the move to `shapers::execute` for `ShapersOp::Agent`
  inside the same transaction.
- **`shapers/add` does not require its `p` to be a NIP-43 member.** The
  Protocol only says so for `op=agent`, and R-4a follows it. A seat can
  therefore be offered to a pubkey the door will not admit under
  `REQUIRE_RELAY_MEMBERSHIP`; the offer lapses unaccepted. Either the Design
  should say that an add implies (or requires) an invite, or `open_shapers`
  should check membership for add too — a one-line change once decided.
- **Passing a `shapers/add` for a `p` who is already seated is a no-op, not
  a refusal.** Opening is refused (`invalid: already a Shaper`), but a
  proposal that was open when `p` got a seat by another add still passes on
  its votes and executes nothing (no `shaper_offered`, no `39103` rewrite);
  likewise a `remove` whose `p` already stepped down. This is R-4a's
  reading of §5.3 "any open proposal keeps its stored `eligible`" — the
  proposal is decided honestly, the execution is idempotent. The
  Protocol's §5.3 `shapers` rows should say so explicitly. The converse
  edge: when execution itself must refuse (a `remove` that would now leave
  zero Shapers, an `agent` whose `p` lost membership or became a Shaper
  mid-vote), the **passing vote is refused** and the proposal stays `open`
  rather than passing without effect; nothing is written. R-6 should treat
  such a proposal like any other — it expires.
- **Two byte-identical commands in one second are one Nostr event.** Same
  signer, tags, content, and `created_at` second → same `id` → the second
  is `duplicate: already processed`, not a fresh refusal. This is correct
  relay behavior, but it bit a Postgres-lane test that re-sent an identical
  open expecting `invalid: already a Shaper`. Test authors: vary the
  content (`why`) between otherwise identical commands.
- ~~**`tool_choice: Some(s)` is written as a JSON string.**~~ A-1's
  `BuzzAgentModel` writes `CompleteOverrides { tool_choice: Some(req.tool_name) }`
  as the raw wire string (not the provider forced-tool object). Widening
  to `Option<Value>` / per-family encoding is still open if a provider
  rejects a bare name.
- ~~**`Llm::summarize` does not take the new fields.**~~ A-1 uses
  `Llm::complete_with` for structured output. `summarize` stays the
  handoff path.
- ~~**The E-1 follow-up that asked A-0 to move the fixture loader**
  behind a `fixtures` feature is deferred to A-1.~~ Done in A-1 (see
  the stub entry above).
- **`org direction history` is `{kinds:[50002], "#d":[slug]}`.** The CLI
  surface table maps history to `50002`; Protocol §6.5's direction-page
  filter is `{kinds:[39102], "#t":["direction"], "#s":["passed"]}`. C-3
  and the desktop Direction page should not assume they are the same REQ.
  C-2 followed the verb table.
- ~~**DRI proofs seed `io_work_items` by SQL.** R-4b can name a holder for an
  item that exists; it cannot create one. The Postgres and E2E DRI tests
  insert a projection row the way R-5a will, then drive `50015`/`50003`
  through `ingest_event` / `POST /events`. Strike this when R-5a lands and
  those tests open a root by a passed `project` instead.~~ Struck in R-5a:
  `a_passed_dri_sets_the_holder_and_the_subject_cannot_vote` (Postgres and
  E2E) opens its roots through a passed `project` (open / `suggested_dri` /
  offer+accept).
- **Two `direction` proposals on the same `base`:** the first to pass writes
  the version; the second passing vote is refused `invalid: stale base` and
  the proposal stays `open` (nothing written). Same shape as R-4a's
  execution-time refusal. R-6 expires it.
- **A `dri` that names the only Shaper as subject has `eligible = []` and
  `needed = 0`,** so it passes with no vote. Protocol-correct (`needed_for(0)
  = 0`, `agrees ≥ needed`), and the opener's `vote=agree` is ignored
  (not eligible). Worth a sentence in §5.3 so the empty-eligible case is
  not read as a bug.
- **Home roster / `maintainers` sync on a passed `dri` is R-9a.** R-4b
  rewrites the `39101` (`dri`, `accepted`, offer cleared) and writes
  `item_accepted`; it does not touch the project room.
- **D-0 has no command UI yet.** The Playwright proof for kind/tags binds
  the production `publishOrgCommand` path through
  `window.__BUZZ_E2E_ORG_COMMANDS__` (`useOrgCommandE2eBridge` on Overview,
  e2e builds only). D-1 / D-2 / D-3 should keep asserting on that same
  `sign_event` capture; drop the window hook once a real tap exists.
- **My Work REQs `39103` in addition to the Protocol §6.5 set** so the
  hook can apply the Shaper `#n=shaper` addendum once the newest
  `d=shapers` names the viewer. D-5 already watches `39103.agent`; a later
  door slice can share that event instead of a second live REQ.
- **Adding `org-skeleton.spec.ts` re-cuts the Playwright smoke shards**
  the same way D-5's `org-agent-defaults.spec.ts` did. Judge shards by
  which specs failed, not by shard number.
- **The mock bridge now accepts `50001–50021` without an `h` tag.** Org
  commands are community-scoped (Protocol §4.8); the mock's default EVENT
  path required a channel tag. D-0's command proof (`io_done` →
  `__BUZZ_E2E_SIGNED_EVENTS__`) needs that exemption. The real relay
  already routes these through the R-3 ingest path.
- ~~**Protocol §6.8 vs the R-8 prove on DM `39002`.** The Protocol said the
  agent's membership is visible on `39002`; the named prove (and V5) said
  a two-human DM's `39000`/`39002` list two `p` while `channel_members`
  holds three.~~ Fixed in [#29](https://github.com/hypha-dao/buzz-hypha/pull/29):
  Protocol §6.8 now names `channel_members` as membership truth, DM
  `39000`/`39002` as identity (humans only), and only a non-DM channel's
  `39002` as listing the agent. Clients do not subtract `39103.agent`
  from a DM `39002`.
- **Bootstrap / `shapers/agent` backfill writes `channel_members` only.**
  It does not re-emit `39000`/`39002` for rooms that already existed.
  A `9001` that drops the agent is repaired on the next `41010` /
  `create_dm` of that identity (the existing-found path re-puts the live
  key), not by a sweep, and that repair also does not rewrite discovery
  until the next membership-shaped emit.
- **`e2e_relay` R-8 names do not match the filtered CI step.**
  `_ci-relay.yml` runs `e2e_relay invite`,
  `nip43_membership_snapshots_are_rejected`, and `nip29_departure_wire`.
  The identity proofs live in that file as the plan names it, but CI
  actually runs the same cases via `e2e_nostr_interop` (unfiltered) and
  the Postgres ingest lane. Adding `--test e2e_intelligent_org` remains
  the C-3 follow-up above.
- **The 0032 roster fence had to learn the V5 exception.** A DM `39002`
  that omits `39103.agent` is rejected by `guard_channel_roster_snapshot`
  unless the canonical set also drops that key. Migration `0046` (and
  `schema.sql`) exclude `io_shapers.agent` from the fence's canonical
  set when `channels.channel_type = 'dm'`. A missing human still fails
  closed. Non-DM rooms are unchanged.
- **A-1 `run` / `replay` do not open a socket or load fixtures.** The
  CLI is the flag surface (`run`, `dry-run`, `replay`, `doctor`); the
  harness proof is `cargo test -p buzz-org-agent`. A live
  `buzz-ws-client` `RelayIo` impl and snapshot (`state.bin` every five
  minutes) wait on a relay (O-1 / A-2).
- **`ChildDoneBriefUnmet` reads STATE's last ticket-draft `coverage`.**
  There is no separate J2 store yet. A-2's J2 should keep writing that
  coverage onto the draft the table already reads, or this row will miss
  a brief that never produced a `50100`.
- **`50009` from talk is the chokepoint only.** `jobs_impl::done_from_talk`
  signs when `IO_DONE_FROM_TALK_ENABLED` is set; the Protocol §5.5
  check-list is A-2+. R-5b accepts `50009` only from the item's `dri`
  (`restricted: not the holder` for anyone else, including `39103.agent`).
- **Protocol §6.2 does not list a ledger verb for `io_reopen`.** R-5b
  writes `item_reopened` (one row, `object_type=work_item`). Add it to
  the verb catalog when the Protocol is next edited; do not invent a
  second name.
- **`io_work_items.done_at` is the reopen clock.** R-2a created the
  column; `apply` now sets it on the first write into `done` and clears
  it on every path that leaves `done` (reopen, and any later rewrite of
  a live item). The 7-day window reads that column, not the command's
  `created_at`.
- **E-2 gold uses `imagine` overlays** for lines and profile skills the
  E-1 seeds do not hold (the hall-roof grant, a brand-money rejection,
  Rafi's `grant-writing`, a new second-island line with no shortlist).
  A-2 should turn the ones it runs into sequence-style deltas rather than
  re-describing them in prompts. The first-cut sequence and who-is-needed
  cases already bind the E-1 snapshots (`weekday-hall`, `iberia-pilot`,
  `hall-electrics`, `who-is-needed/{river,energy}.json`).
- **`DriDraft.suggested` is required** on the wire (Protocol §4.3) while
  Eval J1b gold allows `suggested_holder: null` with `unfilled`. E-2
  records the nobody-fits case at intent level and does not invent a
  payload that the schema cannot parse. A later schema/Protocol sentence
  — not this slice — should name that shape.
- **A child create / offer / accept / decline rewrites the parent's
  `39101`** so `children` stays on the live event the Work door will read.
  §5.1 rule 7 says one `39101` per command; the live head count still
  grows by one item (the parent is replaced, not added). ~~R-5b's done /
  release should keep the same parent rewrite.~~ Done in R-5b: child done /
  release / reopen rewrite the parent the same way.
- ~~**`50009`–`50011` / `50018` stay refused**
  (`invalid: kind {k} is not implemented yet`) until R-5b.~~ Struck in R-5b.
- **Project home is still R-9a.** A passed `project` writes no room and
  leaves `39101.home` absent.

---

## Running the checks on this fork

The repository gates are in [AGENTS.md § Quality Gates](../../../AGENTS.md).
What the org slices have actually needed, and what this host lacks:

```bash
. ./bin/activate-hermit
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
just file-size-check
just org-kinds-check                       # any change near kinds/tags
cargo test -p buzz-core -p buzz-auth -p buzz-db -p buzz-relay --lib
```

**Postgres lane** (any change under `buzz-db`, `buzz-relay` handlers,
`migrations/`, `schema/`). It is nextest-driven and needs `cargo-nextest`
plus `psql`/`createdb`/`dropdb` on `PATH` (or `PG_BIN_DIR`). With the
`docker compose` Postgres and Redis up:

```bash
export BUZZ_POSTGRES_ADMIN_URL=postgres://buzz:buzz_dev@localhost:5432/buzz
export PGHOST=127.0.0.1 PGPORT=5432 PGUSER=buzz PGPASSWORD=buzz_dev
export REDIS_URL=redis://localhost:6379
scripts/test-postgres-test-discovery.sh    # new tests must be in postgres_tests modules + #[ignore]
scripts/postgres-test-run.sh               # whole lane, ~1 min after build
scripts/postgres-test-run.sh -p buzz-db --lib -E 'test(/intelligent_org/)'   # one slice
```

`cargo install cargo-nextest --locked` puts the binary in Hermit's
`CARGO_HOME` (`.hermit/rust/bin`, already on `PATH` once activated). Native
`createdb`/`dropdb` from a host Postgres install satisfy the client-tool
requirement. R-3 ran the whole lane this way (401/402; see the
`writer_pool_rejects…` follow-up for the one hang) and, scoped:

```bash
scripts/postgres-test-run.sh -p buzz-db -p buzz-relay --lib \
  -E 'test(/intelligent_org|relay_rooms|channel_members|replaceable/)'   # 50 tests
```

Without nextest, the same tests run under plain `cargo test` with
`--include-ignored` (they carry `#[ignore = "requires Postgres"]`), against
the shared dev database rather than a per-test copy:

```bash
export DATABASE_URL=postgres://buzz:buzz_dev@localhost:5432/buzz
cargo test -p buzz-db    --lib relay_rooms                  -- --include-ignored
cargo test -p buzz-relay --lib intelligent_org::postgres_tests -- --include-ignored
```

**E2E lane** (`crates/buzz-test-client/tests/e2e_intelligent_org.rs`, C-3).
Needs a running relay and `DATABASE_URL` for seeding; every test makes its
own community (a fresh `*.localhost` sent in the `Host` header), so the suite
is rerunnable against a shared dev relay. Start the relay without MinIO (a
fresh `.env` from `.env.example` has no relay key; `just bootstrap` writes
one, or `scripts/ensure-local-relay-key.sh .env` alone on a host without
Docker):

```bash
set -o allexport; source .env; set +o allexport
BUZZ_GIT_CONFORMANCE_PROBE=false BUZZ_AUTO_MIGRATE=true cargo run -p buzz-relay
# in another shell
RELAY_URL=ws://localhost:3000 DATABASE_URL=postgres://buzz:buzz_dev@localhost:5432/buzz \
  cargo test -p buzz-test-client --test e2e_intelligent_org -- --ignored
```

**Web lane** (anything under `web/`, e.g. the R-12 invite landing page):

```bash
cd web && pnpm install --frozen-lockfile
pnpm check && pnpm typecheck                 # what CI's Web job runs (plus the build)
pnpm exec playwright install chromium        # once
pnpm test:e2e:smoke                          # builds, serves dist on :4173, runs tests/e2e/smoke.spec.ts
```

To see the real page rather than the mocked `/api/join-policy`, start the
relay with `BUZZ_WEB_DIR=./web/dist` after `pnpm build` and browse
`http://<host>.localhost:3000/invite/<code>` — Chromium resolves
`*.localhost` to loopback, and a non-default port is part of the community
host, so seed the community as `<host>.localhost:3000`.

If the host has no native Postgres client tools (macOS with Postgres only in
Docker), install `libpq` (`brew install libpq`, then
`PG_BIN_DIR=/opt/homebrew/opt/libpq/bin`) and `cargo install cargo-nextest
--locked`. A `docker exec` shim works too, as long as it streams `--file=`
arguments over stdin — the lane passes host paths.

**Desktop lane** (any `D-*` slice; D-5 ran it this way). The Tauri crate
needs the sidecar placeholder files before it compiles (`_ensure-sidecar-stubs`
in the Justfile does this), and Playwright needs `pnpm exec playwright install
chromium --with-deps` once per host:

```bash
cd desktop && pnpm check && pnpm typecheck && pnpm test         # biome, tsc, ~6.5k node tests
just desktop-tauri-fmt-check && just desktop-tauri-clippy
just desktop-tauri-test                                         # cargo test --workspace in desktop/src-tauri
cd desktop && pnpm build:e2e && pnpm exec playwright test --project=smoke org-agent-defaults
cd desktop && pnpm test:e2e:smoke                               # whole smoke project, ~90 min on 4 cores
```

`pnpm check` reports a handful of biome warnings/infos on untouched files and
still exits 0; that is the CI behaviour too. Always build with `pnpm
build:e2e`, never `pnpm run build`, before running specs by hand (AGENTS.md
§ Writing E2E Screenshot Specs).

**Migration edits before merge**: the local `buzz` database records each
applied migration's checksum. If you change an unmerged migration file after
applying it, drop its tables and `DELETE FROM _sqlx_migrations WHERE version
= N` before rerunning, or tests fail with a checksum mismatch. Never do this
to a merged migration — add a new one.

**Pre-push hooks** run clippy, tsc, the file-size gate, and unit tests
(~6 min). Skipping them with `--no-verify` is acceptable only when those
lanes were just run by hand; say so in the PR.

---

## Conventions this log adds

- One PR per plan slice (or per half, `R-4a`/`R-4b`, when the plan says so).
  Branch `io/<slice>-<short-name>`; title
  `feat(intelligent-org): <slice> — <what>`; body carries the slice's plan
  row, "Proves" mapped to test names, size vs. the ~800-line guideline, and
  the checks actually run. Squash-merge with the signed-off commit message.
- The PR that finishes a slice flips its row here and adds any follow-ups.
- When a slice's tests are Postgres-backed, name the proof tests in the PR so
  the next slice can rerun them with `-E 'test(/…/)'`.
