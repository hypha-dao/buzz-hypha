import assert from "node:assert/strict";
import test from "node:test";

import {
  holdDoorSnapshot,
  loadOverlapping,
  readSharedDoor,
  resetLiveDoorEvents,
  subscribeSharedDoor,
} from "./useLiveReq.ts";

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

test("an in-flight filter change keeps the events the next filter was derived from", () => {
  const previous = { events: [event("item")], isLoading: false };
  const loading = { events: [], isLoading: true };
  const held = holdDoorSnapshot(previous, 1, loading, 1);
  assert.equal(held.view.isLoading, true);
  assert.equal(held.view.events[0].id, "item");

  const arrived = {
    events: [event("item"), event("health")],
    isLoading: false,
  };
  const done = holdDoorSnapshot(held.held, held.heldEpoch, arrived, 1);
  assert.equal(done.view, arrived);

  const reset = holdDoorSnapshot(previous, 1, loading, 2);
  assert.deepEqual(reset.view.events, []);
  assert.equal(reset.view.isLoading, true);
});

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
  for (
    let attempt = 0;
    attempt < 20 && typeof resolveFetch !== "function";
    attempt += 1
  ) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
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

test("loadOverlapping subscribes to every filter before any history fetch", async () => {
  let fetches = 0;
  let subscribes = 0;
  const client = {
    subscribeLive: async () => {
      assert.equal(fetches, 0);
      subscribes += 1;
      await Promise.resolve();
      return () => {};
    },
    fetchEvents: async () => {
      assert.equal(subscribes, 2);
      fetches += 1;
      return [];
    },
  };

  await loadOverlapping(
    [
      { kinds: [39100], limit: 500 },
      { kinds: [39101], limit: 500 },
    ],
    client,
  );
  assert.equal(fetches, 2);
});

test("each history page is reported before the slower filter returns", async () => {
  let releaseProject;
  const seen = [];
  const client = {
    subscribeLive: async () => () => {},
    fetchEvents: (filter) => {
      if (filter.kinds[0] === 39100)
        return Promise.resolve([event("direction")]);
      return new Promise((resolve) => {
        releaseProject = () => resolve([event("project")]);
      });
    },
  };

  const pending = loadOverlapping(
    [
      { kinds: [39100], limit: 500 },
      { kinds: [39101], limit: 500 },
    ],
    client,
    undefined,
    (page) => {
      seen.push(page.map((item) => item.id));
    },
  );
  for (let attempt = 0; attempt < 20 && seen.length === 0; attempt += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  assert.deepEqual(seen, [["direction"]]);
  releaseProject();
  const result = await pending;
  assert.deepEqual(result.events.map((item) => item.id).sort(), [
    "direction",
    "project",
  ]);
});

test("one filter set is fetched once for every subscriber", async () => {
  resetLiveDoorEvents();
  let fetches = 0;
  const client = {
    subscribeLive: async () => () => {},
    fetchEvents: async () => {
      fetches += 1;
      return [event("shared")];
    },
  };
  const filters = [{ kinds: [39100], limit: 500 }];
  const unsubscribeA = subscribeSharedDoor(filters, () => {}, client);
  const unsubscribeB = subscribeSharedDoor(filters, () => {}, client);

  try {
    for (
      let attempt = 0;
      attempt < 20 && readSharedDoor(filters).isLoading;
      attempt += 1
    ) {
      await new Promise((resolve) => setTimeout(resolve, 0));
    }

    assert.equal(fetches, 1);
    assert.equal(readSharedDoor(filters).isLoading, false);
    assert.equal(readSharedDoor(filters).events[0]?.id, "shared");
  } finally {
    unsubscribeA();
    unsubscribeB();
    resetLiveDoorEvents();
  }
  assert.deepEqual(readSharedDoor(filters).events, []);
});

test("a shared door paints the first history page before the rest finish", async () => {
  resetLiveDoorEvents();
  let releaseProject;
  const client = {
    subscribeLive: async () => () => {},
    fetchEvents: (filter) => {
      if (filter.kinds[0] === 39100) {
        return Promise.resolve([event("direction")]);
      }
      return new Promise((resolve) => {
        releaseProject = () => resolve([event("project")]);
      });
    },
  };
  const filters = [
    { kinds: [39100], limit: 500 },
    { kinds: [39101], limit: 500 },
  ];
  const unsubscribe = subscribeSharedDoor(filters, () => {}, client);

  try {
    for (let attempt = 0; attempt < 20; attempt += 1) {
      const snapshot = readSharedDoor(filters);
      if (snapshot.events.some((item) => item.id === "direction")) break;
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
    const midway = readSharedDoor(filters);
    assert.equal(midway.isLoading, true);
    assert.equal(
      midway.events.some((item) => item.id === "direction"),
      true,
    );
    assert.equal(
      midway.events.some((item) => item.id === "project"),
      false,
    );
    releaseProject();
    for (
      let attempt = 0;
      attempt < 20 && readSharedDoor(filters).isLoading;
      attempt += 1
    ) {
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
    assert.equal(readSharedDoor(filters).isLoading, false);
    assert.equal(readSharedDoor(filters).events.length, 2);
  } finally {
    unsubscribe();
    resetLiveDoorEvents();
  }
});
