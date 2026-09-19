import assert from "node:assert/strict";
import test from "node:test";

import { viewerIsShaper } from "./useMyWorkEvents.ts";

const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const OTHER =
  "0c9a6e2b4d8f1a3c5e7b9d0f2a4c6e8b1d3f5a7c9e0b2d4f6a8c0e1b3d5f7a92";

function shapers(list, createdAt = 10) {
  return {
    id: "1".repeat(64),
    pubkey: "f".repeat(64),
    created_at: createdAt,
    kind: 39103,
    tags: [["d", "shapers"]],
    content: JSON.stringify({ shapers: list }),
    sig: "",
  };
}

test("viewerIsShaper reads the newest 39103 shapers list", () => {
  assert.equal(
    viewerIsShaper([shapers([OTHER], 1), shapers([ME, OTHER], 9)], ME),
    true,
  );
  assert.equal(viewerIsShaper([shapers([OTHER])], ME), false);
  assert.equal(viewerIsShaper([], ME), false);
});
