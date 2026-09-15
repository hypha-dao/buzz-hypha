---
title: 'The Intelligent Organization — Protocol'
date: 2026-09-14
status: current
tags: [architecture, intelligent-org, protocol, nostr, buzz]
parent: docs/intelligent-org/README.md
---

# The Intelligent Organization — Protocol

The event kinds, tags, content schemas, and state machines that carry the
intelligent organization on a Buzz relay. This is the contract between the
relay, the desktop, the CLI, and the org agent. [Design](./intelligent-org-design.md)
explains why it is shaped this way; [What it is](../product/intelligent-org-features.md)
says what it is for. When this document is stable it becomes `docs/nips/NIP-IO.md`.

`draft` `optional` `relay`

**Depends on**: NIP-01 (events, addressable events), NIP-29 (channels, the
Shapers room), NIP-42 (auth), NIP-43 (community membership). Interacts with
NIP-AE (agent memory). The org agent is an ordinary member whose pubkey the
relay reads from `39103.agent` (§4.5); it needs no NIP-AP managed-agent event.

---

## 1. Shape

Three families of events, one pattern — the same one Buzz uses for channel
administration (`kind:9002` edit → `kind:39000` state):

| Family        | Range         | Signed by            | Stored as                             | Purpose                                                          |
| ------------- | ------------- | -------------------- | ------------------------------------- | ---------------------------------------------------------------- |
| **Commands**  | 50000–50049   | a person (or, for health ratings, a Shaper) | regular event, community-global | A human tap. Validated and executed transactionally by the relay. |
| **Drafts & reads** | 50100–50149 | the org agent (drafts may also be person-signed) | regular event, community-global | What the agent proposes or observes. Never changes state.       |
| **State**     | 39100–39149   | the relay            | addressable, `d` = object id          | The current truth about one object. Replaced on every change.    |

Rules that follow from the shape:

- **Every state change is a person's signed command.** The agent has no
  command it may send that mutates work, direction, or decisions — with one
  narrow, relay-verified exception: relaying a DRI's own signed "done"
  message (§5.5). The relay rejects a command whose author lacks the role
  the command needs.
- **Commands are the ledger.** Every accepted command is stored; its `id` is
  the receipt for the state it produced. Nothing is deleted.
- **State is a projection.** A client that only wants "what is true now"
  subscribes to `39100–39149`. A client that wants "why" follows the
  `receipt` tags back to commands and drafts.
- **All three families are community-global** (`channel_id = NULL`, no `h`
  tag). The community is the org; the relay host is the boundary. They are
  readable by every community member and by nobody else.
- **Drafts carry `needs`.** Only the pubkey (or role) a draft names may act
  on it, and the relay checks that when the referencing command arrives.

---

## 2. Terminology

- **item** — one node of the work tree: a project (no parent) or a ticket
  (has a parent). Identified by a UUID, the `d` tag of its `kind:39101`.
- **holder** / **DRI** — the pubkey in an item's `dri` field. _In progress
  means a holder._
- **Shaper** — a pubkey listed in `kind:39103`. The community owner is the
  first Shaper; every later change to the set is a `shapers` proposal.
- **proposal** — a decision waiting on the Shapers: one of `direction`,
  `project`, `dri`, `shapers` — and, in later versions, `money` and `join`.
  `d` tag of its `kind:39102`.
- **rule** — how many eligible Shapers must agree for a proposal of a given
  kind to pass: `majority` (default), `all`, or an integer N.
- **eligible** — the Shapers at the moment a proposal opened, minus its
  subject (`p … subject`). Nobody votes on themselves.
- **draft** — a `kind:50100` event: a suggestion the agent (or a person)
  made, addressed to the one party that can make it real.
- **receipt** — an event id (`e`), an address (`a`), or a direction line ref
  (`ref`) that justifies a claim. Receipts must resolve — in the community,
  not necessarily in a room the reader is a member of; whoever can see the
  event that cites a receipt may read the receipt (§6.8).
- **direction line ref** — `objectives@<version>#<n>` or
  `strategy@<version>#<n>`: one numbered line of one version of one
  direction artifact.

---

## 3. Kinds

### 3.1 State (relay-signed, addressable)

| Kind      | `d`                                            | Object                                    |
| --------- | ---------------------------------------------- | ----------------------------------------- |
| `39100`   | `mission` \| `vision` \| `objectives` \| `strategy` | Latest confirmed **direction** artifact |
| `39101`   | item UUID                                      | One **work item**                         |
| `39102`   | proposal UUID                                  | One **proposal** and its votes            |
| `39103`   | `shapers`                                      | The **Shaper set** and decision rules     |
| `39104`   | draft event id                                 | The **outcome** of one draft (L4)         |
| `39105`   | member pubkey                                  | One member's **org profile** — about, skills, limit |

All are `is_relay_only_kind`: a client `EVENT` of these kinds is rejected.
All are `is_global_only_kind`. Tags on each state event are chosen so that
the existing relay filters (`#p`, `#d`, `#t`) answer the board queries
without a new HTTP endpoint (see §6).

### 3.2 Commands (person-signed, executed)

| Kind    | Name                  | Who may send                                         | Effect                                                       |
| ------- | --------------------- | ---------------------------------------------------- | ------------------------------------------------------------ |
| `50001` | `io_shapers_propose`  | a Shaper; the community owner when `39103` is empty  | open a `shapers` proposal: `op` add / remove / rules / agent (39102) |
| `50002` | `io_direction_propose`| a Shaper                                             | open a `direction` proposal (39102)                          |
| `50003` | `io_vote`             | an eligible Shaper                                   | agree/decline on a proposal; on rule met, execute (§5.3)     |
| `50019` | `io_shaper_accept`    | the `p` of a passed `shapers/add` proposal           | join the Shaper set and `#shapers`; emit 39103               |
| `50020` | `io_shaper_step_down` | a Shaper, unless they are the last one               | leave the Shaper set and `#shapers`; emit 39103              |
| `50004` | `io_project_propose`  | any member                                           | open a `project` proposal (39102)                            |
| `50005` | `io_ticket_create`    | holder of `parent`                                   | create a child item, `open` or `offered`                     |
| `50006` | `io_offer`            | holder of the parent; a Shaper for a root            | item → `offered` to `p`                                      |
| `50007` | `io_accept`           | the `offered_to` pubkey                              | item → `accepted`; `dri` set                                 |
| `50008` | `io_decline`          | the `offered_to` pubkey                              | item → `open`; offer returned to `offered_by`                |
| `50009` | `io_done`             | the `dri`; or the org agent relaying the `dri`'s signed message (§5.5) | item → `done` unless an open child exists      |
| `50010` | `io_release`          | the `dri`                                            | item → `open`; children returned to the item's parent holder |
| `50018` | `io_reopen`           | the `dri` of a `done` item, within 7 days            | item → `accepted`; undoes a done (typically one relayed from talk) |
| `50011` | `io_set_due`          | a Shaper (root); holder of the parent (child)        | new `due_at`; on a root past its last fifth, cancels the scheduled close |
| `50012` | `io_draft_decide`     | the draft's `needs` party                            | record decline (or a standalone accept) of a draft (39104)   |
| `50013` | `io_money_propose`    | _reserved — next version_: the item's `dri`, or the holder of its parent, or a Shaper | open a `money` proposal; item must be `done`  |
| `50014` | `io_money_released`   | _reserved — next version_: the treasury bridge (the relay's contract watcher) | passed `money` proposal → `settled`, with the chain receipt |
| `50015` | `io_dri_propose`      | any member                                           | open a `dri` proposal naming `p` for an item with no holder  |
| `50016` | `io_join_propose`     | _reserved — later_: a member, or the relay from an inbound join request | open a `join` proposal for pubkey `p`              |
| `50017` | `io_health_rate`      | a Shaper                                             | record a blind band for an item and week (L4)                |
| `50021` | `io_profile_set`      | any member, for themselves only                      | replace their own org profile — about, skills, limit; emit 39105 |

`50013`/`50014` and `50016` are **not implemented in the first version**:
the relay rejects them with `restricted: money not enabled` /
`restricted: join not enabled`. Their shape is fixed here so the kind
numbers, the `money` and `join` proposal kinds, and the `settled` status
are reserved (§5.6, §5.7). Until then the org's payments run through Hypha,
and membership is by **invite link** (§6.6): any Shaper may mint one, and
claiming it makes the claimant a member with no proposal.

All commands need the `MessagesWrite` auth scope and community membership;
the role checks above are enforced in the command executor, after scope.
A command that fails its role check is rejected with
`restricted: <reason>`; a command whose target is in the wrong state is
rejected with `invalid: <reason>`. Rejections are not stored.

**Draft settlement rides on the command.** Any command may carry
`["e", <draft-id>, "", "draft"]`. The relay then, in the same transaction,
checks that the command's author is the draft's `needs` party, writes the
draft outcome (`accepted` if the command payload equals the draft payload,
`amended` otherwise) to `39104`, and executes the command. One tap, one
transaction. `io_draft_decide` exists for the taps that produce no other
command: decline, and _not yet_ on a done card.

### 3.3 Drafts and reads (agent- or person-signed, regular)

| Kind    | Name          | Signed by                     | What                                                              |
| ------- | ------------- | ----------------------------- | ----------------------------------------------------------------- |
| `50100` | `io_draft`    | the org agent, or a member    | a suggestion addressed to one party; see §4.3                     |
| `50101` | `io_health`   | the org agent                 | one project's health read for one week                            |
| `50102` | `io_progress` | an item's `dri`, or an agent NIP-OA-attested to the `dri` | a progress note on a held item, with commits as receipts; see §4.7b |
| `50103` | `io_agent_note` | the org agent                | what the agent did **not** publish, and its weekly tally; never a suggestion; see §4.7c |

A person-signed `50100` is how the desktop and CLI carry "publish from the
Personal Assistant" flows before the relay sees a command: the person's own
draft, addressed to themselves or to a Shaper.

A `50102` is what a member's own **Work sync** agent posts after reading the
member's checkout (Design § Members' own agents). It is a read, not a
command: it changes no state. It is the org agent's input for a `done`
draft and one factor in the health read.

A `50103` is the agent's record of its own silence: a draft its judge
dropped, a trigger it skipped, a budget it hit — and, weekly, the tally
of what became of its drafts. It exists because a dropped draft cannot be
a `50100` (its receipts may be exactly what failed) and the evaluation
plan counts drops as carefully as publishes. It is not addressed to
anyone and no client renders it as a card.

### 3.4 Registry additions

In `crates/buzz-core/src/kind.rs`, one new documented range
`// Intelligent organization (39100–39149 state, 50000–50149 commands and
drafts)`; every constant above added to `ALL_KINDS`; `39100–39105` added to
`is_relay_only_kind`; `50001–50021` added to `is_command_kind`; all three
ranges added to `is_global_only_kind`. `50100–50103` are the agent-facing
reads; `50103` is the only one with no receipt check (§4.7c). Mirror in
`desktop/src/shared/constants/kinds.ts` and
`mobile/lib/shared/relay/nostr_models.dart`.

---

## 4. Content and tags

Content is JSON. Fields not listed are ignored. Every timestamp is Unix
seconds as a JSON number. Pubkeys are 64-char lowercase hex. UUIDs are
lowercase RFC 4122.

### 4.1 `kind:39100` — direction artifact

Tags: `["d", "<slug>"]`, `["version", "<n>"]`, `["p", <confirmed_by>]`,
`["receipt", <proposal-id>]`.

```jsonc
{
  "slug": "objectives",
  "version": 3,
  "body": "Markdown. The statement and the paragraph or two behind it.",
  "lines": [                       // objectives and strategy only
    { "n": 1, "id": "l_7f3a", "text": "Weekday hall booked", "date": 1780000000 },
    { "n": 2, "id": "l_c81d", "text": "Stall running every Saturday", "date": 1777000000 }
  ],
  "confirmed_by": "<pubkey>",
  "confirmed_at": 1757800000,
  "proposed_by": "<pubkey>",
  "proposal": "<proposal-uuid>",   // the passed 39102, which holds the diff and the talk it came from
  "prev": "<event-id of the previous 39100 head>"   // absent on v1
}
```

`lines[].id` is stable across versions when the line is carried forward and
fresh when it is added; it is the `objective_ref` target. A `mission` or
`vision` artifact has no `lines`.

### 4.2 `kind:39101` — work item

Tags: `["d", <item-uuid>]`, `["state", <state>]`, `["root", <root-uuid>]`,
`["parent", <parent-uuid>]` (children only), `["p", <dri>]` (when held),
`["p", <offered_to>, "", "offered"]` (when offered), `["due", "<ts>"]`,
`["t", "project"]` or `["t", "ticket"]`, `["ref", <objective_ref>]` (roots
that serve an objective), `["receipt", <event-id>]` for the command that
produced this version.

```jsonc
{
  "id": "<uuid>",
  "parent": null,                  // uuid for a ticket
  "root": "<uuid>",                // == id for a project
  "depth": 0,
  "path": [],                      // ancestor uuids, root first
  "title": "Weekday hall",
  "brief": "Markdown. What this is for and what done looks like.",
  "state": "accepted",             // see §5.1
  "dri": "<pubkey>",               // required when state ∈ {accepted, in_review}
  "offered_to": null,
  "offered_by": null,              // pubkey, or "agent"
  "offered_at": null,
  "due_at": 1785000000,            // end date (root) / estimated completion (child)
  "approved_at": 1757900000,       // when the root went live (roots only)
  "objective_ref": "objectives@3#l_7f3a",
  "created_from": "<event-id>",    // the command that created it; the proposal for a root
  "draft": "<event-id>",           // the 50100 it was promoted from, if any
  "done_receipt": "<event-id>",    // the io_done command, or the chat message it cited
  "closed_by": null,               // "dri" | "rule" | "release"
  "children": { "open": 1, "offered": 0, "accepted": 2, "done": 3 },
  "home": {                        // roots only; written when the proposal passes (§6.7)
    "channel": "<channel-uuid>",   // the project's NIP-29 room
    "repo": "30617:<relay-pubkey>:weekday-hall",    // NIP-34 announcement, relay-signed
    "project": "30621:<relay-pubkey>:weekday-hall"  // NIP-MP project wrapping the repo
  },
  "branch": "io/7f3a-weekday-hall",  // children only; the work branch the desktop opens for the holder
  "after": ["<sibling-uuid>"],     // children only, optional; siblings this piece sensibly follows (advisory)
  "last_progress": "<event-id>"    // newest 50102 for this item, if any
}
```

`after` is **order, not a lock**. It says which sibling(s) this piece
sensibly follows — a permit before the build, a pilot before the rollout —
so the Work board can show the sequence and the agent can draft the next
wave when the earlier one finishes. The relay checks only that every id is
a live or done sibling under the same parent (else
`invalid: after not a sibling`); it never blocks an offer, accept, or done
on it. The holder decides the real order.

A child's `home` is its root's; clients read it from the root. `branch` is
a convention, not a rule: `io/<first 4 of the item uuid>-<slug>`. It is what
**Open in editor** on the ticket page checks out, and what the holder's Work
sync agent looks for (§4.7b). Pushing to any other branch is allowed; it
just is not matched to the ticket.

### 4.3 `kind:50100` — draft

Tags (all required unless noted):

| Tag                              | Meaning                                                                                   |
| -------------------------------- | ----------------------------------------------------------------------------------------- |
| `["needs", "shaper"]` or `["needs", <pubkey>]` | the one party that may act; the relay checks it on the referencing command   |
| `["kind", <draft-kind>]`         | `project` \| `dri` \| `ticket` \| `done` \| `review` \| `objectives` \| `direction` \| `profile` \| `money` |
| `["move", "1".."4"]`             | which agent move produced it (see the AI evaluation plan)                                 |
| `["origin", "talk" \| "gap"]`    | heard in a room, or drafted from the gap between direction and the tree                   |
| `["gap", <gap-key>]`             | dedupe key: an objective line ref, an item uuid, or `<item-uuid>#<covers-slug>`           |
| `["parent", <item-uuid>]`        | for `ticket` and `done` drafts                                                            |
| `["item", <item-uuid>]`          | for `dri`, `review`, `money` drafts                                                       |
| `["p", <pubkey>, "", "needs"]`   | required when `needs` is a pubkey: the same party, so the relay's mention index puts the draft in their inbox (Codebase verification V10). `needs = shaper` drafts carry none; the inbox resolves Shapers from `39103`. |
| `["p", <pubkey>, "", "suggested"]` | suggested holder, when named (optional)                                                 |
| `["e", <event-id>, "", "receipt"]`, `["a", <coord>, "", "receipt"]`, `["ref", <line-ref>]` | receipts; at least one required |
| `["a", "39105:<relay>:<pubkey>", "", "receipt"]`, `["skill", <slug>]` | when a holder is suggested: the profile matched and the skill line(s) it matched on |
| `["shadow", "true"]`             | optional: shown to nobody; recorded for evaluation only                                   |
| `["expiration", "<ts>"]`         | optional NIP-40 cleanup for drafts nobody decides                                         |
| `["prompt", "<job>@<version>"]`, `["model", "<id>"]`, `["trace", "<id>"]` | optional, also on `50101`: the prompt version and model that produced it, and the agent's job id for following a card back to its logs (Org agent § 8.4, § 16). Ignored by clients; read by the tally. |

Content is the structured payload for that draft kind. The schemas are the
zod/serde types in the agent crate (`crates/buzz-org-agent/src/schemas.rs`);
their shape is fixed here:

```jsonc
// kind = project  (also the follow-up in a review)
{ "title": "", "brief": "", "objective_ref": "objectives@3#l_7f3a", "due_at": 0,
  "suggested_dri": "<pubkey>|null", "why": "one line",
  "gaps": [ { "ref": "objectives@3#l_7f3a", "served": "not|partly|served", "by": ["<item-uuid>"] } ] }

// kind = dri
{ "item": "<uuid>", "suggested": "<pubkey>", "why": "",
  "matched": { "skills": ["grant-writing"], "about": "<quoted span>|null", "items": ["<item-uuid>"] },
  "evidence": ["<event-id>"] }
// `matched` is the fit as receipts: skills and an about span from the person's 39105,
// items they held before. `why` may add nothing that is not in `matched`.
// A project/ticket draft's suggested_dri / suggested_holder carries the same `matched` object.

// kind = ticket
{ "parent": "<uuid>", "title": "", "brief": "", "due_at": 0,
  "requires": ["<skill-slug or one-line capability>"],   // what this piece needs from whoever holds it
  "suggested_holder": "<pubkey>|null",
  "unfilled": "one line — why nobody here fits|null",    // required when requires is non-empty and suggested_holder is null
  "covers": "the phrase in the parent brief this answers to",
  "after": ["<sibling-uuid>"],                            // live or done siblings this piece follows; [] when it can start now
  "gate": false,                                          // true when its outcome decides what the later pieces are
  "coverage": [ { "piece": "", "covered_by": "<item-uuid>|null",
                  "order": 1, "after": ["<piece>"], "held": "after <piece>|null" } ] }
// `coverage` is the whole ordered plan for the parent brief; a piece is `held` when a predecessor
// is neither live nor done, and a held piece is never a draft in this batch. `after` on the draft
// carries into the item's `after` on promotion (§4.2). `requires` names what the piece needs;
// `matched` on the holder is the evidence it is met — `requires` without a matching `matched` skill
// is a stretch and says so in `why`.

// kind = done   (the done card when the last child closed, or a transcript nudge)
{ "item": "<uuid>", "why": "last child closed | heard on a call", "heard": "<event-id>|null" }

// kind = review
{ "item": "<uuid>",
  "brief": [ { "text": "", "rows": ["<event-id>"] } ],
  "recommendation": { "type": "follow_up", "project": { /* kind=project payload */ } }
  /* or */ // "recommendation": { "type": "no_further_work", "why": "" }
}

// kind = objectives   (a redraw as operations, never a rewrite)
{ "base_version": 3,
  "ops": [ { "op": "strike", "id": "l_c81d", "why": "" },
           { "op": "move", "id": "l_7f3a", "date": 0, "why": "" },
           { "op": "add", "text": "", "date": 0, "why": "", "source": "<event-id>" } ] }

// kind = direction   (mission, vision, or a strategy line change, drafted from talk)
{ "slug": "strategy", "base_version": 4, "body": "", "lines": [ /* full new list */ ],
  "diff": "unified or line-op summary", "heard": ["<event-id>"] }

// kind = profile   (the agent proposing a member's own About & skills from their DM;
//                   needs = that member; settles through io_profile_set)
{ "pubkey": "<pubkey>", "about": "", "skills": ["hosting-events", "spanish"],
  "open_limit": 3 | null, "heard": ["<event-id>"] }

// kind = money
{ "item": "<uuid>", "payee": "<pubkey>", "amount": "150", "currency": "USDC",
  "agreed": { "amount": "150", "heard": "<event-id>" } | null }
```

### 4.4 `kind:39102` — proposal

Tags: `["d", <uuid>]`, `["t", <kind>]`, `["status", <status>]`,
`["p", <opened_by>]`, `["p", <subject>, "", "subject"]` for `dri`, `join`,
`money`, and `shapers` add/remove (the named person / payee / seat),
`["p", <eligible>, "", "eligible"]` per eligible Shaper (so My Work can
filter by `#p`), `["item", <uuid>]` when about an item,
`["receipt", <opening-command-id>]`.

```jsonc
{
  "id": "<uuid>",
  "kind": "money",                 // direction | project | dri | money | join | shapers
  "status": "open",                // open | passed | rejected | expired | settled (money only)
  "opened_by": "<pubkey>",
  "opened_at": 0,
  "expires_at": 0,                 // opened_at + 39103.decision_window_secs
  "draft": "<event-id>|null",
  "payload": { /* the command content that opened it, verbatim */ },
  "rule": "majority",              // the 39103 rule that applied when it opened; "all" for shapers/rules and shapers/agent
  "needed": 2,                     // the rule resolved against eligible, at opening
  "eligible": ["<pubkey>"],        // Shapers at opening minus the subject; a later Shaper change does not move the bar
  "votes": [ { "p": "<pubkey>", "vote": "agree", "at": 0, "receipt": "<event-id>" } ],
  "decided_at": null,
  "executed": null,                // { "kind": "work_item", "id": "<uuid>" } etc. once passed
  "settlement": null               // money, next version: { "tx": "", "chain": "", "contract": "", "at": 0, "receipt": "<50014-id>", "error": null }
}
```

### 4.5 `kind:39103` — Shapers and rules

Tags: `["d", "shapers"]`, one `["p", <pubkey>]` per Shaper.

```jsonc
{
  "founder": "<pubkey>",           // the community owner at the time 39103 was first written
  "shapers": ["<pubkey>"],
  "offered": [ { "p": "<pubkey>", "proposal": "<uuid>", "at": 0 } ],  // passed adds not yet accepted
  "room": "<channel-uuid>",        // the private #shapers channel the relay keeps in sync
  "agent": "<pubkey>",             // the community's org agent; the relay's hosted default until a shapers/agent proposal passes
  "agent_hosted": true,            // true while `agent` is the relay operator's hosted agent
  "rules": {
    "direction": "majority",
    "project":   "majority",
    "dri":       "majority",
    "shapers":   "majority",       // add and remove; a rules change is always "all"
    "money":     "majority",       // later; ignored while money is not enabled
    "join":      "majority"        // later; ignored while join is not enabled
  },
  "decision_window_secs": 604800,  // 7 days: an open proposal that has not passed expires
  "offer_window_secs": 259200,     // 3 days: unanswered offers renotify once, then return
  "updated_at": 0,
  "receipt": "<event-id>"
}
```

`rules` values: `majority` (more than half of `eligible`), `all` (every
eligible Shaper), or an integer `N ≥ 1` (at least N of `eligible`; capped at
`eligible.len()` when resolved). `majority` is the default for every kind.
The relay resolves the rule to `needed` when the proposal opens and stores
it; a Shaper change afterwards does not move the bar. With one eligible
Shaper every rule resolves to 1.

A change to `rules`, `decision_window_secs`, or `offer_window_secs` is a
`shapers` proposal with `op=rules`, and it **always** passes under `all`,
whatever `rules.shapers` says — nobody's vote changes weight without their
agree. The new rules apply to proposals opened after it passes; open ones
finish under their stored `needed`.

`agent` is the one pubkey the relay treats as this community's org agent:
the only author it accepts for `50101` and `50103` (and, alongside members,
`50100`), the only pubkey the §5.5
exception applies to, and a permanent member of **every channel and every
DM** in the community, `#shapers` included (§6.8). At bootstrap
(§6.4) the relay fills it with the operator's **hosted default** for the
community from `io_hosted_agents` and sets `agent_hosted=true`; no member
configures anything. A relay with no row for the community (a sovereign
relay that hosts nothing) leaves `agent` empty until the Shapers name one. A
`shapers` proposal with `op=agent` and a `["p", <pubkey>]` tag replaces it
with an agent the community runs itself; the same op with no `p` returns to
the hosted default. Like `op=rules` it **always** passes under `all` — who
drafts for the org is something every Shaper agreed to. The named pubkey
must already be a NIP-43 member of the community and must not be a Shaper;
the relay rejects otherwise (`invalid: agent not a member`,
`invalid: agent is a shaper`). Execution is in §5.3.

The Shaper set only changes through: a passed `shapers/add` followed by the
named person's `io_shaper_accept`; a passed `shapers/remove`; or
`io_shaper_step_down`. A remove or step-down that would leave `shapers`
empty is rejected with `invalid: last shaper`. Being a Shaper and holding
work are independent: removal does not touch any item's `dri`.

### 4.6 `kind:39104` — draft outcome

Tags: `["d", <draft-event-id>]`, `["status", <status>]`, `["p", <decided_by>]`.

```jsonc
{ "draft": "<event-id>", "status": "open|accepted|amended|declined|expired|shadow",
  "decided_by": "<pubkey>|null", "decided_at": 0,
  "reason": "already_covered|not_what_the_line_meant|too_big|too_small|wrong_holder|not_now|other|null",
  "result": "<event-id>|null" }   // the command or state the draft turned into
```

The relay writes `open` when a `50100` without `shadow` is stored, so every
draft has exactly one outcome coordinate from birth.

### 4.7 `kind:50101` — health read

Tags: `["item", <uuid>]`, `["week", "2026-W38"]`, `["band", "struggling|wobbly|healthy"]`.

```jsonc
{ "item": "<uuid>", "week": "2026-W38", "pct": 0.62, "band": "wobbly",
  "factors": [ { "name": "overdue", "value": 2, "weight": 0.25, "rows": ["<event-id>"] },
               { "name": "stalled", "value": 1, "weight": 0.15, "rows": ["<event-id>"] } ],
  "sentences": [ { "text": "Two pieces are past their date.", "rows": ["<event-id>"] } ],
  "formula": "health-weights@1" }
```

Factor names are fixed by the formula version. `health-weights@1`:
`done_vs_elapsed`, `overdue`, `unanswered_offers`, `silent_weeks`,
`unheld`, `objective_moved`, and `stalled` — held children with no `50102`
progress note and no `io_*` command for 14 days. The `rows` of `stalled`
are the items' `39101` ids; the sentence names the pieces. A project whose
holders run no Work sync agent has no `50102` rows at all; `stalled` then
falls back to command silence alone, and the paragraph says so ("no progress
notes are being posted for this project").

### 4.7b `kind:50102` — progress note

Tags: `["item", <uuid>]`, `["p", <dri>]`, `["ref", "refs/heads/io/7f3a-weekday-hall"]`,
`["commit", <sha>]` (one per commit the note covers, newest first, at most
50), `["hint", "progressing|blocked|ready"]`, `["e", <previous 50102>, "", "prev"]`?
(the note this one continues from).

```jsonc
{
  "item": "<uuid>",
  "dri": "<pubkey>",                 // the holder; equals the signer, or the signer's NIP-OA owner
  "from": 1757800000, "to": 1757886400,   // the window the note covers
  "summary": "Markdown, ≤ 1200 chars. What moved, in the holder's terms.",
  "hint": "progressing",             // the agent's read of where the work stands; never a state
  "ref": "refs/heads/io/7f3a-weekday-hall",
  "head": "<sha>",                   // the ref's tip when the note was written
  "commits": [ { "sha": "<sha>", "title": "Add booking form" } ],
  "files_changed": 7,
  "uncommitted": { "files": 2 },     // seen in the working tree, not pushed; informative only
  "merged_into": null                // "refs/heads/main" once the ref's head is reachable from it
}
```

Relay checks at ingest (§6.1): the item exists and is `accepted` or
`in_review`; `dri` is the item's holder; the signer is `dri` **or** an agent
whose NIP-OA owner is `dri` (`is_agent_owner`, the same check the git push
policy runs); `ref` exists in the root's `home.repo` in this relay's git
store and every `commit` sha is reachable from it. A note whose receipts do
not resolve is rejected `invalid: unresolved receipt` — same gate as a
draft. `summary` is free text; `commits`, `ref`, `head`, and `merged_into`
are checked against the repository, which is why they are the receipts and
the summary is not.

`hint` is what the agent thinks, written down so the org agent and the
health read can use it. `ready` does not close anything: it makes the item a
candidate for a `done` draft addressed to the holder (Design § Triggers),
and the holder's tap on that card is the `io_done`. `blocked` is a candidate
for a nudge to the holder of the parent.

### 4.7c `kind:50103` — agent note

Tags: `["note", "draft_dropped|trigger_skipped|budget_exhausted|tally"]`,
`["move", "1".."4"]`? (which move it concerns), `["gap", <gap-key>]`? (for
`draft_dropped`), `["item", <uuid>]`? (when about an item),
`["week", "2026-W38"]` (for `tally`), `["trace", <id>]`? (the agent's own
job id, for following a note back to its logs).

```jsonc
// note = draft_dropped   the judge refused a draft before publish
{ "note": "draft_dropped", "move": 2, "gap": "<item-uuid>#rota",
  "reason": "unmatched_skill",         // the judge's reason code (Org agent design § 9.1)
  "kind": "ticket",                    // the draft kind it would have been
  "needs": "<pubkey>|shaper",
  "trace": "<id>" }

// note = trigger_skipped  a trigger ran out of retries, or was coalesced away under load
{ "note": "trigger_skipped", "move": 1, "trigger": "direction_confirmed",
  "object": "objectives@4", "reason": "model_timeout|queue_full", "trace": "<id>" }

// note = budget_exhausted  the agent hit a cap and went to shadow
{ "note": "budget_exhausted", "budget": "calls_per_hour|tokens_per_day",
  "until": 1757890000 }

// note = tally   Friday, rolling four weeks — the online columns of the evaluation plan
{ "note": "tally", "week": "2026-W38", "window_weeks": 4,
  "moves": { "1": { "opened": 6, "accepted": 3, "amended": 1, "declined": { "already_covered": 1, "not_now": 1 },
                    "dropped": { "unresolved_receipt": 0, "nag": 0, "duplicate": 0 }, "shadow": 0 },
             "2": { /* same shape */ }, "3": { /* same */ } },
  "health": { "reads": 4, "rated": 8, "agreed": 7 },
  "open_older_than_5d": 0,
  "receipt_rejected": 0 }              // 50100s the relay refused for an unresolved receipt
```

Relay checks at ingest (§6.1): the author is `39103.agent`; `note` is one
of the four; a `gap` or `item`, if present, is well-formed (it need not
resolve — the whole point of `draft_dropped` is that something did not).
Stored as a regular community-global event; the relay writes the ledger
row (`draft_dropped`, `trigger_skipped`, `budget_exhausted`, `tally`)
with `actor = <agent>`. Community-readable like every other read; the
tally is what Overview's Shapers-only tally card renders, and
`buzz org tally` reads the same event.

### 4.7a `kind:39105` — org profile

One per member per community, `d = <pubkey>`. Written only from that
member's own `io_profile_set`; the relay signs it like the other state
kinds. Community-readable, like the rest of the org.

Tags: `["d", <pubkey>]`, `["p", <pubkey>]`, one `["skill", <slug>]` per
skill (so `{kinds:[39105], "#skill":["rust"]}` answers "who can do X"),
`["version", "<n>"]`.

```jsonc
{ "pubkey": "<pubkey>", "version": 2,
  "about": "I run the Tuesday kitchen and can write a grant if someone checks my numbers.",
  "skills": [ { "slug": "grant-writing", "label": "grant writing" },
              { "slug": "hosting-events", "label": "hosting events" } ],
  "open_limit": 3,               // self-set: how many open pieces at once; null = no limit
  "updated_at": 0,
  "receipt": "<io_profile_set event id>" }
```

Limits (rejected with `invalid:` beyond them): `about` ≤ 1 000 chars;
≤ 20 skills; each label ≤ 40 chars; `slug` is the relay's kebab-case of
the label, deduplicated; `open_limit` 1–50 or null. The whole object is
replaced on every `io_profile_set` — there is no partial update, which is
what lets a DM-drafted `profile` card settle by ordinary draft-on-command
(§3.2). Free text only; the relay does no matching. Skills are the person's
words, not a taxonomy; the agent normalises for matching on its side and
cites the slug it matched, never a category it inferred.

This is separate from `kind:0`. `kind:0` is the Nostr identity — name,
picture, a bio any client may overwrite wholesale — and it is one per key.
The org profile is per community, structured, and only ever written by the
org's own command path, so a skill list survives a profile edit made from
another Nostr client and can be a receipt.

### 4.8 Command contents

Commands are small. Tags name the target; content carries the rest.

| Kind    | Tags                                                                 | Content                                                        |
| ------- | -------------------------------------------------------------------- | -------------------------------------------------------------- |
| `50001` | `["op", "add"\|"remove"\|"rules"\|"agent"]`, `["p", <pubkey>]` for add/remove, and for agent when naming a self-run agent (absent = back to hosted) | `{ why?: "" }` for add/remove/agent; `{ rules, decision_window_secs?, offer_window_secs? }` for rules |
| `50002` | `["d", <slug>]`, `["base", "<version>"]`, `["e", <draft>, "", "draft"]`? | `{ body, lines?, why? }` — the whole new version               |
| `50003` | `["e", <proposal-uuid>]`, `["vote", "agree"\|"decline"]`             | `{ reason?: "" }` (a decline reason of direction may trigger a strategy draft) |
| `50019` | `["e", <proposal-uuid>]`                                             | `{}` — the seat the passed `shapers/add` offered                |
| `50020` | —                                                                    | `{ why?: "" }`                                                 |
| `50004` | `["e", <draft>, "", "draft"]`?                                       | `{ title, brief, due_at, objective_ref?, suggested_dri? }`     |
| `50005` | `["parent", <uuid>]`, `["p", <offer_to>]`?, `["e", <draft>, "", "draft"]`? | `{ title, brief, due_at, after?: ["<sibling-uuid>"] }`   |
| `50006` | `["item", <uuid>]`, `["p", <pubkey>]`, `["e", <draft>, "", "draft"]`? | `{}`                                                          |
| `50007` | `["item", <uuid>]`                                                   | `{}`                                                           |
| `50008` | `["item", <uuid>]`                                                   | `{}` — no reason is recorded; decline is private to the person |
| `50009` | `["item", <uuid>]`, `["e", <message-id>, "", "receipt"]`?, `["e", <draft>, "", "draft"]`? | `{}`                                     |
| `50010` | `["item", <uuid>]`                                                   | `{ why?: "" }`                                                 |
| `50011` | `["item", <uuid>]`, `["due", "<ts>"]`                                | `{ why?: "" }`                                                 |
| `50012` | `["e", <draft>]`, `["outcome", "accept"\|"decline"]`, `["reason", <reason>]`? | `{}`                                                  |
| `50013` | `["item", <uuid>]`, `["p", <payee>]`, `["e", <draft>, "", "draft"]`? | `{ amount, currency, note?, agreed?: { amount, heard } }` — reserved |
| `50014` | `["e", <proposal-uuid>]`, `["tx", <chain-tx-id>]`                    | `{ chain, contract, amount, currency }` — reserved; signed by the treasury bridge key |
| `50015` | `["item", <uuid>]`, `["p", <pubkey>]`, `["e", <draft>, "", "draft"]`? | `{ why?: "" }`                                                |
| `50016` | `["p", <pubkey>]`                                                    | `{ note?: "" }`                                                |
| `50017` | `["item", <uuid>]`, `["week", "<iso-week>"]`, `["band", <band>]`    | `{}`                                                           |
| `50018` | `["item", <uuid>]`                                                   | `{ why?: "" }`                                                 |
| `50021` | `["e", <draft>, "", "draft"]`?                                       | `{ about, skills: [""], open_limit?: n }` — the whole profile; `pubkey` is the signer, never a tag |

---

## 5. State machines

### 5.1 Work item

```
              io_ticket_create (no p)      io_offer
  (child)  ───────────────────────► open ──────────► offered
                                     ▲                 │  io_accept
  (root)   proposal passes ──────────┤                 ▼
                                     │             accepted ──── io_done ────► done
                        io_decline / │                 │                      ▲
                        offer expires│   scheduled     │ io_release           │
                        (rule)       └─────────────────┘                      │
                                                       │                      │
                                                 in_review ── due_at passes (rule) ─┘
                                            (root enters last fifth; rule)     closed_by = "rule"
```

- `open` — real, nobody holds it. A root is `open` from the moment its
  proposal passes.
- `offered` — named person has not answered. `offered_to`, `offered_by`,
  `offered_at` set. The relay renotifies once at half the window and returns
  the item to `open` when the window passes (ledger verb `offer_expired`).
- `accepted` — held. `dri` set. **In progress means a holder.**
- `in_review` — a root in the last fifth of its run
  (`now ≥ due_at − 0.2 × (due_at − approved_at)`, floor two days). Set by
  the relay's date rule; still held. Entering it is the trigger for the
  review draft.
- `done` — closed. `closed_by` says how: `dri` (an `io_done`), `rule` (a
  root that reached `due_at`), `release` is not a close.

Invariants enforced in the executor, not the UI:

1. **One promotion rule.** A root is created only by a passed `project`
   proposal. A child is created only by `io_ticket_create` from the holder
   of its parent.
2. **Only the named person accepts or declines.**
3. **Completion cascades up, never down.** `io_done` on an item with a child
   in `open`, `offered`, `accepted`, or `in_review` is rejected with
   `invalid: open children`. Marking a parent done does not touch children.
4. **Release returns children.** `io_release` on an item with children moves
   each open child's `offered_by`/authority to the released item's parent
   holder (or to the Shapers for a root) and records that in the ledger.
5. **Roots close on `due_at`.** The date rule moves `accepted`/`in_review`
   roots to `done` when `due_at` passes, unless an `io_set_due` moved the
   date. It never closes children; open children under a rule-closed root
   are returned to the Shapers (ledger `orphaned_by_close`) and show as
   `open` under a closed root until re-parented by a Shaper-approved project.
6. **No money field.** The executor rejects any item content with `amount`,
   `budget`, `pot`, or `currency`.
7. **Every accepted command stores one event and emits one `39101`.** The
   command insert, the projection update, and the state emission are one
   transaction. There is no path that changes a projection row without a
   stored command or a ledger row for a rule.

### 5.2 Direction artifact

```
 io_direction_propose ──► proposal(direction, open) ──► io_vote reaches rule ──► 39100 version n+1
                                                    └─► declined ──► proposal rejected (39102), no 39100 change
```

Direction is never edited in place: a new version is a new proposal, and
`base` must equal the current head version or the command is rejected with
`invalid: stale base` (NIP-33 last-write-wins is not the conflict rule here;
the base check is).

### 5.3 Proposal

```
 open ──► passed ──► (money only) settled
   ├───► rejected
   └───► expired
```

Votes: `io_vote` is accepted only from a pubkey in `eligible`, once per
proposal (a second vote from the same Shaper replaces the first while the
proposal is `open`; ledger `vote_changed`). The relay evaluates after every
vote: `agrees ≥ needed` → `passed`; `declines > eligible.len() − needed` →
`rejected` (it can no longer pass). The proposer does not get an automatic
agree; opening and agreeing are two taps, so a proposer can open something
for the others to weigh without pre-committing. `expired` is a scheduler
transition at `expires_at` (§6.3) and notifies `opened_by`; an expired
proposal may be reopened as a new one.

On `passed`, the relay executes in the same transaction:

| Kind        | Execution                                                                                   |
| ----------- | ------------------------------------------------------------------------------------------- |
| `direction` | emit `39100` version `base + 1`; write `prev`                                               |
| `project`   | create the root item in `open` (or `offered` to `suggested_dri` if the payload named one); create its **home** — room, repository, NIP-MP project — in the same transaction and write `home` on the `39101` (§6.7) |
| `dri`       | set `dri` on the item, state `accepted`; if it was `offered` to someone else, that offer is withdrawn |
| `money`     | _next version_: the relay's treasury bridge submits the release to the contract; `settled` when `io_money_released` carries the chain receipt (§5.6) |
| `join`      | _later_: NIP-43 add-member for `p`, as if the owner had sent `kind:9030` (§5.7)              |
| `shapers` add | append `p` to `39103.offered`; the seat is live only on `p`'s `io_shaper_accept` (§6.4); an unaccepted seat lapses after `offer_window_secs` |
| `shapers` remove | drop `p` from `39103.shapers` and from `#shapers`; any open proposal keeps its stored `eligible` |
| `shapers` rules | replace `39103.rules` / windows; ledger `rules_changed`                                  |
| `shapers` agent | set `39103.agent` to the payload's `p` (or back to the relay's hosted default when `p` is absent) and `agent_hosted` accordingly; move membership in **every** channel and DM from the previous agent to the new one (§6.8; ledger `agent_membership_synced` with the count); ledger `agent_changed`. Open `50100` drafts keep their author; the relay reads `39103.agent` at ingest, so the old key can no longer publish drafts or `io_done`. |

A rejected `direction` proposal whose decline carried a reason is a
candidate for the agent (a strategy draft) — but the proposal itself
changes no direction.

### 5.4 Draft outcome

`open → accepted | amended | declined | expired`, or `shadow` from birth.
Written only by the relay; `accepted`/`amended` from a referencing command,
`declined` from `io_draft_decide`, `expired` from the `expiration` tag
passing.

### 5.4a Org profile

No states — one object per member, replaced whole by each `io_profile_set`
(§4.7a), `version` incremented, `profile_set` in the ledger with
`actor = <pubkey>`. Invariants the relay enforces:

- Only the signer's own profile: a `50021` cannot name another pubkey.
- Replaced whole; the limits in §4.7a; an empty profile is allowed (a
  member may clear it).
- A `profile` draft (§4.3) may only be addressed to the pubkey it
  describes (`needs = pubkey`), and settles only through that person's
  `50021` carrying the `draft` tag — nobody, including the agent, writes a
  profile for someone else.
- A draft that names a suggested holder must carry a `39105` receipt for
  that pubkey **or** an `e` receipt to an item they held; a name with neither
  is `invalid: unresolved receipt`. If it carries `skill` tags they must be
  slugs present on that `39105` at ingest time. The relay also rejects a
  suggested holder whose `open_limit` is set and already met by their
  `accepted` items (`invalid: holder at limit`) — the limit is theirs and
  the relay honours it, the agent's judge merely gets there first.
- Removing a member (NIP-43) leaves the `39105` in place, tagged by the
  membership projection as inactive; clients hide it. Re-joining picks it
  up again.

### 5.5 Done from talk — the one agent-authored command

Feature 5 says a DRI closes their ticket by saying so where they already
talk, with no card and no waiting. Under this protocol the agent cannot sign
as the DRI, so the relay accepts `io_done` **from the community's org agent
pubkey** only when all of these hold, checked in the executor:

1. The command carries exactly one `["e", <message-id>, "", "receipt"]`.
2. That message exists in this community, is `kind:9` or `kind:40002`, and
   its **author is the item's `dri`**.
3. The message is younger than 24 hours and was not already used as a
   receipt.
4. The message does not carry a `["transcript", …]` tag — text produced by
   speech-to-text is never a signature (features §5). The desktop huddle STT
   pipeline must tag the `kind:9` messages it posts so this check has
   something to read; until it does, the agent must treat any message from a
   huddle channel as a transcript.
5. The item has no open child (rule 3 as usual).

The ledger row is `item_done` with `actor = <dri>`, `detail.via = "agent"`,
`receipt_event_id = <message-id>`. The `39101` gets `done_receipt` = the
message, not the command, so the receipt a reader clicks is the DRI's own
words. The DRI can undo it with `io_reopen` for seven days.

Every other `io_done` from the agent's pubkey is rejected `restricted`. The
agent has no other command it may send. "The community's org agent pubkey"
is `39103.agent`, read from the projection at ingest; if it is empty, the
exception does not exist.

### 5.6 Money — next version, contract-settled

Not in the first version. Fixed here so nothing built now has to move.

The org's funds sit in a **smart contract the Shapers control**. A passed
`money` proposal in Buzz is the release instruction:

```
 io_money_propose ──► proposal(money, open) ──► io_vote reaches rule ──► passed
        ──► treasury bridge submits release(payee, amount) to the contract
        ──► chain confirms ──► io_money_released (tx) ──► settled
```

- **The vote is the authorisation.** The Shapers' `io_vote` events are the
  signatures; the bridge presents the passed `39102` (rule, eligible, votes,
  each a signed event) to the contract's verifier, or acts as the contract's
  designated executor keyed to the community. Which of the two depends on
  the chain; the protocol only requires that the contract cannot pay without
  a passed proposal and that the proposal cannot be forged (it is
  relay-signed over person-signed votes).
- **Nobody marks it settled.** `io_money_released` (50014) is signed by the
  **treasury bridge key** — a relay-side worker, the same shape as
  `io_scheduler`, watching the chain — and carries the transaction id. The
  relay verifies the bridge key, moves the proposal to `settled`, writes
  `settlement = { tx, chain, contract, at }`, ledger `proposal_settled`. A
  release the chain rejects leaves the proposal `passed` with
  `settlement.error`, and the bridge retries with backoff; a person can see
  it stuck, not fix it by hand.
- **Payee profiles** sum `settled` proposals where they are `p … subject`;
  each line links the proposal and the tx.
- **Out only.** Incoming funds are chain facts the bridge may mirror as
  ledger rows (`treasury_received`); they are never proposals.
- **Nobody votes on themselves** applies: a Shaper who is the payee is not
  eligible.
- **The agent never touches it.** It drafts `50100 kind=money` on request;
  a person sends `50013`; Shapers vote; the contract pays.

Until this lands the relay rejects `50013`/`50014` and `rules.money` is
inert; the org's payments run through Hypha's existing proposals.

### 5.7 Join — later, a Shaper decision

Not in the first version, where membership is by invite link only (§6.6).
Reserved so the kind and the proposal shape do not move:

```
 join request (relay, from a non-member) or io_join_propose (a member, for `p`)
        ──► proposal(join, open) ──► io_vote reaches rule ──► NIP-43 add-member for p
```

- A `join` proposal is people only, has no item, and its subject `p` is
  never eligible (they are not a Shaper anyway).
- How a non-member's request reaches the relay — a knock on a public
  landing page, a NIP-29 `kind:9021`-style request scoped to the
  community — is decided when this ships. Until then the relay rejects
  `50016` and `rules.join` is inert.

---

## 6. Relay behaviour

### 6.1 Ingest

- Commands enter the existing pipeline (`handlers/ingest.rs`) and route to
  `command_executor.rs` via `is_command_kind`. The executor gains one module,
  `handlers/intelligent_org/`, with one function per command and one shared
  `apply(tx, ledger_row, projection_change, state_event)` helper — the single
  write path.
- Drafts (`50100`) are stored like any regular event, then `39104 open` is
  emitted. The relay validates the tag set in §4.3 (a draft without `needs`,
  `kind`, `gap`, or a receipt is rejected `invalid: draft shape`), and that
  every receipt resolves in this community (`invalid: unresolved receipt`).
  Resolution is community-wide: a receipt may point into any channel or DM,
  including one the draft's `needs` party is not a member of — the agent is
  a member of all of them, and the reader gets the cited event through the
  receipt read (§6.8). Receipts that do not resolve are the one hard gate on
  the agent; a draft that fails it never reaches a card.
- Health reads (`50101`) are stored; the relay checks the `item` exists and
  every `rows` id resolves.
- Agent notes (`50103`) are stored after the checks in §4.7c — author is
  `39103.agent`, `note` is known — with **no receipt check**: a
  `draft_dropped` note may name a gap or item that never existed. The
  relay writes the matching ledger row. A `50103` from any other pubkey
  is rejected `restricted: not the org agent`.
- Progress notes (`50102`) are stored after the checks in §4.7b — holder or
  attested agent, item held, `ref` and every `commit` present in the home
  repository. The relay then sets `last_progress` on the `39101` and emits
  it (ledger `progress_noted`, `actor` = the signer, `detail.for` = `dri`).
  This is the one place ingest reads the git store; it reads refs and
  reachability, never content.
- Client `EVENT` of `39100–39105` is rejected `restricted: relay-only kind`.

### 6.2 Projections

Typed tables in `buzz-db`, all with `community_id`, following the sidecar
pattern (`moderation_reports`, workflow tables):

`io_shapers`, `io_direction` (one row per version), `io_work_items` (roots
carry `channel_id`, `repo_coord`, `project_coord`; children carry `branch`;
every row `last_progress_at`), `io_proposals`, `io_votes`, `io_drafts`
(with outcome columns), `io_progress` (`item_id`, `signer`, `dri`, `ref`,
`head`, `hint`, `merged_into`, `event_id`, `at`), `io_health`,
`io_health_ratings`, `io_profiles` (with a `skills text[]` column,
GIN-indexed, for the agent's candidate query), `io_ledger`, and
`io_hosted_agents` — `(community_id, pubkey, provisioned_at, budget?)`,
written by the relay operator's provisioning, never by a command; read once
at `39103` bootstrap and again when a `shapers/agent` proposal with no `p`
returns the community to the hosted default.

`io_ledger` is `(community_id, at, actor, verb, object_type, object_id,
receipt_event_id, detail jsonb)`. Verbs: `shaper_offered`, `shaper_added`,
`shaper_removed`, `shaper_stepped_down`, `shaper_offer_lapsed`,
`rules_changed`, `agent_changed` (detail: `from`, `to`, `hosted`),
`direction_proposed`, `direction_confirmed`,
`proposal_opened`, `vote_cast`, `vote_changed`, `proposal_passed`,
`proposal_rejected`, `proposal_expired`, `proposal_settled`, `member_joined`, `item_created`, `item_offered`, `offer_renotified`,
`offer_expired`, `item_accepted`, `item_declined`, `item_released`,
`item_done`, `item_closed_by_rule`, `item_entered_review`, `due_changed`,
`orphaned_by_close`, `home_created` (detail: `channel`, `repo`, `project`),
`home_member_synced` (detail: `p`, `role`, `added|removed`, `why`),
`agent_membership_synced` (detail: `agent`, `channels`, `dms`, `why` —
`bootstrap|agent_changed`; per-room joins at creation write no row),
`progress_noted` (detail: `for`, `hint`, `head`), `draft_stored`,
`draft_decided`, `draft_dropped` (judge failure; detail: `move`, `gap`,
`reason` — from a `50103`), `trigger_skipped`, `budget_exhausted`, `tally`
(all three from a `50103`, `actor = <agent>`), `health_read`,
`health_rated`, `profile_set`.

Ledger rows for rule-driven changes (`offer_expired`, `item_closed_by_rule`,
`item_entered_review`, `offer_renotified`) have `actor = "relay"` and a
`receipt_event_id` pointing at the `39101` they produced.

### 6.3 Date rules

One relay job, `io_scheduler`, runs every five minutes per community (same
worker pattern as `admin_action_worker`):

1. Offers past half their window → renotify (`kind:44100`-style notification
   to `offered_to`; ledger `offer_renotified`). Past the window → return to
   `open`.
2. Roots in `accepted` with `now ≥ due_at − 0.2 × (due_at − approved_at)`
   (floor two days) → `in_review`.
3. Roots in `accepted`/`in_review` with `now ≥ due_at` → `done`,
   `closed_by = "rule"`, children handled per §5.1 rule 5.
4. Drafts past `expiration` → `39104 expired`.
5. Proposals `open` past `expires_at` → `expired` (ledger
   `proposal_expired`; notify `opened_by`). Seats in `39103.offered` past
   `offer_window_secs` → dropped (ledger `shaper_offer_lapsed`).

Each step is idempotent and each transition is one transaction with its
ledger row and its state event. The agent never runs these rules; it
subscribes to their results.

### 6.4 Shapers room

Three commands change the Shaper set, and each updates `39103` and the
membership of the `room` channel in the same transaction (NIP-29 put/remove
user): `io_shaper_accept` (after a passed `shapers/add`), a passed
`shapers/remove` executing, and `io_shaper_step_down`. There is no reconcile
loop because there is no second writer; a Shaper removed from `#shapers` by
Buzz's own channel administration is re-added by the next `39103` write,
and the ledger notes it.

**Bootstrap.** When `39103` does not exist, the community owner may send
`io_shapers_propose op=add` naming themselves. The relay creates the private
`#shapers` channel, adds the owner and the org agent, adds the org agent to
**every channel and DM that already exists** in the community (§6.8; ledger
`agent_membership_synced why=bootstrap`), writes `39103` with
`shapers=[owner]`, `founder=owner`, `agent` = the relay's **hosted default**
for this community (`agent_hosted=true`; how the operator provisions it is
in Design § Where it runs), default rules, and the proposal passes
and executes immediately — the owner is the only eligible Shaper and the
subject rule is waived for this one self-add. Every later add is a normal
proposal: with one Shaper it passes on their agree; with two or more it
takes the rule. A second Shaper's seat is live only when they accept it.

### 6.5 Reads

No new HTTP endpoint. The doors are REQ filters:

| Door              | Filter                                                                                  |
| ----------------- | --------------------------------------------------------------------------------------- |
| Overview          | `{kinds:[39100]}`, `{kinds:[39103]}`, `{kinds:[39101], "#t":["project"]}`                |
| Work              | `{kinds:[39101]}` (tree assembled client-side from `root`/`parent`), `{kinds:[50101], "#item":[…]}`; `last_progress` on each row is enough for the _last moved_ column |
| Item page         | `{kinds:[39101], "#d":[id]}`, `{kinds:[39101], "#parent":[id]}`, `{kinds:[50001–50021], "#item":[id]}` for the trail, `{kinds:[50102], "#item":[id]}` for the **work log**; commits link into the repo browser at `home.repo` |
| Decisions         | `{kinds:[39102]}` with `#t` / `#status`; each card renders `votes.len(agree)` of `needed` |
| My Work           | `{kinds:[39101], "#p":[me]}`, `{kinds:[50100], "#needs":[me]}`, `{kinds:[39102], "#p":[me], "#status":["open"]}` (as eligible voter or offered seat); Shapers add `{kinds:[50100], "#needs":["shaper"]}` |
| Direction page    | `{kinds:[39100], "#d":[slug]}` plus `{kinds:[39102], "#t":["direction"], "#status":["passed"]}` for history |
| Profile           | `{kinds:[39105], "#d":[pubkey]}`, `{kinds:[39101], "#p":[pubkey]}`, `{kinds:[39102], "#p":[pubkey]}`; "who can do X" is `{kinds:[39105], "#skill":[slug]}` |

**Verified 15 Sep ([Codebase verification V2](./intelligent-org-codebase-verification.md#v2--multi-letter-tag-filters-cs-1)):
the relay's filter type cannot express multi-letter tag filters at all.**
`#needs`, `#item`, `#parent`, `#status`, `#skill` above are therefore
placeholders for the single-letter tags decided in
[Readiness D11](../plans/intelligent-org-readiness.md#3-decisions-to-pin)
— `n`, `i`, `u`, `s`, `k` — and this table and the § 4 tag tables are
rewritten with those letters in the D-decisions PR. `#t`, `#d`, `#p` are
native. Only `#e`, single-value `#p`, `#d`, and `#h` are pushed into SQL
today; the rest are matched after the page, which R-2 of the Development
plan fixes with generic tag pushdown.

The Home feed's `needs_action` bucket (`buzz-db/src/store/feed.rs`,
`query_needs_action`) gains three sources: `39101` where `#p … "offered"` is
the viewer, `50100` where `#needs` is the viewer (or `shaper` for Shapers),
and open `39102` for Shapers.

### 6.6 Membership — invite links

In the first version the only way into an org is the community's existing
**invite link** (`POST /api/invites` mints a code; `/invite/<code>` lands;
`POST /api/invites/claim` adds the claimant as a NIP-43 member with role
`member`) — or having created the community. This protocol changes one
thing about it: **mint authorisation includes Shapers.** `mint_invite`
currently allows `owner` and `admin`; it additionally allows any pubkey in
the community's `39103.shapers`, read from the projection at request time.
Lifetime and `max_uses` are the invite's own controls; the Shaper picks
them. Nothing else changes — no proposal, no card, no relay-side org state
on a claim. The ledger notes `member_joined` with `actor = <claimant>`,
`detail.via = "invite"`, `detail.minted_by = <pubkey>` so Overview can show
who invited whom, and the org agent greets the new member in a DM (a
`kind:9`/DM message, not an `io_*` event).

Owners and admins keep Buzz's direct add (`kind:9030`) and remove; those are
relay administration, outside the org's decisions. When `join` proposals
land (§5.7) the invite path stays: an invite is a Shaper's decision made in
advance, a join request is one made on demand.

### 6.7 Project home — room, repository, project

Every live project has a **home**: a room to talk in, a repository to push
to, and the NIP-MP project that ties the two together in Buzz's existing
Projects (git) surface. The relay creates all three when the `project`
proposal passes, in the transaction that creates the root (§5.3), and
writes their coordinates into `39101.home`:

| Object       | Kind    | Signer        | Tags / content                                                                                                       |
| ------------ | ------- | ------------- | -------------------------------------------------------------------------------------------------------------------- |
| Room         | NIP-29 channel | relay | name `<slug>`, topic = the project title; community-visible, anyone in the community may join to read and talk        |
| Repository   | `30617` | **the relay** | `d=<slug>`, `name`, `description` = brief, `clone` = this relay's git URL, `buzz-channel=<room>`, `maintainers=[dri]`, `["buzz-protect","refs/heads/main","push:admin","no-force-push","no-delete"]` |
| Project      | `30621` | **the relay** | `d=<slug>`, `name`, `buzz-channel=<room>`, one member: the repository above                                          |

**Why the relay signs, not the DRI.** A `30617` is keyed by `(author, d)`
and only its author can replace it — so the signer is whoever gets to
rebind the room, change `maintainers`, and change the protection rules.
Three facts decide it:

1. A project passes with **no DRI** more often than not (§5.1 — a root is
   `open` until offered and accepted). A DRI-signed repository could not
   exist until someone accepted; a relay-signed one exists the moment the
   Shapers say yes, and the room is there for the DRI conversation itself.
2. **DRIs change.** Release, decline, a `dri` vote naming someone else. If
   the person signed, every change would re-announce the repository under a
   new key: new coordinates, every `a` tag in issues and patches pointing at
   the old one, and the old holder still able to edit the announcement the
   org now depends on. Relay-signed, the coordinates never move and the
   relay rewrites `maintainers` in the same transaction as `dri`.
3. **Push rights follow the tree, not one person.** Buzz's git ACL is the
   room: channel role = repo role (`buzz-core/src/git_perms.rs`), and
   `buzz-protect` rules bind everyone including the owner. The relay
   already knows who holds what under a root; it is the only writer that
   can keep the room's membership equal to the holders.

The relay key owning the repository grants nobody a bypass: the relay is not
a pusher, and the protection rules apply to the owner too. What the signer
choice does **not** touch is ticket state — that lives in `39101`, relay-
signed either way. It also does not touch **who did the work**: commits are
signed by the people who made them (`git-sign-nostr`), and a `50102` names
its `dri`.

**Membership sync.** The relay keeps the room's roster equal to the tree:

| Ledger event                        | Room change                                                                                 |
| ----------------------------------- | ------------------------------------------------------------------------------------------- |
| root `item_accepted` / `dri` passed | holder → **admin** of the room (`push:admin` on `main`); `maintainers=[holder]` on `30617`  |
| child `item_accepted`               | holder → **member** (may push branches; not `main`)                                         |
| `item_released`, holder replaced    | admin → member (history stays readable; they can still push a branch); `maintainers` rewritten |
| any `item_accepted`                 | the holder's NIP-OA-attested agents → **bot** in the room, so the Work sync agent can push the holder's branch and post `50102` (bots push as members; `policy.rs`) |
| root `done`                         | protection on `refs/heads/*` gains `push:admin`; the room is archived one week later by the scheduler unless a Shaper reopens the root |

Every sync writes `home_member_synced`. Members who joined the room to talk
are never removed by the sync; it only adds roles for holders and lowers
them on release. A holder removed from the room by Buzz's own channel
administration is re-added on the next sync, as `#shapers` is (§6.4).

**The desktop's "Open in editor".** On a held ticket the desktop clones
`home.repo` into the member's repos directory (the same `repos_dir`
managed agents use), creates `branch` from `main` if it does not exist, and
opens it in the member's editor. Nothing about this is relay state; it is
what makes the Work sync agent's branch-to-ticket match reliable (§4.7b).

**Sovereign relays** without a git store: `home.repo` and `home.project`
are absent, `50102` notes are rejected `invalid: no home repository`, and
the room is still created. Projects then run on talk alone, as before.

### 6.8 The org agent everywhere — membership and receipt reads

The org agent is a member of every conversation in the community. This is
a fork decision (Design § The layers on Buzz — _All of it is the org's_),
and the relay enforces it; no client asks for it and none can decline it.

**Membership.** Whenever the relay creates a conversation it adds
`39103.agent` to it in the same transaction:

| Event                                   | Effect                                                                 |
| --------------------------------------- | ---------------------------------------------------------------------- |
| NIP-29 channel create (`kind:9007`) or any relay-created room (`#shapers`, a project home) | `39103.agent` put as `member` alongside the creator |
| `kind:41010` DM open                    | `39103.agent` added to the participant set                             |
| `kind:41011` DM add member              | unchanged — the agent is already there                                 |
| `39103` bootstrap (§6.4)                | every existing channel and DM backfilled; ledger `agent_membership_synced why=bootstrap` |
| `shapers/agent` passes (§5.3)           | every membership row moved from the old key to the new; ledger `agent_membership_synced why=agent_changed` |

The membership is real — the same rows any member has, so Buzz's existing
read gate is unchanged and the agent's REQs and NIP-50 searches pass it as
any member's would. The agent removed from a room by Buzz's own channel
administration (a `9001` remove-user) is re-added on the next write to that
room, and the ledger notes it, as `#shapers` and project rooms already do
(§6.4, §6.7). A `41012` hide is a sidebar preference and touches no
membership.

**Not a participant, for display.** `39103.agent` is excluded from the
**DM identity**: the participant set that dedupes `41010` opens (two people
opening "the DM between us" find the same one), that names the conversation
in the sidebar, and that the channel-summary sidecar (`40901`) reports as
`participants`, is computed without it. Membership lists (`39002`) do carry
the agent — they are the truth — and clients filter `39103.agent` out of
participant renderings and member counts. A 1:1 stays a 1:1; a room with
five people shows five.

**Receipt read.** Every event this protocol defines is community-global
(§1), so a member can always see a draft, a proposal, an item, or a health
read. A receipt on one of those may point into a channel or DM the reader
is not a member of. The relay resolves that case as follows: a REQ
`{ids:[<event-id>]}` from an authenticated community member is served for an
event **in a room the member is not in** if and only if that id appears as a
`["e", <id>, "", "receipt"]` (or `done_receipt`, or a `50101.rows` id, or
a `50102.commits` entry) on a community-global `io` event the relay has
stored. The reader gets **that one event** — author, content, timestamp,
channel — and nothing else from the room: no `#h` REQ, no window, no
search. Every other read of that room is gated as today. A receipt is a
window onto one message, not a key to the room.

**What this is not.** The agent's membership does not make the model read
everything: which rooms the agent screens without a mention, and which need
one, is the agent's behaviour (Design § Triggers), not the relay's. Nor does
it widen any human's membership: members still see only their rooms plus
whatever the org has cited. The transparency is of the record the org
builds, through receipts, not of the rooms themselves.

---

## 7. Client behaviour

- Clients sign commands with the user's key (desktop: `sign_event` in
  `src-tauri`; CLI: `BUZZ_PRIVATE_KEY`) and publish through the normal
  `EVENT` path or `POST /events`. They never construct `39100–39105`.
- Clients render state from `39100–39105` and drafts from `50100`
  + `39104`; they do not derive state from commands. The trail on an item
  page is the commands with `["item", id]`, newest first.
- A card's primary tap builds the command **and** carries the draft `e` tag
  so settlement is atomic (§3.2). The secondary tap (Decline / Not yet) is
  `io_draft_decide`.
- `io_decline` carries no reason: a person's decline of a **work offer**
  records nothing but the fact. The fixed reason chips on agent drafts
  (`io_draft_decide`) are for the agent's learning; `39104` is
  community-readable like all state, which is why that list is impersonal
  (_already covered_, _too big_ — never _I don't like this person_).
- The **My Profile → About & skills** form is a `50021` on Save, carrying
  the whole profile. A `profile` card in the DM is the same command with
  the draft `e` tag; editing the card before Confirm settles it `amended`.
  A suggestion card that names a holder renders `matched.skills` as chips
  that link to the person's profile; a DRI's offer picker shows each
  candidate's skills, about, and open count next to their name.
- The org agent (Design §"The org agent") subscribes to `39100–39105`,
  `50100` (its own and others'), `50102` progress notes, and `kind:9`/`40002`
  in every channel and DM — it is a member of all of them (§6.8). It runs
  the model on a batch only in the rooms it screens passively (`#shapers`,
  project rooms, each member's own DM with it) or when a message carries a
  `["p", <39103.agent>]` mention; elsewhere it stores nothing and reads
  nothing into a prompt until tagged (Design § Triggers). Its NIP-50
  searches behind a draft or an answer span the whole community. It
  publishes `50100` and
  `50101`, sends `io_done` only under §5.5, and publishes `50103` agent
  notes — `draft_dropped` for judge failures, `trigger_skipped`,
  `budget_exhausted`, and the Friday `tally` — so the tally can count what
  it did not say (`buzz org ledger note` builds the same event). It never
  publishes `50102`: a progress note is the
  holder's word (or their own agent's), not the org's.
- A member's **Work sync** agent (Design § Members' own agents) is an
  ordinary managed agent with the member as NIP-OA owner. It reads
  `{kinds:[39101], "#p":[owner]}` for the items its owner holds, pushes the
  owner's work branch to `home.repo`, and publishes `50102`. It sends no
  `io_*` command; **Mark done** stays the holder's tap.
  When it names a holder it reads `39105` and past `39101` rows only; it
  does not infer skills from talk into anyone's profile — talk becomes a
  `profile` draft to that person, and only their Confirm makes it true.

---

## 8. Security and privacy

- Authority is a relay check, never a client check. Role data (`39103`,
  `dri`, `offered_to`, `needs`) is read from projections inside the command
  transaction, so a stale client cannot promote with a revoked role.
- With two or more Shapers no proposal passes on one Shaper's vote unless
  the Shapers together set a rule of `1` for that kind — and changing a
  rule needs every Shaper. A Shaper cannot vote on their own naming,
  payment, or removal (`eligible` excludes the subject), and cannot add or
  remove Shapers alone. With one Shaper every rule resolves to their own
  agree by design (the founder alone).
- Membership in the first version is the existing invite path with one
  widened check (Shapers may mint). An invite code is still a signed,
  expiring, optionally use-limited HMAC token; a Shaper leaking one leaks at
  most what the owner leaking one already could. When `join` proposals land
  they will be the only proposal whose execution reaches outside the `io_*`
  tables, reusing the NIP-43 path with its audit row.
- Drafts are community-readable, including `shadow` ones. Shadow hides them
  from cards, not from a raw REQ. Do not put anything in a draft that the
  community may not read.
- The org profile (`39105`) is community-readable and **self-authored
  only**: the relay refuses a `50021` for another pubkey and a `profile`
  draft addressed to anyone but its subject, so nothing about a person's
  skills is written into org state by the agent or another member. It is
  scoped to the community — a person's `39105` on one relay is not read by
  another — and it says what the person chose to say; the agent cites it,
  never a category it inferred. The self-set `open_limit` is a request the
  relay honours, not a measurement of the person.
- The org agent's key is held by whoever runs the agent — the relay
  operator for the hosted default, the community for one it runs itself
  (`39103.agent`, §4.5). Losing it loses provenance for future drafts, not
  authority: it never had any. The operator holding the key can put bad
  drafts in front of Shapers, and nothing else — but see the next point for
  what the key can _read_.
- **The org agent is in every channel and every DM** (§6.8). Nothing said in
  the community is private from the org: the agent may search any
  conversation and cite any message as a receipt, and a receipt is readable
  by whoever can see the event that cites it, whether or not they were in
  the room. This is deliberate — the fork's radical-transparency decision
  (Design § What the agent hears, and who sees it) — and it is stated once,
  where people join, not per conversation; the agent is not rendered as a
  participant. Three limits hold it in shape: the model reads a room only
  when it screens that room passively or is tagged there, so presence is
  not surveillance; a receipt read returns one cited event, never the room;
  and the agent still has no command. The trust placed in whoever holds the
  agent's key — the operator by default, the Shapers' own agent after a
  `shapers/agent` vote — is therefore a **read** trust over the whole
  community and a **write** trust over nothing. Whatever reaches the model
  (a tagged DM, a search hit) reaches that key-holder's model provider.
- A progress note (`50102`) is community-readable, like every other read.
  It carries a summary, commit shas, and a file count — never a diff, a
  path list, or anything from a file. The agent that writes it runs on the
  holder's machine under the holder's control; it reads only the checkout
  of the branch matched to a held item, and the holder can stop it at any
  time. A member who runs no agent is a member in full: done is still a
  sentence or a button, and the project's health read says only that no
  notes are being posted, not that no work is being done.
- No event in this protocol is p-gated or author-only; everything the org
  decides is legible to every member, and every receipt behind a decision
  is readable by whoever can see the decision (§6.8). That is the product.

---

## 9. Worked example — a project from direction to close

1. Shaper Maya sends `50002` (`d=objectives`, `base=2`) with three lines.
   Relay: stores it, emits `39102` (`direction`, open, rule `majority`,
   `eligible=[Maya, Sam]`, `needed=2`).
2. Maya sends `50003` agree on her own proposal; Sam sends `50003` agree.
   Relay: on Sam's vote `agrees=2 ≥ needed` → `passed`; emits `39100`
   `objectives` v3; ledger `direction_confirmed`.
3. The org agent sees `39100` v3, runs move 1, publishes `50100`
   (`kind=project`, `needs=shaper`, `gap=objectives@3#l_7f3a`, `origin=gap`,
   `ref=objectives@3#l_7f3a`). Relay: stores, emits `39104 open`.
4. Maya taps **Agree** on the card: `50004` with `["e", draft, "", "draft"]`.
   Relay, one transaction: `39104 accepted`; `39102` (`project`, open,
   `needed=2`); the card now sits on Sam's My Work too, showing _1 of 2_.
5. Maya and Sam agree (`50003`). Relay: `passed`; creates the root, `39101`
   `open`, `approved_at = now`.
6. The agent sees a root with no holder, queries `io_profiles` for skills
   near the brief, and publishes a `dri` draft naming Lea (`needs=shaper`,
   `a=39105:…:Lea receipt`, `skill=hosting-events`, `matched.items` = the
   stall she held last year). Lea wrote _hosting events_ on her profile the
   week she joined (`50021` → `39105` v1). Maya taps **Offer to Lea**:
   `50006` with the draft tag. Relay: `39104 accepted`; `39101` `offered`,
   `p=Lea offered`.
7. Lea sends `50007`. Relay: `39101` `accepted`, `dri=Lea`.
8. The agent sees a held root, runs move 2, publishes `ticket` drafts to
   `needs=Lea`. Lea taps **Offer to Jun** on one: `50005` (`parent`, `p=Jun`,
   draft tag). Relay: child `offered`.
9. Jun accepts; later writes "rota printed — done" in `#saturday-stall`. The
   agent recognises it and sends `50009` with the message as `receipt`
   (§5.5). Relay: the message's author is the `dri`, it is fresh, not a
   transcript, no open children → `39101` `done`, `done_receipt` = Jun's
   message. The agent posts the receipt back in the room.
10. Eleven weeks in, the scheduler moves the root to `in_review`. The agent
    publishes a `review` draft (`needs=shaper`) with the brief assembled from
    `io_ledger` rows and a `follow_up` recommendation.
11. Maya taps **Open the follow-up**: `50004` with the draft tag. Both
    agree. A new root exists. On `due_at` the scheduler closes the old one,
    `closed_by=rule`.
12. The agent sees the close, publishes an `objectives` redraw draft striking
    `l_7f3a`. Maya sends `50002` with the rendered lines and the draft tag;
    both agree; `39100` v4.

Every arrow above is an event id somebody can click.

---

## Related

- [The Intelligent Organization — Design](./intelligent-org-design.md) — why the protocol is shaped this way, the org agent, the surfaces, the build order
- [The Intelligent Organization — What it is](../product/intelligent-org-features.md) — the behaviour this protocol must deliver
- [Intelligent Org on Buzz — Phase 0](../plans/intelligent-org-phase-0.md) — the subset built first
- [CONTRIBUTING.md — How to Add a New Event Kind](../../../CONTRIBUTING.md#how-to-add-a-new-event-kind)
- `docs/nips/NIP-MP.md`, `docs/nips/NIP-ER.md` — the house style this becomes `NIP-IO` in
