import assert from "node:assert/strict";
import test from "node:test";

import {
  ORG_ONBOARDING_COPY,
  orgOnboardingStorageKey,
  readOrgOnboardingStage,
  writeOrgOnboardingStage,
} from "./orgOnboarding.ts";

test("org onboarding copy names Shapers like a board", () => {
  assert.match(ORG_ONBOARDING_COPY.shaperExplain, /board/i);
  assert.match(ORG_ONBOARDING_COPY.aloneNext, /50001/);
  assert.match(ORG_ONBOARDING_COPY.othersNext, /#shapers/);
});

test("orgOnboardingStorageKey is scoped per community and pubkey", () => {
  const key = orgOnboardingStorageKey("abcd", "ws://localhost:3000");
  assert.match(key, /buzz-org-onboarding\.v1/);
  assert.match(key, /abcd/);
});

test("write/read org onboarding stage round-trips in localStorage", () => {
  const store = new Map();
  globalThis.window = {
    localStorage: {
      getItem: (k) => store.get(k) ?? null,
      setItem: (k, v) => {
        store.set(k, v);
      },
    },
  };
  writeOrgOnboardingStage("pub", "ws://localhost:3000", "alone");
  assert.equal(readOrgOnboardingStage("pub", "ws://localhost:3000"), "alone");
  assert.equal(readOrgOnboardingStage(null, "ws://localhost:3000"), null);
});
