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
no doors.

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
| **DMs**                                 | DM channels opened by `kind:41010`; messages are ordinary `kind:9` in that channel — an agent reads and writes them like any room.                                                                                | `command_executor.rs` (`handle_dm_open`)                                                    |
| **Agents as members**                   | Managed agents (NIP-AP `30177`) with their own key, owner, persona (`30175`), deployed from the desktop Agents view; ACP harness subscribes by mention, publishes via CLI; credentials injected as `BUZZ_RELAY_URL` / `BUZZ_PRIVATE_KEY` / `BUZZ_AUTH_TAG`. | `crates/buzz-acp`, `crates/buzz-persona`, `desktop/src/features/agents/`             |
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
| **L2 activity ledger**                                       | Not built. Commands `50001–50018`, ledger projection `io_ledger`.                                                                                      | Protocol §3.2, §6.2                                |
| **L3 direction** (mission, vision, objectives, strategy)     | Not built. `kind:39100` heads; `io_direction_propose` → `io_vote`. Overview today: nothing; the community description is a listing one-liner, not belief. | Protocol §4.1, §5.2                            |
| **L4 decision memory**                                       | Not built. `kind:39104` draft outcomes; `io_health_ratings`.                                                                                          | Protocol §4.6                                      |
| **Work tree** (projects / tickets, any depth) / DRI / offer–accept | Not built. `kind:39101`; commands `50004–50011`, `50018`; `io_scheduler` for offer expiry, review window, close on date.                          | Protocol §4.2, §5.1, §6.3                          |
| **Shapers**                                                  | Not built. `kind:39103` + relay-synced `#shapers` channel. Community `owner` role exists and seeds it.                                                | Protocol §4.5, §6.4                                |
| **Proposals** (project, dri, money, direction, join)         | Not built. `kind:39102`; thresholds in `39103.rules`. Workflow approvals are a different object and are not reused.                                    | Protocol §4.4, §5.3                                |
| **Org agent** (HEAR → THINK → ROUTE)                         | Not built. `crates/buzz-org-agent`; deployed as a managed agent with a `role: org-agent` tag. `buzz-acp` is mention-driven and conversational — the identity and deployment path are reused, the pipeline is not. | Design § The org agent |
| **Drafts and cards**                                         | Not built. `kind:50100` + `39104`; card component set in `features/org/`.                                                                             | Protocol §4.3; Design § Surfaces                   |
| **Health reads**                                             | Not built. `kind:50101`.                                                                                                                              | Protocol §4.7                                      |
| **Doors** — Overview / Work / Decisions / My Work / Profile  | Not built. `desktop/src/features/org/`, routes under `/org`. Walkable in `prototypes/org-preview`.                                                     | Design § Surfaces                                  |
| **Inbox sources**                                            | Not built. `needs_action` gains offers, drafts addressed to me, open proposals for Shapers.                                                            | Protocol §6.5                                      |
| **Transcript tag**                                           | Not built. Huddle STT must tag its `kind:9` output `["transcript", "huddle"]`.                                                                         | Protocol §5.5                                      |
| **Done-from-talk**                                           | Not built. The one agent-authored command, five relay checks.                                                                                          | Protocol §5.5                                      |
| **"Ask the org anything" with receipts**                     | Not built. L3 + ledger aggregates + NIP-50 search, citations as event ids.                                                                             | Design § Personal Assistant                        |
| **Money settlement**                                         | Not built. `io_money_settle`; no treasury, settlement outside Buzz.                                                                                    | Protocol §3.2; Design § Money                      |
| **Join via proposal**                                        | Not built. `io_join_propose` → NIP-43 add member. Inbound join-request path depends on how a community accepts requests today.                          | Protocol §5.3                                      |
| **`buzz org` CLI**, SDK builders, kinds in `kinds.ts` / `nostr_models.dart` | Not built.                                                                                                                              | Design § Surfaces                                  |

---

## What to verify before building

Facts the design leans on that should be checked in code before step 1 of the build order:

1. **Multi-letter tag filters.** The doors filter on `#needs`, `#item`, `#parent`, `#status`.
   Confirm `buzz-db` indexes and matches arbitrary tag names in REQ filters, or plan the index
   migration.
2. **Global-only kinds with `d` tags.** `39100–39104` are addressable, relay-signed, and
   community-global. Confirm `replace_parameterized_event` handles relay-authored events with
   `channel_id = NULL` the way `39000` does.
3. **Managed-agent role tag.** `kind:30177` content is an allowlist projection; confirm a
   `["role", "org-agent"]` tag can be added without leaking anything and that the relay can
   read it at ingest time to gate Protocol §5.5.
4. **Huddle STT authorship.** Which key signs the `kind:9` messages the STT pipeline posts —
   the speaker's, or the device owner's? Either way they need the transcript tag; the answer
   decides how strongly the relay must distrust them.
5. **Community join requests.** How a non-member asks to join a community today (if at all)
   decides whether `io_join_propose` is opened by the relay or only by members on someone's
   behalf.

---

## Related

- [The Intelligent Organization — What it is](../product/intelligent-org-features.md) — the target
- [The Intelligent Organization — Design](./intelligent-org-design.md) — the how, on Buzz
- [The Intelligent Organization — Protocol](./intelligent-org-protocol.md) — the kinds
- [Intelligent Org on Buzz — Phase 0](../plans/intelligent-org-phase-0.md) — the first slice to build
- [VISION.md](../../../VISION.md) — Buzz's own status table
