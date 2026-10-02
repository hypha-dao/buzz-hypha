import assert from "node:assert/strict";
import test from "node:test";

import { KIND_IO_PROPOSAL } from "../../shared/constants/kinds.ts";
import {
  passedProposalLinks,
  proposalChatLink,
  proposalDestination,
} from "./proposalChat.ts";

const PROPOSAL = "33333333-3333-4333-8333-333333333333";
const ITEM = "44444444-4444-4444-8444-444444444444";

function event(content, tags, created_at = 10) {
  return {
    id: `e-${created_at}`,
    kind: KIND_IO_PROPOSAL,
    content: JSON.stringify(content),
    created_at,
    tags,
  };
}

test("an open direction proposal opens that direction page", () => {
  const link = proposalChatLink(
    event(
      {
        kind: "direction",
        status: "open",
        payload: { slug: "mission", body: "We host the hall." },
      },
      [
        ["d", PROPOSAL],
        ["t", "direction"],
        ["s", "open"],
      ],
    ),
  );
  assert.equal(link.kind, "direction");
  assert.equal(link.slug, "mission");
  assert.equal(link.label, "Open mission");
});

test("a passed project proposal opens the work item", () => {
  const link = proposalChatLink(
    event(
      {
        kind: "project",
        status: "passed",
        payload: { title: "Weekday hall" },
        executed: { kind: "work_item", id: ITEM },
      },
      [
        ["d", PROPOSAL],
        ["t", "project"],
        ["s", "passed"],
      ],
    ),
  );
  assert.equal(link.kind, "project");
  assert.equal(link.itemId, ITEM);
  assert.equal(link.label, "Open Weekday hall");
  assert.deepEqual(
    proposalDestination(
      event(
        {
          kind: "project",
          status: "passed",
          payload: { title: "Weekday hall" },
          executed: { kind: "work_item", id: ITEM },
        },
        [
          ["d", PROPOSAL],
          ["t", "project"],
          ["s", "passed"],
        ],
      ),
    ),
    { to: "/org/work/$itemId", params: { itemId: ITEM } },
  );
  assert.equal(
    proposalChatLink(
      event(
        {
          kind: "project",
          status: "open",
          payload: { title: "Weekday hall" },
        },
        [
          ["d", PROPOSAL],
          ["s", "open"],
        ],
      ),
    ),
    null,
  );
  assert.deepEqual(
    proposalDestination(
      event(
        {
          kind: "project",
          status: "open",
          payload: { title: "Weekday hall", brief: "Open the hall." },
        },
        [
          ["d", PROPOSAL],
          ["t", "project"],
          ["s", "open"],
        ],
      ),
    ),
    { to: "/org/proposal/$proposalId", params: { proposalId: PROPOSAL } },
  );
});

test("passed project links keep the newest head", () => {
  const links = passedProposalLinks([
    event(
      {
        kind: "project",
        status: "open",
        payload: { title: "Weekday hall" },
      },
      [
        ["d", PROPOSAL],
        ["s", "open"],
      ],
      1,
    ),
    event(
      {
        kind: "project",
        status: "passed",
        payload: { title: "Weekday hall" },
        executed: { id: ITEM },
      },
      [
        ["d", PROPOSAL],
        ["s", "passed"],
      ],
      2,
    ),
  ]);
  assert.equal(links.length, 1);
  assert.equal(links[0].itemId, ITEM);
});
