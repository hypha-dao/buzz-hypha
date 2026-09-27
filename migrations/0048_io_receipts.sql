-- R-10: receipt citation index for Protocol §6.8 Receipt read (V6).
--
-- A community member's `{ids:[…]}` REQ may return an event from a room they
-- are not in iff that id is cited as a receipt on a stored community-global
-- io event. This table is the one-SELECT lookup that fills
-- `EventQuery.receipt_ids` so the access-scope SQL can OR those ids in
-- without a second round trip. Written when a `50100` e-receipt, a `50101`
-- health row, or a `50009` done receipt marker is stored; never for
-- `50102.commits` (those are git shas, not event ids).

CREATE TABLE io_receipts (
    community_id        UUID NOT NULL REFERENCES communities(id),
    cited_id            BYTEA NOT NULL CHECK (length(cited_id) = 32),
    citing_id           BYTEA NOT NULL CHECK (length(citing_id) = 32),
    -- How the citation was stored: e_tag | done_receipt | health_row.
    source              TEXT NOT NULL CHECK (source IN ('e_tag', 'done_receipt', 'health_row')),
    PRIMARY KEY (community_id, cited_id, citing_id, source)
);

-- REQ lookup: which of these candidate ids are cited in this community?
CREATE INDEX idx_io_receipts_cited
    ON io_receipts (community_id, cited_id);

SELECT attach_community_write_fence('io_receipts');
