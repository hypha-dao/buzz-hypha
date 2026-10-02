import assert from "node:assert/strict";
import test from "node:test";

import {
  overviewCardDelayMs,
  overviewLineDelayMs,
  overviewMarkDelayMs,
  overviewRuleDelayMs,
  overviewStampDelayMs,
} from "./overviewMotion.ts";

test("overview cards settle, then write each line, then stamp the footer", () => {
  assert.equal(overviewCardDelayMs(0), 0);
  assert.equal(overviewCardDelayMs(1), 140);
  assert.equal(overviewCardDelayMs(3), 420);

  assert.equal(overviewMarkDelayMs(1), 260);
  assert.equal(overviewRuleDelayMs(1), 320);

  assert.equal(overviewLineDelayMs(0, 0), 320);
  assert.equal(overviewLineDelayMs(0, 1), 490);
  assert.equal(overviewLineDelayMs(2, 0), 600);

  assert.equal(overviewStampDelayMs(0, 1), 710);
  assert.equal(overviewStampDelayMs(0, 2), 880);
});
