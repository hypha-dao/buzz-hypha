import assert from "node:assert/strict";
import test from "node:test";

import { directionPageFilters, tallyFilters } from "./overviewExtraFilters.ts";

test("tallyFilters is kinds 50103 t=tally", () => {
  assert.deepEqual(tallyFilters(), [
    { kinds: [50103], "#t": ["tally"], limit: 500 },
  ]);
});

test("directionPageFilters is Protocol §6.5 Direction page, not C-2 50002", () => {
  const filters = directionPageFilters("mission");
  assert.deepEqual(filters, [
    { kinds: [39100], "#d": ["mission"], limit: 500 },
    { kinds: [39102], "#t": ["direction"], "#s": ["passed"], limit: 500 },
  ]);
  assert.ok(!filters.some((filter) => filter.kinds?.includes(50002)));
});
