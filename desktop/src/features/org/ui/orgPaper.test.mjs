import assert from "node:assert/strict";
import test from "node:test";

import { ORG_PAPER_CLASS, applyPaperSurface } from "./orgPaper.ts";

test("the paper surface is applied once on the document root", () => {
  const added = [];
  applyPaperSurface({
    classList: {
      add(name) {
        added.push(name);
      },
    },
  });
  assert.deepEqual(added, [ORG_PAPER_CLASS]);
});
