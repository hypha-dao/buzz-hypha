import assert from "node:assert/strict";
import test from "node:test";

import {
  KIND_IO_ACCEPT,
  KIND_IO_DECLINE,
  KIND_IO_SHAPER_ACCEPT,
  KIND_IO_VOTE,
} from "../../shared/constants/kinds.ts";
import { afterOrgCommandPublished } from "./channelListRefresh.ts";

function spy() {
  const fn = () => {
    fn.calls++;
  };
  fn.calls = 0;
  return fn;
}

test("accepting work refreshes the sidebar channel list", () => {
  const refresh = spy();
  afterOrgCommandPublished(KIND_IO_ACCEPT, refresh);
  assert.equal(refresh.calls, 1);
});

test("a passing vote refreshes the sidebar channel list", () => {
  const refresh = spy();
  afterOrgCommandPublished(KIND_IO_VOTE, refresh);
  assert.equal(refresh.calls, 1);
});

test("accepting a Shaper seat refreshes the sidebar channel list", () => {
  const refresh = spy();
  afterOrgCommandPublished(KIND_IO_SHAPER_ACCEPT, refresh);
  assert.equal(refresh.calls, 1);
});

test("declining work does not refresh the sidebar channel list", () => {
  const refresh = spy();
  afterOrgCommandPublished(KIND_IO_DECLINE, refresh);
  assert.equal(refresh.calls, 0);
});
