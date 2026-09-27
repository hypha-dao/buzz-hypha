#!/usr/bin/env node
/**
 * Unit coverage for scripts/check-org-staging-prep.sh invariants that are
 * pure JSON / path presence — so a regression on org.defaultEnabled fails
 * without needing the shell wrapper.
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

describe("O-2 staging prep invariants", () => {
  it("org is defaultEnabled for the team dogfood build", () => {
    const manifest = JSON.parse(
      readFileSync(path.join(root, "preview-features.json"), "utf8"),
    );
    const org = manifest.features.find((f) => f.id === "org");
    assert.ok(org, "org feature must exist");
    assert.equal(org.defaultEnabled, true);
  });

  it("R-wave migrations 0045 and 0046 are present", () => {
    assert.ok(
      existsSync(path.join(root, "migrations/0045_intelligent_org.sql")),
    );
    assert.ok(
      existsSync(
        path.join(
          root,
          "migrations/0046_dm_roster_fence_excludes_org_agent.sql",
        ),
      ),
    );
  });

  it("operator runbook and smoke/prep scripts exist", () => {
    assert.ok(
      existsSync(
        path.join(
          root,
          "docs/intelligent-org/plans/intelligent-org-o2-staging-deploy.md",
        ),
      ),
    );
    assert.ok(existsSync(path.join(root, "scripts/check-org-staging-prep.sh")));
    assert.ok(existsSync(path.join(root, "scripts/org-staging-smoke.sh")));
  });
});
