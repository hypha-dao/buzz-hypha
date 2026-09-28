#!/usr/bin/env bash
# Intelligent-org O-2 — in-repo prerequisites for staging deploy (no cluster).
#
# Proves the repo is ready for an operator to roll the staging relay and a
# team desktop with the `org` gate on. Does NOT talk to staging. Live smoke
# is scripts/org-staging-smoke.sh (needs credentials).
#
# Checks:
#   1. Migrations 0045 (io_* tables) and 0046 (DM roster fence) exist
#   2. preview-features.json has org.defaultEnabled === true
#   3. O-1 provision script + admin writer are present
#   4. Deploy runbook is present
#
# Exit 0 = prep green. Exit 1 = repo not ready for O-2.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { echo "OK: $*"; }

# ── 1. R-wave migrations ────────────────────────────────────────────────────
[[ -f migrations/0045_intelligent_org.sql ]] \
  || fail "missing migrations/0045_intelligent_org.sql (R-2a)"
[[ -f migrations/0046_dm_roster_fence_excludes_org_agent.sql ]] \
  || fail "missing migrations/0046_dm_roster_fence_excludes_org_agent.sql (R-8)"
ok "migrations 0045 and 0046 present"

# Embedded migration list must still name them (sqlx::migrate! picks up the dir).
grep -q 'io_hosted_agents' migrations/0045_intelligent_org.sql \
  || fail "0045 does not create io_hosted_agents"
ok "0045 defines io_hosted_agents"

# ── 2. Desktop org gate default-on for the team ─────────────────────────────
node --input-type=module -e '
import { readFileSync } from "node:fs";
const manifest = JSON.parse(readFileSync("preview-features.json", "utf8"));
const org = manifest.features?.find((f) => f.id === "org");
if (!org) {
  console.error("FAIL: org feature missing from preview-features.json");
  process.exit(1);
}
if (org.defaultEnabled !== true) {
  console.error(
    "FAIL: org.defaultEnabled must be true for O-2 team dogfood (got " +
      JSON.stringify(org.defaultEnabled) +
      ")",
  );
  process.exit(1);
}
console.log("OK: org.defaultEnabled === true");
'

# ── 3. O-1 writer path still on main ────────────────────────────────────────
[[ -x scripts/org-agent-provision.sh ]] \
  || fail "scripts/org-agent-provision.sh missing or not executable (O-1)"
[[ -f crates/buzz-admin/src/org.rs ]] \
  || fail "crates/buzz-admin/src/org.rs missing (O-1 hosted-agent writer)"
ok "O-1 provision script and buzz-admin org writer present"

# ── 4. Operator runbook ─────────────────────────────────────────────────────
[[ -f docs/intelligent-org/plans/intelligent-org-o2-staging-deploy.md ]] \
  || fail "missing docs/intelligent-org/plans/intelligent-org-o2-staging-deploy.md"
ok "O-2 staging deploy runbook present"

echo
echo "org-staging-prep: all in-repo checks passed."
echo "Live deploy + smoke still need staging credentials — see the runbook."
