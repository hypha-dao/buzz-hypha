-- Situation: where the org stands today, a fifth direction artifact.
--
-- `io_direction.slug` gains `situation`. Like mission and vision it has no
-- lines.

ALTER TABLE io_direction DROP CONSTRAINT io_direction_slug_check;
ALTER TABLE io_direction ADD CONSTRAINT io_direction_slug_check
    CHECK (slug IN ('mission', 'vision', 'situation', 'objectives', 'strategy'));
