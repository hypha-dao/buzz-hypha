-- R-6: multi-pod claim rows for `io_scheduler` transitions (Protocol §6.3, V9).
--
-- Each transition is one transaction that first inserts a claim keyed by
-- (community, kind, object, epoch). A concurrent pod's INSERT hits the PK
-- and does nothing — the same pattern as `scheduled_workflow_fires`. The
-- epoch is the timestamp the rule fires on (offered_at, due_at, expires_at,
-- seat.at), so a moved `due_at` or a fresh offer/seat is a new claim.

CREATE TABLE io_scheduler_claims (
    community_id        UUID NOT NULL REFERENCES communities(id),
    kind                TEXT NOT NULL CHECK (kind IN (
        'offer_renotify',
        'offer_expire',
        'enter_review',
        'close_due',
        'draft_expire',
        'proposal_expire',
        'seat_lapse'
    )),
    -- Work-item / proposal uuid, draft event-id hex, or `p:proposal` for seats.
    object_id           TEXT NOT NULL,
    epoch               TIMESTAMPTZ NOT NULL,
    claimed_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (community_id, kind, object_id, epoch)
);

CREATE INDEX idx_io_scheduler_claims_claimed_at
    ON io_scheduler_claims (claimed_at);

SELECT attach_community_write_fence('io_scheduler_claims');
