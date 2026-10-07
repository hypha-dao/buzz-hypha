import assert from "node:assert/strict";
import test from "node:test";

import {
  linesForDirectionPropose,
  presentStrategyDraft,
  strategyLinesForPublish,
} from "./directionLines.ts";

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

test("a period after the type still counts, and the box leaves it out", () => {
  const parsed = linesForDirectionPropose(
    "strategy",
    "No brand money. Type: refusal.",
  );
  assert.deepEqual(parsed, {
    lines: [{ text: "No brand money", type: "refusal" }],
  });
  const presented = presentStrategyDraft(
    [
      "We bet the next task is what makes the app daily. Type: bet.",
      "No brand money. Type: refusal.",
    ],
    [],
  );
  assert.deepEqual(presented, [
    {
      text: "We bet the next task is what makes the app daily",
      type: "bet",
    },
    { text: "No brand money", type: "refusal" },
  ]);
  assert.deepEqual(
    strategyLinesForPublish(
      presented.map((line) => line.text).join("\n"),
      presented.map((line) => line.type),
    ),
    { lines: presented },
  );
});

test("a strategy line with no written type is a bet", () => {
  const parsed = linesForDirectionPropose(
    "strategy",
    "Borrow a hall before we buy one.",
  );
  assert.deepEqual(parsed, {
    lines: [{ text: "Borrow a hall before we buy one.", type: "bet" }],
  });
});

test("a type marker on its own stays with the line before it", () => {
  assert.deepEqual(
    presentStrategyDraft(["No brand money.", "Type: refusal."], []),
    [{ text: "No brand money.", type: "refusal" }],
  );
});
