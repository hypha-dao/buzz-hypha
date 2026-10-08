import assert from "node:assert/strict";
import test from "node:test";

import {
  agreedOfferItemIds,
  claimAgreedOffer,
  resetAgreedOfferAttempts,
} from "./agreedOffer.ts";

const ME = "deadbeef".repeat(8);
const ALICE =
  "953d3363262e86b770419834c53d2446409db6d918a57f8f339d495d54ab001f";
const ITEM = "11111111-1111-4111-8111-111111111111";
const PROPOSAL = "66666666-6666-4666-8666-666666666666";

function proposal(votes, status = "passed") {
  return {
    id: "aa".repeat(32),
    pubkey: ME,
    kind: 39102,
    created_at: 20,
    content: JSON.stringify({
      kind: "project",
      status,
      votes,
      executed: { id: ITEM, kind: "work_item" },
    }),
    tags: [
      ["d", PROPOSAL],
      ["t", "project"],
      ["s", status],
    ],
  };
}

function offer(offeredTo = ME) {
  return {
    id: "bb".repeat(32),
    pubkey: "f".repeat(64),
    kind: 39101,
    created_at: 30,
    content: JSON.stringify({
      title: "make hypha great",
      state: "offered",
      offered_to: offeredTo,
    }),
    tags: [
      ["d", ITEM],
      ["s", "offered"],
      ["t", "project"],
      ["p", offeredTo, "", "offered"],
    ],
  };
}

test("an agreed project offer is the viewer's to accept once", () => {
  const ids = agreedOfferItemIds(
    [proposal([{ p: ME, vote: "agree" }]), offer()],
    ME,
  );
  assert.deepEqual(ids, [ITEM]);
});

test("someone else's agree does not accept the offer for the viewer", () => {
  const ids = agreedOfferItemIds(
    [proposal([{ p: ALICE, vote: "agree" }]), offer(ME)],
    ME,
  );
  assert.deepEqual(ids, []);
});

test("an offer to someone else stays theirs", () => {
  const ids = agreedOfferItemIds(
    [proposal([{ p: ME, vote: "agree" }]), offer(ALICE)],
    ME,
  );
  assert.deepEqual(ids, []);
});

test("an open proposal has not accepted the project", () => {
  const ids = agreedOfferItemIds(
    [proposal([{ p: ME, vote: "agree" }], "open"), offer()],
    ME,
  );
  assert.deepEqual(ids, []);
});

test("a held project is not offered again", () => {
  const held = {
    ...offer(),
    content: JSON.stringify({
      title: "make hypha great",
      state: "accepted",
      dri: ME,
    }),
    tags: [
      ["d", ITEM],
      ["s", "accepted"],
      ["t", "project"],
      ["p", ME],
    ],
  };
  assert.deepEqual(
    agreedOfferItemIds([proposal([{ p: ME, vote: "agree" }]), held], ME),
    [],
  );
});

test("claiming an agreed offer happens once until the community resets", () => {
  resetAgreedOfferAttempts();
  assert.equal(claimAgreedOffer(ITEM), true);
  assert.equal(claimAgreedOffer(ITEM), false);
  resetAgreedOfferAttempts();
  assert.equal(claimAgreedOffer(ITEM), true);
  resetAgreedOfferAttempts();
});
