#!/usr/bin/env bash
# Named O-1 prove: provision against a local relay, twice.
#
# Requires the progress-log local relay recipe:
#   BUZZ_GIT_CONFORMANCE_PROBE=false BUZZ_AUTO_MIGRATE=true cargo run -p buzz-relay
#   (Postgres + Redis up; .env from .env.example + ensure-local-relay-key)
#
# Proves:
#   - io_hosted_agents has one live row for the community
#   - that pubkey is a NIP-43 member (relay_members)
#   - its kind:0 name is "Org agent"
#   - a second run keeps the same pubkey / same row (no duplicate member)
#   - stdout/stderr never contain the secret
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

if [[ -f .env ]]; then
  set -o allexport
  # shellcheck disable=SC1091
  source .env
  set +o allexport
fi

export PGHOST="${PGHOST:-localhost}"
export PGPORT="${PGPORT:-5432}"
export PGUSER="${PGUSER:-buzz}"
export PGPASSWORD="${PGPASSWORD:-buzz_dev}"
export PGDATABASE="${PGDATABASE:-buzz}"
export DATABASE_URL="${DATABASE_URL:-postgres://buzz:buzz_dev@localhost:5432/buzz}"
export RELAY_URL="${RELAY_URL:-ws://localhost:3000}"
export REDIS_URL="${REDIS_URL:-redis://localhost:6379}"

fail() { echo "FAIL: $*" >&2; exit 1; }

command -v psql >/dev/null || fail "psql not on PATH (progress-log Postgres recipe)"

if [[ -z "${BUZZ_RELAY_PRIVATE_KEY:-}" ]]; then
  fail "BUZZ_RELAY_PRIVATE_KEY is unset (run scripts/ensure-local-relay-key.sh .env)"
fi

# Relay must be reachable — this is the named local-relay prove, not a mock.
RELAY_HTTP="${RELAY_URL/ws:/http:}"
RELAY_HTTP="${RELAY_HTTP/wss:/https:}"
if ! curl -fsS --max-time 3 "${RELAY_HTTP}" >/dev/null 2>&1; then
  fail "relay not reachable at ${RELAY_HTTP} — start it with BUZZ_GIT_CONFORMANCE_PROBE=false"
fi

STAMP="$(date +%s)-$$"
COMMUNITY="o1-${STAMP}.localhost:3000"
STORE="$(mktemp -d "${TMPDIR:-/tmp}/o1-store.XXXXXX")"
LOG1="$(mktemp "${TMPDIR:-/tmp}/o1-run1.XXXXXX")"
LOG2="$(mktemp "${TMPDIR:-/tmp}/o1-run2.XXXXXX")"
cleanup() {
  rm -rf "${STORE}" "${LOG1}" "${LOG2}"
}
trap cleanup EXIT

export ORG_AGENT_STORE="${STORE}"

COMMUNITY_ID="$(psql -Atqc \
  "INSERT INTO communities (host) VALUES ('${COMMUNITY}') RETURNING id;")"
[[ -n "${COMMUNITY_ID}" ]] || fail "could not insert test community"

sql() { psql -Atqc "$1"; }

echo "community ${COMMUNITY} id=${COMMUNITY_ID}"

"${SCRIPT_DIR}/org-agent-provision.sh" "${COMMUNITY}" >"${LOG1}" 2>&1 || {
  cat "${LOG1}" >&2
  fail "first provision run failed"
}
"${SCRIPT_DIR}/org-agent-provision.sh" "${COMMUNITY}" >"${LOG2}" 2>&1 || {
  cat "${LOG2}" >&2
  fail "second provision run failed"
}

PUBKEY="$(sed -n 's/^pubkey: //p' "${LOG1}" | head -n1)"
[[ "${PUBKEY}" =~ ^[0-9a-f]{64}$ ]] || fail "first run did not print pubkey: …"

PUBKEY2="$(sed -n 's/^pubkey: //p' "${LOG2}" | head -n1)"
[[ "${PUBKEY}" == "${PUBKEY2}" ]] || fail "second run minted a different pubkey"

SECRET="$(cat "${STORE}/${COMMUNITY}/secret")"
[[ "${SECRET}" =~ ^[0-9a-fA-F]{64}$ ]] || fail "operator store secret is not 64 hex chars"
[[ "${SECRET}" != "${PUBKEY}" ]] || fail "store secret equals the pubkey"

# Secrets must not appear in either run's captured output.
if grep -F -q -- "${SECRET}" "${LOG1}" "${LOG2}"; then
  fail "secret was printed (found in provision stdout/stderr)"
fi

LIVE_COUNT="$(sql "SELECT count(*) FROM io_hosted_agents
  WHERE community_id = '${COMMUNITY_ID}' AND retired_at IS NULL;")"
[[ "${LIVE_COUNT}" == "1" ]] || fail "expected 1 live io_hosted_agents row, got ${LIVE_COUNT}"

LIVE_HEX="$(sql "SELECT encode(pubkey, 'hex') FROM io_hosted_agents
  WHERE community_id = '${COMMUNITY_ID}' AND retired_at IS NULL;")"
[[ "${LIVE_HEX}" == "${PUBKEY}" ]] || fail "live row pubkey ${LIVE_HEX} != ${PUBKEY}"

MEMBER_COUNT="$(sql "SELECT count(*) FROM relay_members
  WHERE community_id = '${COMMUNITY_ID}' AND pubkey = '${PUBKEY}';")"
[[ "${MEMBER_COUNT}" == "1" ]] || fail "expected 1 NIP-43 member row, got ${MEMBER_COUNT}"

KIND0="$(sql "SELECT content FROM events
  WHERE community_id = '${COMMUNITY_ID}'
    AND kind = 0
    AND pubkey = decode('${PUBKEY}', 'hex')
  ORDER BY created_at DESC
  LIMIT 1;")"
echo "${KIND0}" | grep -q '"name": "Org agent"\|"name":"Org agent"' \
  || fail "kind:0 content was not name=Org agent (got: ${KIND0})"

echo "PASS: org-agent-provision.sh is idempotent; row + NIP-43 member + kind:0 Org agent; secret stayed in ${STORE}/${COMMUNITY}/secret"
