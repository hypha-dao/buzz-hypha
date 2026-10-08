-- The codebases list is a proposal. Passing it writes kind 39106.
-- Strategy (kind 39100) stays a separate artifact.

ALTER TABLE io_proposals DROP CONSTRAINT io_proposals_kind_check;
ALTER TABLE io_proposals ADD CONSTRAINT io_proposals_kind_check
    CHECK (kind IN ('direction', 'project', 'dri', 'shapers', 'money', 'join', 'withdraw', 'codebases'));
