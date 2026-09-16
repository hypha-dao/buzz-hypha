---
title: 'The Intelligent Organization — Current State'
date: 2026-09-14
status: current
tags: [architecture, intelligent-org, ai, buzz]
---

# The Intelligent Organization — Current State

Snapshot of what **Buzz** has today that the intelligent organization needs, and what is
designed but not built. Read this to know where to start; read
[Design](./intelligent-org-design.md) for what to build and the
[Protocol](./intelligent-org-protocol.md) for exactly how. The earlier version of this file
(2026-08-29) was a snapshot of `hypha-web`; that platform is no longer the target and its state
is not tracked here.

---

## Clickable preview

[`prototypes/org-preview`](../../../prototypes/org-preview/README.md) — a standalone Next.js
app, deliberately outside the pnpm workspace — walks the five doors, the DMs and rooms, and the
Personal Assistant for two sample orgs, **River Commons** and **Hypha Energy**. It is the target
UI for the desktop `org` feature. It still calls the board door _Projects_; on Buzz it is
**Work**. Live copy:
[hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app).

---

## One sentence

Buzz has the substrate — a signed event log, channels, DMs, full-text search, managed agents
with their own keys, relay-side command execution, and a desktop with an Inbox — and none of
the intelligent-org objects: no work tree, no direction artifacts, no proposals, no org agent,
no doors. Its agent model is bring-your-own; the Hypha fork inverts that for the one agent
the org depends on — the org agent is hosted by default — and keeps it for members' own.

---

## What Buzz has

| Need                                    | What exists                                                                                                                                                                                                      | Where                                                                                       |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| **L1 substrate**                        | Every message (`kind:9`, `40002`), forum post, canvas, file, huddle lifecycle event, and DM stored in Postgres with signed author, channel, timestamp. Postgres FTS via NIP-50 `search` on `POST /query`. Community-scoped by relay host. | `buzz-relay`, `buzz-db`, `buzz-search`                                                  |
| **Transcripts**                         | Desktop huddles run on-device speech-to-text and post the text as `kind:9` into the huddle channel. Not tagged as transcript yet.                                                                                | `desktop/src-tauri/src/huddle/stt.rs`, `pipeline.rs`                                        |
| **Command kinds executed transactionally** | `is_command_kind` → `command_executor::handle_command`: validate → begin tx → insert event → mutate → commit. Used for DM open/add/hide, workflow defs and triggers, approval grant/deny.                       | `crates/buzz-core/src/kind.rs`, `crates/buzz-relay/src/handlers/command_executor.rs`        |
| **Relay-signed addressable state**      | NIP-29 group metadata/admins/members (`39000–39003`) emitted by the relay after admin commands; `is_relay_only_kind` rejects client writes.                                                                       | `handlers/side_effects.rs` (`emit_group_discovery_events`)                                  |
| **Typed projections beside events**     | Sidecar tables with `community_id` for reports, push leases, workflows, approvals; thread counters materialised on insert.                                                                                       | `migrations/`, `buzz-db/src/store/*`                                                        |
| **Roles**                               | NIP-43 relay membership with `owner` / `admin` / `member` roles; NIP-29 per-channel admins and roles.                                                                                                             | `handlers/relay_admin.rs`, `channel_authz.rs`                                               |
| **Private rooms**                       | Private channels, invite-only, relay-enforced membership.                                                                                                                                                        | NIP-29 handlers                                                                             |
| **DMs**                                 | DM channels opened by `kind:41010`; messages are ordinary `kind:9` in that channel — an agent reads and writes them like any room. Stored plaintext (NIP-17 gift wrap exists for interop but is not the product's DM path). Reads are gated by channel membership; a DM is identified by its participant set. | `command_executor.rs` (`handle_dm_open`)                                                    |
| **Agents as members**                   | Managed agents (NIP-AP `30177`) with their own key, owner, persona (`30175`), deployed from the desktop Agents view; ACP harness subscribes by mention, publishes via CLI; credentials injected as `BUZZ_RELAY_URL` / `BUZZ_PRIVATE_KEY` / `BUZZ_AUTH_TAG`. Built-in personas Fizz, Honey, Pollen seeded in `managed_agents/personas.rs`. `buzz-acp` has a **heartbeat** (`heartbeat_interval_secs`, `heartbeat_prompt`) that self-prompts on a timer, but the desktop's respond-to picker does not expose the heartbeat-only (`nobody`) mode. **In the Hypha fork the Agents door stays for members' own agents, seeds no sample personas, offers the Work sync template, and never lists the org agent** (Design § Where it runs, § Work sync). | `crates/buzz-acp/src/config.rs`, `crates/buzz-persona`, `desktop/src/features/agents/`, `desktop/src-tauri/src/managed_agents/personas.rs`, `managed_agents/types.rs` |
| **Git forge**                           | NIP-34 repos (`30617`) with Buzz's push ACL: the `buzz-channel` tag binds a repo to a channel and **channel role = repo role**; `buzz-protect` rules per ref pattern (`push:admin`, `push:member`, `no-force-push`, `no-delete`, `require-patch`) bind everyone including the owner; `maintainers` tag grants settings rights; a managed agent's NIP-OA owner inherits its repo authority (`is_agent_owner`). NIP-MP projects (`30621`) group repos and bind a channel. `buzz projects create --channel` mints a default repo, project, and binding in one go. Repo browser in `web/`. | `crates/buzz-core/src/git_perms.rs`, `crates/buzz-relay/src/api/git/{policy,binding,settings}.rs`, `crates/buzz-cli/src/commands/projects.rs` |
| **Agent memory**                        | NIP-AE engrams (`30174`), encrypted agent↔owner; `buzz mem` CLI.                                                                                                                                                  | `crates/buzz-core/src/engram.rs`, `crates/buzz-cli`                                         |
| **Model access**                        | `buzz-agent` provider config (Anthropic, OpenAI-compatible, OpenRouter, Databricks); Buzz Mesh as a local OpenAI-compatible endpoint.                                                                             | `crates/buzz-agent/src/config.rs`, `desktop/src-tauri/.../relay_mesh.rs`                    |
| **Schedules**                           | Relay-side due-time handling for reminders (NIP-ER `not_before`); workflow `schedule` triggers (cron / interval); admin worker pattern.                                                                           | `buzz-workflow`, `handlers/admin_action_worker.rs`                                          |
| **Items needing action**                | Relay-assembled Home feed with a `needs_action` bucket (approvals, reminders) surfaced in the desktop Inbox.                                                                                                      | `buzz-db/src/store/feed.rs`, `desktop/src/features/home/`                                   |
| **Desktop shell**                       | Tauri 2 + React 19; TanStack file routes; primary menu (Inbox, Pulse, Projects, Agents, Workflows); feature folders; Rust-side signing (`sign_event`); relay client with live REQ.                                | `desktop/src/app/routes.ts`, `features/sidebar/`, `shared/api/relayClientSession.ts`        |
| **Approval card pattern**               | Workflow approval card (pending, approver, expiry) — the visual pattern for decision cards. Visual only: its grant/deny is not wired in the desktop and the executor's `request_approval` is unfinished (WF-08). Our cards sign `io_*` commands directly; nothing here is reused but the look. | `desktop/src/features/workflows/.../WorkflowApprovalCard.tsx`                              |
| **CLI**                                 | Agent-first `buzz` CLI with typed subcommands, JSON output, exit codes; SDK builders per kind.                                                                                                                    | `crates/buzz-cli`, `crates/buzz-sdk/src/builders.rs`                                        |
| **Tests**                               | `buzz-test-client` E2E suite against a live relay; desktop Playwright with a mock bridge.                                                                                                                         | `crates/buzz-test-client/tests/`, `desktop/tests/e2e/`                                      |

---

## Designed, not built

| Layer / feature                                              | Status on Buzz                                                                                                                                        | Where it is specified                              |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| **L2 activity ledger**                                       | Not built. Commands `50001–50021`, ledger projection `io_ledger`.                                                                                      | Protocol §3.2, §6.2                                |
| **L3 direction** (mission, vision, objectives, strategy)     | Not built. `kind:39100` heads; `io_direction_propose` → `io_vote`. Overview today: nothing; the community description is a listing one-liner, not belief. | Protocol §4.1, §5.2                            |
| **L4 decision memory**                                       | Not built. `kind:39104` draft outcomes; `io_health_ratings`.                                                                                          | Protocol §4.6                                      |
| **Work tree** (projects / tickets, any depth) / DRI / offer–accept | Not built. `kind:39101`; commands `50004–50011`, `50018`; `io_scheduler` for offer expiry, review window, close on date.                          | Protocol §4.2, §5.1, §6.3                          |
| **Shapers**                                                  | Not built. `kind:39103` + relay-synced `#shapers` channel. Community `owner` role exists and seeds it.                                                | Protocol §4.5, §6.4                                |
| **Proposals** (project, dri, direction, shapers; money and join later) | Not built. `kind:39102`; rule per kind in `39103.rules`, `needed` fixed at opening, subject not eligible. Workflow approvals are a different object and are not reused. Money waits for the treasury contract (Protocol §5.6); payments stay in Hypha. | Protocol §4.4, §5.3, §5.6                          |
| **Org agent** (HEAR → THINK → ROUTE)                         | Not built. `crates/buzz-org-agent`; an ordinary member whose pubkey is `39103.agent`. **Hosted by Hypha by default**, one key and one instance per community, provisioned by the operator; replaceable by a `shapers/agent` proposal. `buzz-acp` is mention-driven and conversational — the relay client is reused, the pipeline is not. What the workspace offers the crate today: `buzz-ws-client` (connect, NIP-42, raw REQ, poll, publish — **no reconnect**), `buzz-agent`'s provider matrix and tool-calling `Llm` (**private module; no `temperature`, no `tool_choice`**), `buzz_core::engram` for NIP-44 v2 engrams, NIP-50 `search` over REQ. | [Org agent design](./intelligent-org-agent.md); Design § Where it runs; Protocol §4.5 |
| **Org agent in every conversation**                          | Not built. The relay adds `39103.agent` to every channel on create and to every DM's participant set on `41010`, backfills at `39103` bootstrap, and moves all of it on `shapers/agent`; the agent's pubkey is excluded from the **DM identity** (the `41010` dedupe key, the DM's `39000` `p` tags, the `41010` system message) and filtered from participant renderings in desktop and mobile; the **receipt read** — a member may `REQ {ids:[…]}` an event in a room they are not in iff a stored community-global `io` event cites it as a receipt. Today `handle_dm_open` keys the DM on exactly the `p`-tagged participants and the read gate has no receipt exception. | Protocol §6.8; Design § What the agent hears, and who sees it |
| **Hosted-agent provisioning**                                | Not built. `io_hosted_agents` (community → pubkey, written by the operator, read at `39103` bootstrap); `scripts/org-agent-provision.sh` (mint key, `kind:0`, NIP-43 add, registry row, launch); the supervisor that stops an instance when `agent_hosted` flips to false. | Design § Where it runs; Phase 0 § Deploy |
| **Agents door without sample personas**                      | Not built. Stop seeding `BUILT_IN_PERSONAS` (Fizz, Honey, Pollen) and the retired set; the door starts empty but for the **Work sync** template. DMs list the org agent first. | Design § Surfaces |
| **Project home**                                             | Not built. On a passed `project`: relay-created room, relay-signed `30617` (`buzz-channel`, `maintainers`, `buzz-protect main push:admin`) and `30621`, written to `39101.home`; room-roster sync on accept/release/`dri` (holder → admin, ticket holders → member, attested agents → bot); ledger `home_created`, `home_member_synced`. Buzz creates these only from a person's CLI/desktop today, never relay-side. | Protocol §6.7; Design § Work objects |
| **Progress notes**                                           | Not built. `kind:50102`; ingest check against the git store (ref exists, shas reachable) and the holder-or-attested-agent rule; `io_progress` projection; `last_progress` on `39101`; `stalled` health factor. | Protocol §4.7b, §6.1 |
| **Work sync agent**                                          | Not built. A `buzz-acp` persona ("Work sync") that runs on the heartbeat: `buzz org my-work` → match `branch` in `repos_dir` → `git log`/`diff --stat` → push → `buzz org progress note`. Needs the heartbeat-only respond mode exposed in the desktop's agent creation, and **Open in editor** on the ticket page (clone `home.repo`, create `branch`, open the editor). | Design § Work sync; Protocol §4.7b |
| **Drafts and cards**                                         | Not built. `kind:50100` + `39104`; card component set in `features/org/`.                                                                             | Protocol §4.3; Design § Surfaces                   |
| **Health reads**                                             | Not built. `kind:50101`.                                                                                                                              | Protocol §4.7                                      |
| **Org profile** — about, skills, open limit                  | Not built. `kind:39105` from `io_profile_set`; `io_profiles` with a GIN-indexed `skills` column; About & skills form in `features/profile`. Buzz's `kind:0` profile (name, picture, `about`) exists and stays the identity; it is not reused for skills because any client may overwrite it whole. | Protocol §4.7a, §5.4a                              |
| **Doors** — Overview / Work / Decisions / My Work / Profile  | Not built. `desktop/src/features/org/`, routes under `/org`. Walkable in `prototypes/org-preview`.                                                     | Design § Surfaces                                  |
| **Inbox sources**                                            | Not built. `needs_action` gains offers, drafts addressed to me, open proposals for Shapers.                                                            | Protocol §6.5                                      |
| **Transcript tag**                                           | Not built. Huddle STT must tag its `kind:9` output `["transcript", "huddle"]`.                                                                         | Protocol §5.5                                      |
| **Done-from-talk**                                           | Not built. The one agent-authored command, five relay checks.                                                                                          | Protocol §5.5                                      |
| **"Ask the org anything" with receipts**                     | Not built. L3 + ledger aggregates + NIP-50 search, citations as event ids.                                                                             | Design § Personal Assistant                        |
| **Money — treasury contract**                                | Next version, not first. Funds in a Shaper-controlled smart contract; a passed `money` proposal is the release; a relay-side treasury bridge records the chain receipt (`io_money_released`). Until then payments run through Hypha. | Protocol §5.6; Design § Money                      |
| **Membership by invite**                                     | Mostly built. Buzz has stateless invite codes (`POST /api/invites`, `/invite/<code>`, claim → NIP-43 member) minted by owner/admin only. Needed: allow pubkeys in `39103.shapers` to mint; ledger `member_joined`; agent DM greeting; the one-time **transparency notice** on the invite landing page (the agent is in every conversation, DMs included; what is said may be cited to any member). | `crates/buzz-relay/src/api/invites.rs`; Protocol §6.6; features §6a |
| **Join via proposal**                                        | Later, not first. `io_join_propose` → NIP-43 add member; the inbound request path is undecided.                                                         | Protocol §5.7                                      |
| **`buzz org` CLI**, SDK builders, kinds in `kinds.ts` / `nostr_models.dart` | Not built.                                                                                                                              | Design § Surfaces                                  |

---

## What to verify before building

> **Answered 15 Sep 2026** in
> [Codebase verification](./intelligent-org-codebase-verification.md) —
> two refuted (1: multi-letter tag filters; 7: git store at ingest), three
> partial (3, 6, 9), the rest confirmed. The list is kept as the record of
> what was asked.

Facts the design leans on that should be checked in code before step 1 of the build order:

1. **Multi-letter tag filters.** The doors filter on `#needs`, `#item`, `#parent`, `#status`.
   Confirm `buzz-db` indexes and matches arbitrary tag names in REQ filters, or plan the index
   migration.
2. **Global-only kinds with `d` tags.** `39100–39105` are addressable, relay-signed, and
   community-global. Confirm `replace_parameterized_event` handles relay-authored events with
   `channel_id = NULL` the way `39000` does.
3. **Org-agent lookup at ingest.** The relay must read `39103.agent` (or the `io_shapers`
   projection) inside the ingest path for `50100`, `50101`, and the §5.5 `io_done` check.
   Confirm the command executor can consult a projection before insert without a second
   transaction, and where the operator-written `io_hosted_agents` row is read when `39103`
   does not exist yet. Confirm NIP-43 membership can be granted to a pubkey by an operator
   process (as the owner would with `kind:9030`) before any member has opened the app.
4. **Huddle STT authorship.** Which key signs the `kind:9` messages the STT pipeline posts —
   the speaker's, or the device owner's? Either way they need the transcript tag; the answer
   decides how strongly the relay must distrust them.
5. **Invite mint authz.** `mint_invite` checks `role == owner || admin` from the NIP-43
   membership row. Confirm the cleanest seam to also accept a pubkey present in the
   `io_shapers` projection (a second lookup in the handler, or a relay-level `shaper` role
   mirrored onto the membership row — the former keeps org state out of NIP-43).
6. **Relay-signed `30617` / `30621`.** Buzz's forge assumes a person (or their agent) signs
   the announcement; the relay only enforces. Confirm nothing in `api/git/` — the
   `repo_owner` path segment, the settings authz, the CAS publish path — refuses an
   announcement whose author is the relay key, and that the bare repository is initialised
   for it the way it is for a CLI-created one (`ensure_default_create_repo` in the CLI is
   the reference). Confirm a channel created relay-side gets the same NIP-29 discovery
   events (`39000–39003`) a person-created one does.
7. **Git store from ingest.** `50102` validation reads refs and reachability from the git
   store inside the event pipeline. Confirm the store handle is reachable from
   `command_executor`/ingest without a cycle, and how expensive `merge-base --is-ancestor`
   is for the `merged_into` check on a large repository — a bounded check or a post-ingest
   fill of `merged_into` may be the right shape.
8. **Bot push through NIP-OA.** The push policy promotes a channel `Bot` to `Member` and
   grants the repo owner's attested agent the owner's authority. Confirm a ticket holder's
   agent, added to the room as a bot, can push `refs/heads/io/*` but is stopped on `main` by
   `push:admin`, and that the same attestation satisfies the `50102` signer rule.
9. **DM identity with a hidden member.** `handle_dm_open` derives the DM from the `p`-tagged
   participants; the desktop and mobile DM lists, the `40901` summary, and DM discovery in
   `e2e_nostr_interop.rs` all assume the participant set _is_ the membership. Find every
   place that computes a DM's identity or renders its participants and confirm the agent's
   pubkey can be held in membership yet excluded from all of them — otherwise every 1:1
   becomes a three-party group DM the day the agent is backfilled (Protocol §6.8).
10. **Receipt read against the membership gate.** The read gate (`buzz-auth`, the query
    handler's channel check) has no notion of "cited as a receipt". Confirm where a
    `{ids:[…]}` REQ is authorised and whether a lookup into `io_drafts` / `io_work_items` /
    `io_health` / `io_progress` for the cited id can sit there without a second round trip;
    the result must be one event, never a window into the room (Protocol §6.8).

---

## Related

- [The Intelligent Organization — What it is](../product/intelligent-org-features.md) — the target
- [The Intelligent Organization — Design](./intelligent-org-design.md) — the how, on Buzz
- [The Intelligent Organization — Protocol](./intelligent-org-protocol.md) — the kinds
- [Intelligent Org on Buzz — Phase 0](../plans/intelligent-org-phase-0.md) — the first slice to build
- [VISION.md](../../../VISION.md) — Buzz's own status table
