-- Intelligent organization — typed projections (Protocol §6.2).
--
-- Twelve community-scoped sidecar tables following the `moderation_reports` /
-- workflow pattern: every row carries `community_id`, every key leads with it,
-- and every table is attached to the universal community write fence. The
-- relay-signed state events (`39100–39105`) remain the wire truth; these rows
-- are the executor's, scheduler's, and agent's query surface for the same
-- state. Each projection keeps the canonical Protocol §4 content JSON next to
-- the typed columns the relay filters on, so a row can re-emit its event and a
-- query never has to parse JSON to find a due date or a holder.
--
-- No behaviour: the tables are written by nothing until R-3 (the executor)
-- lands. `io_hosted_agents` is the one table written by operator provisioning
-- rather than by a command.

-- ── Shapers and rules (kind:39103) — one row per community ───────────────────
CREATE TABLE io_shapers (
    community_id        UUID NOT NULL REFERENCES communities(id),
    founder             BYTEA NOT NULL CHECK (length(founder) = 32),
    -- Live seats. `offered` seats live only in `content` until accepted.
    shapers             BYTEA[] NOT NULL,
    room_channel_id     UUID,
    agent               BYTEA CHECK (agent IS NULL OR length(agent) = 32),
    agent_hosted        BOOLEAN NOT NULL DEFAULT true,
    -- Canonical §4.5 content of the latest 39103.
    content             JSONB NOT NULL,
    event_id            BYTEA NOT NULL CHECK (length(event_id) = 32),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (community_id)
);

-- ── Direction artifacts (kind:39100) — one row per (slug, version) ───────────
CREATE TABLE io_direction (
    community_id        UUID NOT NULL REFERENCES communities(id),
    slug                TEXT NOT NULL CHECK (slug IN ('mission', 'vision', 'objectives', 'strategy')),
    version             INTEGER NOT NULL CHECK (version >= 1),
    -- Canonical §4.1 content of this version.
    content             JSONB NOT NULL,
    event_id            BYTEA NOT NULL CHECK (length(event_id) = 32),
    proposal_id         UUID,
    confirmed_by        BYTEA NOT NULL CHECK (length(confirmed_by) = 32),
    confirmed_at        TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (community_id, slug, version)
);

-- ── Work items (kind:39101) ───────────────────────────────────────────────────
-- Roots (projects) carry the home triple; children (tickets) carry `branch`.
CREATE TABLE io_work_items (
    community_id        UUID NOT NULL REFERENCES communities(id),
    id                  UUID NOT NULL,
    root_id             UUID NOT NULL,
    parent_id           UUID,
    depth               INTEGER NOT NULL CHECK (depth >= 0),
    kind                TEXT NOT NULL CHECK (kind IN ('project', 'ticket')),
    state               TEXT NOT NULL CHECK (state IN ('open', 'offered', 'accepted', 'in_review', 'done')),
    dri                 BYTEA CHECK (dri IS NULL OR length(dri) = 32),
    offered_to          BYTEA CHECK (offered_to IS NULL OR length(offered_to) = 32),
    offered_at          TIMESTAMPTZ,
    due_at              TIMESTAMPTZ NOT NULL,
    approved_at         TIMESTAMPTZ,
    done_at             TIMESTAMPTZ,
    channel_id          UUID,
    repo_coord          TEXT,
    project_coord       TEXT,
    branch              TEXT,
    last_progress_at    TIMESTAMPTZ,
    -- Canonical §4.2 content of the latest 39101.
    content             JSONB NOT NULL,
    event_id            BYTEA NOT NULL CHECK (length(event_id) = 32),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (community_id, id),
    CHECK ((parent_id IS NULL) = (depth = 0)),
    CHECK (parent_id IS NULL OR parent_id <> id)
);

CREATE INDEX idx_io_work_items_parent
    ON io_work_items (community_id, parent_id);
CREATE INDEX idx_io_work_items_root
    ON io_work_items (community_id, root_id);
-- The scheduler's sweeps (§6.3): offers by window, roots by due date.
CREATE INDEX idx_io_work_items_state_due
    ON io_work_items (community_id, state, due_at);
CREATE INDEX idx_io_work_items_dri
    ON io_work_items (community_id, dri)
    WHERE dri IS NOT NULL;
CREATE INDEX idx_io_work_items_offered_to
    ON io_work_items (community_id, offered_to)
    WHERE offered_to IS NOT NULL;

-- ── Proposals (kind:39102) ────────────────────────────────────────────────────
CREATE TABLE io_proposals (
    community_id        UUID NOT NULL REFERENCES communities(id),
    id                  UUID NOT NULL,
    kind                TEXT NOT NULL CHECK (kind IN ('direction', 'project', 'dri', 'shapers', 'money', 'join')),
    status              TEXT NOT NULL CHECK (status IN ('open', 'passed', 'rejected', 'expired', 'settled')),
    opened_by           BYTEA NOT NULL CHECK (length(opened_by) = 32),
    opened_at           TIMESTAMPTZ NOT NULL,
    expires_at          TIMESTAMPTZ NOT NULL,
    -- The 50100 this proposal settles, when opened from a card.
    draft_event_id      BYTEA CHECK (draft_event_id IS NULL OR length(draft_event_id) = 32),
    -- Frozen at opening (§5.2): agreeing votes required to pass.
    needed              INTEGER NOT NULL CHECK (needed >= 0),
    decided_at          TIMESTAMPTZ,
    -- Canonical §4.4 content of the latest 39102 (votes, eligible, payload).
    content             JSONB NOT NULL,
    event_id            BYTEA NOT NULL CHECK (length(event_id) = 32),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (community_id, id)
);

-- Open proposals by expiry for the §6.3 sweep; by kind for the Decisions door.
CREATE INDEX idx_io_proposals_status_expires
    ON io_proposals (community_id, status, expires_at);
CREATE INDEX idx_io_proposals_kind_opened
    ON io_proposals (community_id, kind, opened_at DESC);

-- ── Votes — one row per (proposal, voter); a changed vote overwrites ─────────
CREATE TABLE io_votes (
    community_id        UUID NOT NULL REFERENCES communities(id),
    proposal_id         UUID NOT NULL,
    voter               BYTEA NOT NULL CHECK (length(voter) = 32),
    vote                TEXT NOT NULL CHECK (vote IN ('agree', 'decline')),
    reason              TEXT,
    -- The 50003 (or the D1 opener-vote command) that cast it.
    receipt_event_id    BYTEA NOT NULL CHECK (length(receipt_event_id) = 32),
    cast_at             TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (community_id, proposal_id, voter),
    FOREIGN KEY (community_id, proposal_id) REFERENCES io_proposals (community_id, id)
);

-- ── Drafts (kind:50100) with their outcome (kind:39104) ──────────────────────
CREATE TABLE io_drafts (
    community_id        UUID NOT NULL REFERENCES communities(id),
    -- The 50100 event id.
    event_id            BYTEA NOT NULL CHECK (length(event_id) = 32),
    author              BYTEA NOT NULL CHECK (length(author) = 32),
    draft_kind          TEXT NOT NULL CHECK (draft_kind IN ('project', 'dri', 'ticket', 'done', 'review', 'objectives', 'direction', 'profile', 'money')),
    -- `["n", …]`: a pubkey hex or the literal `shaper`.
    needs               TEXT NOT NULL,
    gap                 TEXT NOT NULL,
    move                SMALLINT CHECK (move IS NULL OR move BETWEEN 1 AND 4),
    origin              TEXT CHECK (origin IS NULL OR origin IN ('talk', 'gap')),
    -- `["i", …]` for dri/review/money drafts; `["u", …]` for ticket/done drafts.
    item_id             UUID,
    parent_id           UUID,
    shadow              BOOLEAN NOT NULL DEFAULT false,
    expires_at          TIMESTAMPTZ,
    -- Canonical §4.3 payload for `draft_kind`.
    payload             JSONB NOT NULL,
    -- Outcome columns (§4.6): the latest 39104 for this draft.
    status              TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'accepted', 'amended', 'declined', 'expired', 'shadow')),
    decided_by          BYTEA CHECK (decided_by IS NULL OR length(decided_by) = 32),
    decided_at          TIMESTAMPTZ,
    decline_reason      TEXT CHECK (decline_reason IS NULL OR decline_reason IN ('already_covered', 'not_what_the_line_meant', 'too_big', 'too_small', 'wrong_holder', 'not_now', 'other')),
    -- The command that settled it, when accepted or amended.
    result_event_id     BYTEA CHECK (result_event_id IS NULL OR length(result_event_id) = 32),
    outcome_event_id    BYTEA CHECK (outcome_event_id IS NULL OR length(outcome_event_id) = 32),
    created_at          TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (community_id, event_id),
    -- Table-level CHECKs are spelled with `IS NULL` only: pgschema (the
    -- desired-state bootstrap) silently drops a table constraint whose text
    -- contains `IS NOT NULL`. `intelligent_org_schema_parity_between_desired_
    -- state_and_migrations` compares CHECK sets across both bootstrap paths.
    CHECK ((status IN ('accepted', 'amended', 'declined')) <> (decided_at IS NULL)),
    CHECK (NOT (status = 'declined' AND decline_reason IS NULL))
);

-- One open draft per gap (§6.1) — the relay's dedupe key.
CREATE UNIQUE INDEX idx_io_drafts_open_gap
    ON io_drafts (community_id, gap)
    WHERE status = 'open';
-- Expiry sweep (§6.3 rule 4) and the inbox's "needs me" source.
CREATE INDEX idx_io_drafts_open_expires
    ON io_drafts (community_id, expires_at)
    WHERE status = 'open' AND expires_at IS NOT NULL;
CREATE INDEX idx_io_drafts_needs
    ON io_drafts (community_id, needs, status);

-- ── Progress notes (kind:50102) ───────────────────────────────────────────────
CREATE TABLE io_progress (
    community_id        UUID NOT NULL REFERENCES communities(id),
    event_id            BYTEA NOT NULL CHECK (length(event_id) = 32),
    item_id             UUID NOT NULL,
    signer              BYTEA NOT NULL CHECK (length(signer) = 32),
    dri                 BYTEA NOT NULL CHECK (length(dri) = 32),
    git_ref             TEXT NOT NULL,
    head                TEXT NOT NULL,
    hint                TEXT NOT NULL CHECK (hint IN ('progressing', 'blocked', 'ready')),
    head_verified       BOOLEAN,
    -- Filled by the push hook when `head` reaches the default branch (D12).
    merged_into         TEXT,
    noted_at            TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (community_id, event_id)
);

CREATE INDEX idx_io_progress_item
    ON io_progress (community_id, item_id, noted_at DESC);

-- ── Health reads (kind:50101) and blind ratings (kind:50017) ─────────────────
CREATE TABLE io_health (
    community_id        UUID NOT NULL REFERENCES communities(id),
    event_id            BYTEA NOT NULL CHECK (length(event_id) = 32),
    item_id             UUID NOT NULL,
    week                TEXT NOT NULL,
    pct                 DOUBLE PRECISION NOT NULL,
    band                TEXT NOT NULL CHECK (band IN ('struggling', 'wobbly', 'healthy')),
    -- Canonical §4.7 content: factors, sentences, formula.
    content             JSONB NOT NULL,
    read_at             TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (community_id, event_id)
);

CREATE INDEX idx_io_health_item
    ON io_health (community_id, item_id, read_at DESC);

CREATE TABLE io_health_ratings (
    community_id        UUID NOT NULL REFERENCES communities(id),
    item_id             UUID NOT NULL,
    week                TEXT NOT NULL,
    rater               BYTEA NOT NULL CHECK (length(rater) = 32),
    band                TEXT NOT NULL CHECK (band IN ('struggling', 'wobbly', 'healthy')),
    receipt_event_id    BYTEA NOT NULL CHECK (length(receipt_event_id) = 32),
    rated_at            TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (community_id, item_id, week, rater)
);

-- ── Org profiles (kind:39105) ─────────────────────────────────────────────────
CREATE TABLE io_profiles (
    community_id        UUID NOT NULL REFERENCES communities(id),
    pubkey              BYTEA NOT NULL CHECK (length(pubkey) = 32),
    version             INTEGER NOT NULL CHECK (version >= 1),
    about               TEXT NOT NULL,
    -- Skill slugs, denormalised from `content.skills[].slug` for the agent's
    -- candidate query (`skills @> ARRAY[...]` / `&&`).
    skills              TEXT[] NOT NULL DEFAULT '{}',
    open_limit          INTEGER CHECK (open_limit IS NULL OR open_limit >= 0),
    -- False once the member leaves (NIP-43 removal); the row stays for history.
    active              BOOLEAN NOT NULL DEFAULT true,
    -- Canonical §4.7a content of the latest 39105.
    content             JSONB NOT NULL,
    event_id            BYTEA NOT NULL CHECK (length(event_id) = 32),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (community_id, pubkey)
);

CREATE INDEX idx_io_profiles_skills
    ON io_profiles USING GIN (skills);

-- ── Ledger — append-only, one row per change (§6.2 verbs) ────────────────────
CREATE TABLE io_ledger (
    community_id        UUID NOT NULL REFERENCES communities(id),
    id                  BIGINT GENERATED ALWAYS AS IDENTITY,
    at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- A pubkey hex, or the literal `relay` for rule-driven changes.
    actor               TEXT NOT NULL,
    verb                TEXT NOT NULL,
    object_type         TEXT NOT NULL,
    object_id           TEXT NOT NULL,
    receipt_event_id    BYTEA CHECK (receipt_event_id IS NULL OR length(receipt_event_id) = 32),
    detail              JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (community_id, id)
);

CREATE INDEX idx_io_ledger_at
    ON io_ledger (community_id, at DESC);
CREATE INDEX idx_io_ledger_object
    ON io_ledger (community_id, object_type, object_id, at DESC);

-- ── Hosted org agents — written by operator provisioning only ────────────────
CREATE TABLE io_hosted_agents (
    community_id        UUID NOT NULL REFERENCES communities(id),
    pubkey              BYTEA NOT NULL CHECK (length(pubkey) = 32),
    provisioned_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    budget              JSONB,
    retired_at          TIMESTAMPTZ,
    PRIMARY KEY (community_id, pubkey)
);

-- At most one live hosted default per community.
CREATE UNIQUE INDEX idx_io_hosted_agents_live
    ON io_hosted_agents (community_id)
    WHERE retired_at IS NULL;

-- ── Universal community write fence ──────────────────────────────────────────
SELECT attach_community_write_fence('io_direction');
SELECT attach_community_write_fence('io_drafts');
SELECT attach_community_write_fence('io_health');
SELECT attach_community_write_fence('io_health_ratings');
SELECT attach_community_write_fence('io_hosted_agents');
SELECT attach_community_write_fence('io_ledger');
SELECT attach_community_write_fence('io_profiles');
SELECT attach_community_write_fence('io_progress');
SELECT attach_community_write_fence('io_proposals');
SELECT attach_community_write_fence('io_shapers');
SELECT attach_community_write_fence('io_votes');
SELECT attach_community_write_fence('io_work_items');
