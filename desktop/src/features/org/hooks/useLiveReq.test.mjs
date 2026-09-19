import assert from "node:assert/strict";
import test from "node:test";

import { loadOverlapping } from "./useLiveReq.ts";

function event(id, createdAt = 1) {
  return {
    id,
    pubkey: "a".repeat(64),
    created_at: createdAt,
    kind: 39100,
    tags: [],
    content: "{}",
    sig: "",
  };
}

test("loadOverlapping subscribes live before fetching history", async () => {
  const order = [];
  let onLive;
  const client = {
    subscribeLive: async (_filter, onEvent) => {
      order.push("subscribe");
      onLive = onEvent;
      return () => {
        order.push("unsubscribe");
      };
    },
    fetchEvents: async () => {
      order.push("fetch");
      return [event("history")];
    },
  };

  const result = await loadOverlapping(
    [{ kinds: [39100], limit: 500 }],
    client,
  );
  assert.deepEqual(order, ["subscribe", "fetch"]);
  assert.equal(
    result.events.some((item) => item.id === "history"),
    true,
  );
  onLive(event("live-after"));
  result.unsubscribe();
});

test("an event that arrives between subscribe and fetch is kept", async () => {
  let resolveFetch;
  let onLive;
  const client = {
    subscribeLive: async (_filter, onEvent) => {
      onLive = onEvent;
      return () => {};
    },
    fetchEvents: () =>
      new Promise((resolve) => {
        resolveFetch = resolve;
      }),
  };

  const pending = loadOverlapping([{ kinds: [39100], limit: 500 }], client);
  await Promise.resolve();
  assert.equal(typeof onLive, "function");
  onLive(event("gap"));
  resolveFetch([event("history")]);
  const result = await pending;
  const ids = result.events.map((item) => item.id).sort();
  assert.deepEqual(ids, ["gap", "history"]);
});

test("live and history events with the same id become one event", async () => {
  const liveCopy = event("same", 20);
  const client = {
    subscribeLive: async (_filter, onEvent) => {
      onEvent(liveCopy);
      return () => {};
    },
    fetchEvents: async () => [event("same", 10)],
  };
  const result = await loadOverlapping(
    [{ kinds: [39100], limit: 500 }],
    client,
  );
  assert.equal(result.events.length, 1);
  assert.equal(result.events[0].created_at, 20);
});

test("loadOverlapping unsubscribes live REQs when history fetch fails", async () => {
  const order = [];
  const client = {
    subscribeLive: async () => {
      order.push("subscribe");
      return () => {
        order.push("unsubscribe");
      };
    },
    fetchEvents: async () => {
      order.push("fetch");
      throw new Error("relay unavailable");
    },
  };

  await assert.rejects(
    () => loadOverlapping([{ kinds: [39100], limit: 500 }], client),
    /relay unavailable/,
  );
  assert.deepEqual(order, ["subscribe", "fetch", "unsubscribe"]);
});
