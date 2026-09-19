import assert from "node:assert/strict";
import test from "node:test";

import { deriveShellRoute } from "./AppShell.helpers.ts";

test("deriveShellRoute treats every /org path as the org view", () => {
  assert.deepEqual(deriveShellRoute("/org"), {
    selectedChannelId: null,
    selectedView: "org",
  });
  assert.deepEqual(deriveShellRoute("/org/work"), {
    selectedChannelId: null,
    selectedView: "org",
  });
  assert.deepEqual(
    deriveShellRoute("/org/work/11111111-1111-4111-8111-111111111111"),
    {
      selectedChannelId: null,
      selectedView: "org",
    },
  );
  assert.deepEqual(deriveShellRoute("/org/my-work"), {
    selectedChannelId: null,
    selectedView: "org",
  });
});

test("deriveShellRoute does not steal Inbox for other paths", () => {
  assert.equal(deriveShellRoute("/").selectedView, "home");
  assert.equal(deriveShellRoute("/agents").selectedView, "agents");
});
