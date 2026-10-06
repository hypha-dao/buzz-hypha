import assert from "node:assert/strict";
import test from "node:test";

import { linesForDirectionPropose } from "./directionLines.ts";

test("a vague objective is not a proposal line", () => {
  const parsed = linesForDirectionPropose("objectives", "be more visible");
  assert.equal("error" in parsed, true);
});

test("an objective line keeps its date and done when", () => {
  const parsed = linesForDirectionPropose(
    "objectives",
    "A weekday hall is open. Done when: the hall has hosted one paid night. By: 2026-06-01",
  );
  assert.deepEqual(parsed, {
    lines: [
      {
        text: "A weekday hall is open",
        done_when: "the hall has hosted one paid night",
        date: Date.parse("2026-06-01T00:00:00Z") / 1000,
      },
    ],
  });
});

test("a strategy line keeps its type", () => {
  const parsed = linesForDirectionPropose(
    "strategy",
    "No brand money. Type: refusal",
  );
  assert.deepEqual(parsed, {
    lines: [{ text: "No brand money", type: "refusal" }],
  });
});
