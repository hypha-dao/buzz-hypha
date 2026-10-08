-- Withdrawal: a project or ticket leaves the live board. The events stay.
--
-- `io_work_items.state` gains `withdrawn`. `io_proposals.kind` gains
-- `withdraw`, the proposal a Shaper opens when more than one Shaper is seated.

ALTER TABLE io_work_items DROP CONSTRAINT io_work_items_state_check;
ALTER TABLE io_work_items ADD CONSTRAINT io_work_items_state_check
    CHECK (state IN ('open', 'offered', 'accepted', 'in_review', 'done', 'withdrawn'));

ALTER TABLE io_proposals DROP CONSTRAINT io_proposals_kind_check;
ALTER TABLE io_proposals ADD CONSTRAINT io_proposals_kind_check
    CHECK (kind IN ('direction', 'project', 'dri', 'shapers', 'money', 'join', 'withdraw'));
