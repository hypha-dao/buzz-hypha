#!/usr/bin/env bash
# Intelligent-org O-2 — live staging smoke (Phase 0 § Deploy prove).
#
#   scripts/org-staging-smoke.sh
#
# Proves (plan O-2):
#   1. Owner can `buzz org bootstrap` on the target community
#   2. `buzz org shapers list` returns a kind:39103 (visible to Overview)
#
# Required env (never invent placeholders — exit 2 when missing):
#   BUZZ_RELAY_URL      staging websocket or http base (e.g. wss://…)
#   BUZZ_PRIVATE_KEY    community owner hex or nsec
# Optional:
#   BUZZ_AUTH_TAG       NIP-42 auth tag when the relay requires it
#   BUZZ_BIN            path to buzz CLI (default: target/release/buzz or cargo run)
#   ORG_SMOKE_WHY       bootstrap why text (default includes a timestamp)
#
# Exit codes:
#   0  smoke passed
#   1  smoke failed (relay refused, unexpected shape, CLI error)
#   2  credentials / tooling missing — do not treat as a product failure
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

need_creds() {
  echo "BLOCKED: $*" >&2
  echo "BLOCKED: O-2 live smoke needs staging credentials — see" >&2
  echo "  docs/intelligent-org/plans/intelligent-org-o2-staging-deploy.md" >&2
  exit 2
}

fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { echo "OK: $*"; }

if [[ -f .env ]]; then
  set -o allexport
  # shellcheck disable=SC1091
  source .env
  set +o allexport
fi

[[ -n "${BUZZ_RELAY_URL:-}" ]] || need_creds "BUZZ_RELAY_URL is unset"
[[ -n "${BUZZ_PRIVATE_KEY:-}" ]] || need_creds "BUZZ_PRIVATE_KEY is unset"

resolve_buzz() {
  if [[ -n "${BUZZ_BIN:-}" && -x "${BUZZ_BIN}" ]]; then
    printf '%s' "${BUZZ_BIN}"
    return
  fi
  if [[ -x "${REPO_ROOT}/target/release/buzz" ]]; then
    printf '%s' "${REPO_ROOT}/target/release/buzz"
    return
  fi
  if [[ -x "${REPO_ROOT}/target/debug/buzz" ]]; then
    printf '%s' "${REPO_ROOT}/target/debug/buzz"
    return
  fi
  need_creds "buzz CLI binary not found (build with: cargo build -p buzz-cli --release)"
}

BUZZ="$(resolve_buzz)"
WHY="${ORG_SMOKE_WHY:-o2-staging-smoke $(date -u +%Y%m%dT%H%M%SZ)}"

echo "org-staging-smoke: relay=${BUZZ_RELAY_URL}"
echo "org-staging-smoke: buzz=${BUZZ}"

# Probe HTTP reachability (Cloudflare Access often returns HTML 302/403).
# Follow redirects so the Access login HTML is what we inspect — a bare 302
# body is just "302 Found" and would otherwise look like success.
RELAY_HTTP="${BUZZ_RELAY_URL/ws:/http:}"
RELAY_HTTP="${RELAY_HTTP/wss:/https:}"
HTTP_CODE="$(curl -sS -L -o /tmp/org-staging-smoke-http.body -w '%{http_code}' \
  --connect-timeout 10 --max-time 20 \
  -H 'Accept: application/nostr+json' \
  "${RELAY_HTTP}/" 2>/dev/null || true)"
if [[ -z "${HTTP_CODE}" || "${HTTP_CODE}" == "000" ]]; then
  need_creds "relay not reachable at ${RELAY_HTTP} (network or DNS)"
fi
if grep -qiE 'cloudflareaccess|cloudflare access|Sign in ・ Cloudflare|login\.block\.xyz' \
  /tmp/org-staging-smoke-http.body 2>/dev/null; then
  need_creds "relay at ${RELAY_HTTP} is behind Cloudflare Access — SSO/token required"
fi
# NIP-11 should be JSON. HTML at 200 means we got a portal, not the relay.
if head -c 1 /tmp/org-staging-smoke-http.body 2>/dev/null | grep -q '<'; then
  need_creds "relay at ${RELAY_HTTP} returned HTML (Access or wrong host), not NIP-11 JSON"
fi
case "${HTTP_CODE}" in
  2*) ok "relay HTTP ${HTTP_CODE} at ${RELAY_HTTP}" ;;
  401|403) need_creds "relay HTTP ${HTTP_CODE} — auth required before bootstrap" ;;
  *) fail "relay HTTP ${HTTP_CODE} at ${RELAY_HTTP}" ;;
esac

BOOT_JSON="$("${BUZZ}" org bootstrap --why "${WHY}" 2>/tmp/org-staging-smoke-boot.err)" \
  || fail "buzz org bootstrap failed: $(tr '\n' ' ' </tmp/org-staging-smoke-boot.err)"
echo "${BOOT_JSON}" | node --input-type=module -e '
const chunks = [];
for await (const c of process.stdin) chunks.push(c);
const text = Buffer.concat(chunks).toString("utf8").trim();
let j;
try { j = JSON.parse(text); } catch (e) {
  console.error("FAIL: bootstrap output is not JSON:", text.slice(0, 200));
  process.exit(1);
}
if (j.accepted !== true && j.accepted !== "true") {
  // duplicate in the same second is acceptable if a prior smoke already ran —
  // caller should vary ORG_SMOKE_WHY. Still surface the message.
  if (typeof j.message === "string" && /duplicate/i.test(j.message)) {
    console.log("OK: bootstrap duplicate (already processed) — continuing to shapers list");
    process.exit(0);
  }
  console.error("FAIL: bootstrap not accepted:", text.slice(0, 400));
  process.exit(1);
}
if (!j.event_id) {
  console.error("FAIL: bootstrap missing event_id:", text.slice(0, 400));
  process.exit(1);
}
console.log("OK: bootstrap accepted event_id=" + j.event_id);
'

SHAPERS_JSON="$("${BUZZ}" org shapers list 2>/tmp/org-staging-smoke-list.err)" \
  || fail "buzz org shapers list failed: $(tr '\n' ' ' </tmp/org-staging-smoke-list.err)"
echo "${SHAPERS_JSON}" | node --input-type=module -e '
const chunks = [];
for await (const c of process.stdin) chunks.push(c);
const text = Buffer.concat(chunks).toString("utf8").trim();
let events;
try { events = JSON.parse(text); } catch (e) {
  console.error("FAIL: shapers list is not JSON:", text.slice(0, 200));
  process.exit(1);
}
if (!Array.isArray(events)) {
  console.error("FAIL: shapers list is not an array:", text.slice(0, 200));
  process.exit(1);
}
const head = events.find((e) => e && e.kind === 39103);
if (!head) {
  console.error("FAIL: no kind:39103 in shapers list (Overview would be empty)");
  process.exit(1);
}
console.log("OK: kind:39103 id=" + head.id + " (Overview can render Shapers)");
'

echo
echo "org-staging-smoke: PASS"
echo "Manual desktop check: open the team build (org gate on), Overview → Shapers card."
