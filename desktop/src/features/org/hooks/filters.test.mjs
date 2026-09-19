import assert from "node:assert/strict";
import test from "node:test";

import {
  IO_COMMAND_KINDS,
  myWorkFilters,
  overviewFilters,
  workFilters,
  workItemFilters,
} from "./filters.ts";

const ME = "e5ebc6cdb579be112e336cc319b5989b4bb6af11786ea90dbe52b5f08d741b34";
const ITEM = "11111111-1111-4111-8111-111111111111";

test("overviewFilters is the Protocol §6.5 Overview set", () => {
  const filters = overviewFilters();
  assert.deepEqual(
    filters.map((filter) => ({
      kinds: filter.kinds,
      d: filter["#d"],
      t: filter["#t"],
    })),
    [
      { kinds: [39100], d: undefined, t: undefined },
      { kinds: [39103], d: ["shapers"], t: undefined },
      { kinds: [39101], d: undefined, t: ["project"] },
    ],
  );
});

test("workFilters starts as the tree and adds health only when ids exist", () => {
  assert.deepEqual(
    workFilters().map((filter) => filter.kinds),
    [[39101]],
  );
  const withHealth = workFilters([ITEM]);
  assert.equal(withHealth.length, 2);
  assert.deepEqual(withHealth[1].kinds, [50101]);
  assert.deepEqual(withHealth[1]["#i"], [ITEM]);
});

test("workItemFilters is d, u, command trail, work log", () => {
  const filters = workItemFilters(ITEM);
  assert.deepEqual(filters[0], {
    kinds: [39101],
    "#d": [ITEM],
    limit: 500,
  });
  assert.deepEqual(filters[1]["#u"], [ITEM]);
  assert.deepEqual(filters[2].kinds, IO_COMMAND_KINDS);
  assert.equal(IO_COMMAND_KINDS[0], 50001);
  assert.equal(IO_COMMAND_KINDS.at(-1), 50021);
  assert.deepEqual(filters[2]["#i"], [ITEM]);
  assert.deepEqual(filters[3].kinds, [50102]);
});

test("myWorkFilters is p/n/s plus 39103, and the shaper addendum", () => {
  const member = myWorkFilters(ME);
  assert.deepEqual(
    member.map((filter) => filter.kinds[0]),
    [39103, 39101, 50100, 39102],
  );
  assert.deepEqual(member[1]["#p"], [ME]);
  assert.deepEqual(member[2]["#n"], [ME]);
  assert.deepEqual(member[3]["#s"], ["open"]);
  const shaper = myWorkFilters(ME, true);
  assert.equal(shaper.length, 5);
  assert.deepEqual(shaper[4]["#n"], ["shaper"]);
});
