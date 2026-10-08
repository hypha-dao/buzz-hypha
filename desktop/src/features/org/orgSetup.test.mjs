import assert from "node:assert/strict";
import test from "node:test";

import { parseOrgSetupMessage, runOrgSetup } from "./orgSetup.ts";

const AGENT = "aa".repeat(32);

test("an invite tag from the org agent is the join cap and the agreement count", () => {
  assert.deepEqual(
    parseOrgSetupMessage(
      {
        signerPubkey: AGENT,
        tags: [["io-setup", "invite", "3", "2"]],
      },
      AGENT,
    ),
    { kind: "invite", maxUses: 3, quorum: 2 },
  );
  assert.equal(
    parseOrgSetupMessage(
      { pubkey: "bb".repeat(32), tags: [["io-setup", "invite", "3", "2"]] },
      AGENT,
    ),
    null,
  );
  assert.deepEqual(
    parseOrgSetupMessage(
      { signerPubkey: AGENT, tags: [["io-setup", "alone"]] },
      AGENT,
    ),
    { kind: "alone" },
  );
});

test("setup bootstraps, then sets how many must agree, then mints the invite", async () => {
  const calls = [];
  const result = await runOrgSetup({
    bootstrapped: false,
    setup: { kind: "invite", maxUses: 4, quorum: 2 },
    publishBootstrap: async () => {
      calls.push("bootstrap");
    },
    publishRules: async (need, of) => {
      calls.push(`rules:${need}/${of}`);
    },
    mint: async (maxUses) => {
      calls.push(`mint:${maxUses}`);
      return { url: "http://localhost:3000/invite/abc" };
    },
  });
  assert.deepEqual(calls, ["bootstrap", "rules:2/5", "mint:4"]);
  assert.equal(result.url, "http://localhost:3000/invite/abc");
});

test("a bootstrapped org does not bootstrap again", async () => {
  const calls = [];
  const result = await runOrgSetup({
    bootstrapped: true,
    setup: { kind: "alone" },
    publishBootstrap: async () => {
      calls.push("bootstrap");
    },
    publishRules: async () => {
      calls.push("rules");
    },
    mint: async () => {
      calls.push("mint");
      return { url: "unused" };
    },
  });
  assert.deepEqual(calls, []);
  assert.equal(result.url, null);
});
