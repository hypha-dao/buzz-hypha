---
title: The Intelligent Organization — Codebase verification
status: current — read before step 1
date: 2026-09-15
verified_against: main @ 2026-09-15 (rust-nostr 0.44)
---

# The Intelligent Organization — Codebase verification

[Current state § What to verify before building](./intelligent-org-current-state.md#what-to-verify-before-building)
lists facts the design leans on. This document checks each one against the
code, adds the five facts the [Development plan](../plans/intelligent-org-development-plan.md)
leans on that Current state did not list, and says what each answer changes.

Every item has a verdict — **CONFIRMED**, **PARTIAL**, or **REFUTED** — a
pointer into the code, and a _consequence_ that names the plan slice or the
Protocol section it changes. The Readiness review's gate reads this table:
step 1 is cleared when every REFUTED and PARTIAL consequence is a named
slice or a pinned decision.

The numbering V1–V15 is the one the Development plan cites. The Current
state list is cross-referenced as CS-n.

## Summary

| #   | Fact                                                    | Verdict       | Changes                                                  |
| --- | ------------------------------------------------------- | ------------- | -------------------------------------------------------- |
| V1  | A `kinds.ts` ↔ `kind.rs` parity test exists             | REFUTED       | R-1 adds one                                             |
| V2  | Multi-letter tag filters (`#needs`, `#item`, …) work    | **REFUTED**   | Protocol tag names (D-11); R-2 pushdown                  |
| V3  | Executor can read a projection and write atomically; relay-signed `d`-keyed global events (CS-2) | PARTIAL | R-3 uses the open `tx`; `publish_dm_visibility_snapshot` is the state template, not `39000` |
| V4  | Operator can grant NIP-43 membership before first login | CONFIRMED     | —                                                        |
| V5  | DM identity survives a hidden member                    | PARTIAL       | R-8: three relay seams, no client change                 |
| V6  | A receipt read can be authorised in one round trip      | CONFIRMED     | R-10 hook location named                                 |
| V7  | Invite mint authz is one seam; `MemberJoined` observable | CONFIRMED    | R-12 one lookup; R-11 needs no extra event               |
| V8  | Relay may author `30617`; relay-side room gets discovery | PARTIAL      | R-9: repo quota, side-effect path, `buzz-protect` tags   |
| V9  | A place exists to host `io_scheduler`                   | CONFIRMED     | R-6 follows the reaper pattern                           |
| V10 | `query_needs_action` exists to extend                   | CONFIRMED     | R-13                                                     |
| V11 | Huddle STT posts are signed by the speaker              | CONFIRMED     | R-14 adds the tag; §5.5 distrust stands                  |
| V12 | `buzz-ws-client` and `buzz-agent::llm` fit the agent    | PARTIAL       | A-0, A-1 as planned (no REQ helper to reuse)             |
| V13 | Sample personas are seeded in one place                 | CONFIRMED     | D-5                                                      |
| V14 | Desktop has a feature-gate mechanism                    | CONFIRMED     | D-0 uses `preview-features.json`                         |
| V15 | `50102` can read the git store at ingest                | **REFUTED**   | Protocol §4.7 `50102` validation shape (D-12); R-9       |

Two REFUTED items change the Protocol before R-1 (V2, V15). Both are listed
as decisions D-11 and D-12 in the Readiness review.

---

## V1 — kinds parity test (Development plan R-1)

**Verdict: REFUTED (the test); CONFIRMED (the ranges).** No integer in
`39100–39149`, `50000–50049`, or `50100–50149` appears in
`crates/buzz-core/src/kind.rs` today — the Protocol's ranges are free.
`desktop/src/shared/constants/kinds.test.mjs` tests
`isConversationalUnreadKind` and a few constants; nothing compares the file
to `kind.rs`, and `mobile/lib/shared/relay/nostr_models.dart` has no such
test either. AGENTS.md says the three must stay in sync by hand.

**Consequence.** R-1 adds a parity test for the org range only — a small
script that greps `39100–39105`, `50001–50021`, `50100–50103` from the three
files and fails on a mismatch — so the mirror is enforced from the first
slice. Not a general parity test for all of Buzz.

## V2 — multi-letter tag filters (CS-1)

**Verdict: REFUTED, at the type level, not the index level.** The relay parses
REQ filters into `nostr::Filter` (rust-nostr 0.44), whose tag filters are
`generic_tags: BTreeMap<SingleLetterTag, BTreeSet<String>>`. A `#needs`,
`#item`, `#parent`, `#status`, or `#skill` key is not representable; the
matcher in `crates/buzz-core/src/filter.rs` iterates `f.generic_tags` and
the SQL mapper in `crates/buzz-relay/src/handlers/req.rs::filter_to_query_params`
reads the same map. This is also the NIP-01 rule: only single-letter tags
are indexed and filterable.

Second finding: even for single-letter tags, only `#e`, single-value `#p`,
`#d` (NIP-33 kinds only), and `#h` are pushed into SQL. Any other tag —
`#t` today — is matched in memory _after_ the SQL page (`filters_match`),
so a filter such as "drafts that need me" would page through every `50100`
and could miss matches past `LIMIT`. `EventQuery.custom_tag` supports one
exact pair via JSONB containment on the GIN index (migration 0004,
`jsonb_path_ops`) and is used only by the HTTP bridge for `buzz-channel`.

**Consequence.**

1. **Protocol (D-11).** Every tag a door filters on becomes a single letter.
   Proposed mapping. The long names are not duplicated as tags: the content
   JSON already carries each field by its full name, so a human reading raw
   events loses nothing.

   | Long name today | Filter tag | Note                                                  |
   | --------------- | ---------- | ----------------------------------------------------- |
   | `item`          | `i`        | work item UUID on `50001–50021`, `50101`, `50102`      |
   | `parent`        | `u`        | "up" — parent item UUID on `39101`                    |
   | `status`        | `s`        | item / proposal state on `39101`, `39102`             |
   | `needs`         | `n`        | pubkey or `shaper` on `50100`                          |
   | `skill`         | `k`        | one per skill on `39105`                              |
   | `kind` (draft)  | `t`        | already the type tag on `39101`/`39102`; D-5 resolves the same way |
   | `note`          | `t`        | `50103` note type; the CLI's `tally` filter becomes `#t`  |

   `t`, `d`, `p`, `e`, `h` are unchanged. `root` on `39101` becomes `r`
   only if a door filters on it; today none does, so it stays a content
   field. The Protocol § 6.5 door table and § 4 tag tables are rewritten
   with these letters, and the `date` bumped, in the same PR as D-1…D-6.

2. **R-2** adds generic single-letter tag pushdown: `EventQuery.custom_tags:
   Vec<(String, Vec<String>)>` rendered as `AND (tags @> $a OR tags @> $b …)`
   per tag, on the existing GIN index — no new migration. The in-memory
   matcher stays as the second check. Bind the test to the production
   query path with a page of 600 `50100`s where only the last one needs
   the reader.

3. **Live fan-out** needs nothing: `filters_match` already matches any
   single-letter tag.

## V3 — projection read and atomic write inside the executor (CS-3, first half)

**Verdict: PARTIAL.** `handlers/command_executor.rs::persist_command_event`
opens a transaction, inserts the command event (or replaces it for `d`-tagged
kinds), and returns the **open** `tx` to the handler
(`PersistResult::Inserted(tx)`). The handler may run any number of reads and
writes on `tx` before `commit`. The seam the org executor needs —
`apply(tx, ledger_row, projection_change, state_event)` in one transaction —
therefore exists. What the module doc also says, and the existing handlers
show, is that today's mutations (`open_dm`, `upsert_workflow`, …) run on the
pool, not on `tx`: "idempotent but not strictly atomic". The org handlers
must not copy that pattern.

Reads before insert (authorisation against `io_shapers`, `io_work_items`)
can run on the pool before `persist_command_event` — they are checks, not
writes — or on `tx` after it. Relay-signed state emission inside the same
transaction is available: `replace_parameterized_event_in_transaction`
(`buzz-db/src/store/replaceable.rs`) takes `&mut tx`. Fan-out after commit
uses `dispatch_persistent_event` as `side_effects.rs` does.

**Consequence.** R-3 as written. Add to its "Proves": _a handler that fails
after the projection write leaves no event, no ledger row, and no `39xxx`_ —
this is the test that binds the atomicity the executor doc says nothing else
here has.

**CS-2, relay-signed community-global addressable events — CONFIRMED,
with a correction to Current state.** `39000` is _not_ the template:
`emit_addressable_discovery_event` stores it through
`replace_addressable_event`, which keys replacement on
`(kind, pubkey, channel_id)` and ignores the `d` tag. `39100–39105` need
`d`-keyed replacement with `channel_id = NULL`, which is
`replace_parameterized_event` — keyed on `(community, kind, pubkey, d_tag)`
— and the relay already uses exactly that shape for its own
`KIND_DM_VISIBILITY` snapshot (`side_effects.rs::publish_dm_visibility_snapshot`:
relay-signed, `d` = viewer pubkey, `channel_id = None`). That function,
including its "created_at strictly greater than the previous" guard, is the
template for `state.rs`. Two registrations go with it: `is_global_only_kind`
lives in `handlers/ingest.rs` (not `kind.rs`) and strips any `h` tag from a
kind it names — `39100–39105` join it so a client-supplied `h` can never
scope them; and `is_relay_only_kind` in `kind.rs` rejects client authorship
of relay-signed kinds — `39100–39105` join that too. R-1 lists both.

## V4 — operator grant of membership before first login (CS-3, second half)

**Verdict: CONFIRMED.** `Db::add_relay_member(community, pubkey_hex, role,
added_by)` (`buzz-db/src/store/relay_members.rs`) is the seam `kind:9030`
uses (`handlers/relay_admin.rs`), and `buzz-admin` already calls it. Bootstrap
can grant the hosted agent's pubkey `member` with `added_by = relay` and
then call `publish_nip43_member_added` / `publish_nip43_membership_list` so
the `8000` and `13534` events exist before any member opens the app. The
operator-written `io_hosted_agents` row is read by the same bootstrap code
directly from the table; `39103` is written afterwards and is the source of
truth from then on.

**Consequence.** None. `buzz-admin` gains `org hosted-agent set <pubkey>`
(writes `io_hosted_agents`) in C-2; the relay's bootstrap is R-3.

## V5 — DM identity with a hidden member (CS-9)

**Verdict: PARTIAL — identity is safe, presentation leaks in two relay
seams plus the DM-open system message; no client code needs to change.**

- **Identity** is the `participant_hash` column on the channel row
  (`buzz-db/src/store/dm.rs::compute_participant_hash`, sorted + deduped
  SHA-256 of the pubkeys), fixed at `create_dm` from the `p` tags plus the
  creator. Adding the agent to `channel_members` afterwards does not touch
  it, so two humans re-opening "our DM" still find one DM. **Good.**
- **Leak 1 — `41011` add-member.** `handle_dm_add_member` rebuilds the new
  participant set from `get_members_for_event_write`, which would include
  the agent, and computes the new hash from it. The next `41010` by the
  humans computes a hash without the agent and opens a duplicate group DM.
  Fix: filter `39103.agent` out of the member set before hashing.
- **Leak 2 — discovery `p` tags.** Desktop (`desktop/src-tauri/src/nostr_convert.rs`)
  and mobile derive `participants` / `participantPubkeys` from the `p` tags
  of the relay-signed `39000`/`39002` for the DM; `emit_group_discovery_events`
  builds those from `channel_members`. Fix: omit the agent from the `p` tags
  of DM-type channels in `group_members_tags` / the `39000` tag builder.
  Access control does not read these events, so nothing else changes.
- **Leak 3 — the `41010` system message.** `handle_dm_open` posts a system
  message whose `participants` list is the full hex set (`command_executor.rs`
  ~374–383). Fix: build that list from the human set.
- **Not a leak — `40901`.** The constant exists (`kind.rs`) and desktop
  optionally merges a `40901` sidecar (`nostr_convert.rs`), but the relay has
  no emitter for it today. Nothing to change; if one is ever added, subtract
  the agent for DM-type channels.
- **Cap.** `open_dm` / `handle_dm_add_member` enforce 2–9 participants. The
  agent must not count against it: apply the cap to the human set.

The agent still receives DM fan-out because `channel_members` holds it —
which is the point.

**Consequence.** R-8 stays one slice and is relay-only. Development plan
risk 2 ("R-8 likely needs a desktop half") is withdrawn; replace it with:
_R-8's proof must assert on `39002` (three `p`), `39000` (two `p`), the
`41010` system message (two participants), a nine-human DM still opening
with the agent as a tenth row, and a second `41010` finding the same
channel._

## V6 — receipt read against the membership gate (CS-10)

**Verdict: CONFIRMED — one hook, one query.** The WS REQ handler scopes every
un-`#h`-scoped filter to the reader's accessible channels in SQL:
`apply_channel_scope_to_query` sets `EventQuery.channel_ids = accessible_channels`
(plus community-global rows). The HTTP bridge calls the same function. So a
`{ids:[…]}` REQ for a message in a room the reader is not in returns nothing
before any result-level gate runs. A receipt exemption is therefore an
`EventQuery` field — `receipt_ids: Vec<Vec<u8>>` — rendered as
`OR id = ANY($receipt_ids)`, filled in the REQ handler when the filter has
`ids` and no `#h`, from a lookup of `io_receipts(cited_id → citing_id)` for
those ids whose citing event is community-global (`39104`, `50100`) or in a
channel the reader can access (`50101`). One extra SELECT, no second round
trip; the result-level gate `reader_authorized_for_event` still applies.
Live fan-out (`event.rs`) needs no change — receipts are historic by
definition.

**Consequence.** R-10 as written, with the location named:
`req.rs::apply_channel_scope_to_query` and the bridge's caller. `io_receipts`
is written by R-7 when a `50100`/`50101` is stored, so R-10 depends on R-7.

## V7 — invite mint authz and `MemberJoined` (CS-5)

**Verdict: CONFIRMED.** `api/invites.rs::mint_invite` does one
`get_relay_member` lookup and tests `role == owner || admin`. Adding
`|| io_shapers.contains(sender)` is a second lookup in the same handler;
org state stays out of NIP-43, as Current state preferred. `claim_invite`
already calls `publish_nip43_member_added` (`8000`) and
`publish_nip43_membership_list` (`13534`) on a successful claim — both
best-effort (`warn!` on failure). The agent's `MemberJoined` signal exists
today.

**Consequence.** R-12: one lookup. R-11: the fallback clause ("if V7 is
refuted, emit a membership-list event") is dropped. The relay's ledger row
`member_joined` (Protocol § 6.6) should be written by `claim_invite` itself,
not derived from the best-effort `8000`, so a failed publish does not lose
the ledger fact — rule 1 of AGENTS.md § Review-Proven Rules.

## V8 — relay-authored `30617` / `30621` and relay-created rooms (CS-6, CS-8)

**Verdict: PARTIAL — nothing refuses the relay as author; three details
change R-9.**

- **Author.** `handle_git_repo_announcement` (`side_effects.rs`) reads the
  owner from `event.pubkey` and checks only the repo-id grammar, the name
  registry, and the per-pubkey quota. No "must be a person" check exists;
  the git read gate and push policy look up `30617` by
  `(community, owner pubkey from the URL, d)` and would find a relay-signed
  one. `30621` (`ingest.rs` project envelope) validates the `a` coordinate
  shape only.
- **Quota.** `BUZZ_GIT_MAX_REPOS_PER_PUBKEY` (default 100) counts repos per
  owner pubkey. Every project home would be owned by the relay key, so the
  hundred-and-first project fails. R-9 must exempt the relay pubkey from
  the quota (or count org repos under the project's DRI while keeping the
  relay as signer — the former is simpler and honest).
- **Path.** The announcement handler runs as a side effect of a _stored_
  `30617`. A relay-side create must store the event through the same
  `replace_parameterized_event` path and then call
  `handle_git_repo_announcement_inner`, inside R-9's transaction where the
  DB parts are concerned; the manifest pointer seed is an S3 write behind
  the deletion lease and is not transactional — R-9 orders it after commit
  and, per rule 1, records a durable retry if it fails.
- **No bare repository.** The relay holds no per-repo disk state; a repo is
  hydrated from the object-store manifest per request (`api/git/hydrate.rs`).
  "Initialise the bare repository" in Protocol § 6.7 means "seed the
  manifest pointer", which the handler already does. The CLI's
  `ensure_default_create_repo` is only a client-side idempotent announce.
- **Push policy (CS-8).** `api/git/policy.rs` promotes a channel `Bot` to
  `Member` for git, applies `buzz-protect` rules from the `30617` tags, and
  gives the repo owner's managed agent owner authority via `is_agent_owner`.
  With the relay as owner, no pusher is "owner" — good; authority is the room
  roster. The relay-signed `30617` must carry
  `["buzz-protect", "refs/heads/main", "push:admin", "no-force-push", "no-delete"]`
  or `main` is open to any member. `refs/heads/io/*` needs no rule. The hook
  callback also receives `is_ancestor` per ref — see V15.
- **Relay-created rooms.** `Db::create_channel` + `add member` +
  `emit_group_discovery_events(channel_id)` is the exact sequence the
  `9007` handler uses, so `#shapers` and project rooms get `39000–39003`
  like a member-created channel.

**Consequence.** R-9 gains three lines: quota exemption, protect tags on the
relay-signed announcement, pointer seed ordered after commit with a retry
record. Development plan risk 3 ("relay cannot sign `30617`") is withdrawn.

## V9 — a home for `io_scheduler` (Development plan R-6)

**Verdict: CONFIRMED.** `crates/buzz-relay/src/main.rs` spawns interval loops
for the NIP-43 snapshot reconciler (`BUZZ_..._INTERVAL_SECS`, default 60) and
the ephemeral channel reaper (`BUZZ_REAPER_INTERVAL_SECS`), plus the
workflow cron. Each is a `tokio::spawn` around a `Db` method that does one
idempotent sweep. `io_scheduler` follows the reaper: one tick, one
`sweep_intelligent_org(now)` on the store, one transaction per transition.

The relay runs as several pods. The two workers that must fire at most
once — `admin_action_worker.rs` and the workflow cron
(`buzz-workflow/src/lib.rs`, `claim_scheduled_workflow_fire`) — do not
elect a leader; each transition is a DB claim (`SELECT … FOR UPDATE SKIP
LOCKED` or a unique `(id, scheduled_for)` row). `io_scheduler` must do the
same: the transition row is the lock, and a tick that loses the claim does
nothing.

**Consequence.** R-6 gains one line: _every transition is a DB claim, never
a process-local decision_; its E2E runs two sweeps concurrently and asserts
each transition once. The test clock hook is an
`IO_SCHEDULER_NOW_OVERRIDE`-style env read only in test config, as the
reaper's interval is.

## V10 — `query_needs_action` (Development plan R-13)

**Verdict: CONFIRMED.** `buzz-db/src/store/feed.rs::query_needs_action`
builds a mention-joined query (`build_needs_action_query`): a `kind IN
(approval_requested, stream_reminder)` list joined to `event_mentions`,
i.e. only events that carry a `p` tag for the reader appear. R-13 extends
the kind list with the three org sources — and the org events must
therefore name their party in a `p` tag, not only in `needs`/`n`. `39101`
already carries `["p", <offered_to>, "", "offered"]` and `39102` one
`["p", <eligible>, "", "eligible"]` per Shaper; `50100` carried the party
only in `needs`. Protocol § 4.3 now requires `["p", <pubkey>, "", "needs"]`
when the party is a pubkey. `needs = shaper` drafts have no fixed pubkey
set, so R-13 surfaces them by joining the reader against `io_shapers`
instead of `event_mentions`.

**Consequence.** R-13 as written plus the `p`-tag rule above; its E2E
asserts that a `50100` without a `p` for the party does not appear in that
party's feed, and that a `needs = shaper` draft appears for a Shaper with
no `p` tag at all.

## V11 — huddle STT authorship (CS-4)

**Verdict: CONFIRMED — the speaker signs.** Transcription runs on the
speaker's own device (`desktop/src-tauri/src/huddle/stt.rs`) and
`pipeline.rs::spawn_transcription_task` signs each `kind:9` with
`state.keys` — the device owner's key — and posts it through `POST /events`
with NIP-98 auth. No transcript tag is set; `events::build_message` has no
extra-tags parameter, so R-14 adds one (or a sibling builder).

**Consequence.** The relay can trust the _author_ of a transcript line as
much as any message; it is the _text_ that is machine-produced. Protocol
§ 5.5's rule stands as written: the `["transcript", "huddle"]` tag is the
client's declaration that lowers trust, never raises it, and an `io_done`
from talk requires a typed confirmation from the holder.

## V12 — `buzz-ws-client` and `buzz-agent::llm` (Development plan A-0, A-1)

**Verdict: PARTIAL.** `buzz-ws-client` offers `connect_authenticated`,
`send_event`, `next_event(timeout)`, `send_raw`, and `publish_event` — a
NIP-42 connect-and-publish client with no subscription management, no
reconnect, and no watermarks. `RelayLink` builds on it (`send_raw` for
`REQ`, `next_event` for the stream) rather than reusing a helper that does
not exist. `buzz-agent::llm` is `mod llm;` (private); `tool_choice` is
hard-coded `"auto"` and there is no `temperature`.

**Consequence.** A-0 and A-1 as written. Nothing to remove.

## V13 — sample personas (Development plan D-5)

**Verdict: CONFIRMED.** `desktop/src-tauri/src/managed_agents/personas.rs`
holds `BUILT_IN_PERSONAS` (Fizz, Honey, Pollen) with tests in
`personas/tests.rs`; `merge_personas` re-inserts any missing built-in on
every load, so emptying the list is the only way to keep the door empty.
D-5 empties the fork's default and keeps the type.

One adjacent gap: `buzz-acp` has `RespondTo::Nobody` (heartbeat-only), but
the desktop `RespondToField.tsx` deliberately omits it and the TS
`RespondToMode` union has no `nobody`. The Work sync template is
mention-driven, so D-5 does not need it; the hosted org agent is configured
by `buzz-admin`, not the desktop, so A-0 does not either. Noted so nobody
reaches for it.

## V14 — desktop feature gate (Development plan D-0)

**Verdict: CONFIRMED.** `desktop/src/shared/features/manifest.ts` validates
`preview-features.json` plus `protectedFeatureDefinitions` with a zod
schema; `useFeatureEnabled.ts` is the hook. D-0 adds an `org` feature id
to `preview-features.json` and a `route("/org", …)` entry in
`desktop/src/app/routes.ts` (TanStack virtual file routes).

## V15 — git store from ingest (CS-7)

**Verdict: REFUTED as designed.** There is no persistent git store to read.
A repository is hydrated into a tempdir from the object-store manifest per
request, through a bounded pack cache. Running `merge-base --is-ancestor`
inside `50102` ingest would hydrate the repo on every progress note.

What _is_ cheap and available at ingest: the relay-signed `30618` ref-state
event (`api/git/manifest_event.rs`), which lists every ref and its oid and
is replaced on each push. What is available at push time: the hook callback
(`api/git/policy.rs`) receives, per ref, `old_oid`, `new_oid`, and
`is_ancestor`, and the pusher pubkey.

**Consequence (D-12).** Protocol § 4.7 `50102` validation changes shape:

- At ingest the relay checks only that `ref` is present in the latest
  `30618` for `home.repo` and that `head` equals its oid (or is an ancestor
  claim the relay does not verify — it stores `head` as stated and marks the
  row `head_verified=false` if it does not match). Signer rule unchanged.
- `merged_into` is **not** a client field. The push hook, on an allowed
  fast-forward to `main` (or the repo's default branch), records
  `(repo, ref, new_oid)` and R-9's post-push job marks every `50102` whose
  `head` is in the pushed range as `merged_into = main` — a relay-signed
  `39101` refresh, not a new event kind. The hook already has `is_ancestor`;
  the range walk uses the manifest, not a working tree.
- Phase 0 does not need any of this until wave 6 (Work sync); R-9's
  repository half can land then without blocking the room-only home.

---

## Related

- [Current state](./intelligent-org-current-state.md) — the list this answers
- [Readiness review](../plans/intelligent-org-readiness.md) — D-11, D-12 and the gate
- [Development plan](../plans/intelligent-org-development-plan.md) — the slices cited above
- [Protocol](./intelligent-org-protocol.md) — § 4, § 6.5, § 6.7, § 6.8
