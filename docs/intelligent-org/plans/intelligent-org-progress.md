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
| R-4a  |        |    |           | **Next.** Every non-bootstrap `shapers` op is rejected `invalid: … not implemented yet` until it lands. |
| R-4b  |        |    |           | |
| R-5a  |        |    |           | |
| R-5b  |        |    |           | |
| R-6   |        |    |           | |
| R-7   |        |    |           | |
| R-8   |        |    |           | |
| R-9a  |        |    |           | R-9b is wave 6. |
| R-10  |        |    |           | |
| R-11  |        |    |           | |
| R-12  |        |    |           | Needs only R-3; can run parallel to R-4+. |
| R-13  |        |    |           | |
| C-1   |        |    |           | Needs only R-1; can start now. |
| C-2   |        |    |           | After R-3. |
| C-3   |        |    |           | Grows with every R. |

### Waves 2–4

Not opened. D-5 (Agents door defaults) and E-1 (fixtures) need only R-1 and
can start any time; D-0 waits on R-4; A-0/A-1 wait on E-1; O-1 onward waits
on the relay being live.

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
is rerunnable against a shared dev relay. Start the relay without MinIO:

```bash
set -o allexport; source .env; set +o allexport
BUZZ_GIT_CONFORMANCE_PROBE=false BUZZ_AUTO_MIGRATE=true cargo run -p buzz-relay
# in another shell
RELAY_URL=ws://localhost:3000 DATABASE_URL=postgres://buzz:buzz_dev@localhost:5432/buzz \
  cargo test -p buzz-test-client --test e2e_intelligent_org -- --ignored
```

If the host has no native Postgres client tools (macOS with Postgres only in
Docker), install `libpq` (`brew install libpq`, then
`PG_BIN_DIR=/opt/homebrew/opt/libpq/bin`) and `cargo install cargo-nextest
--locked`. A `docker exec` shim works too, as long as it streams `--file=`
arguments over stdin — the lane passes host paths.

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
