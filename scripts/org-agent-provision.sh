#!/usr/bin/env bash
# Provision the hosted org agent for one community (intelligent-org O-1).
#
#   scripts/org-agent-provision.sh <community> [--no-launch]
#
# <community> is `communities.host` (e.g. localhost:3000).
#
# Steps (Org agent § 15.2, Phase 0 § Deploy, Design § Where it runs):
#   1. Mint a keypair into the operator store (reuse on re-run).
#   2. NIP-43 add via `buzz-admin add-member` (kind:13534 roster).
#   3. Publish kind:0 `{"name":"Org agent"}`.
#   4. Write the `io_hosted_agents` row (`buzz-admin org hosted-agent set`).
#   5. Launch `buzz-org-agent run` with env (A-1 run exits after the skeleton
#      check — it does not open a live RelayIo socket).
#
# Operator store (secrets never printed):
#   ${ORG_AGENT_STORE:-${XDG_STATE_HOME:-$HOME/.local/state}/buzz/org-agents}/<community>
#     secret   0600  hex private key
#     pubkey   0644  hex public key
#
# Env the script reads:
#   DATABASE_URL, RELAY_URL, REDIS_URL, BUZZ_RELAY_PRIVATE_KEY
#     — operator / relay (add-member publishes the 13534 list)
#   BUZZ_RELAY_URL   websocket the agent uses (default: RELAY_URL)
#   BUZZ_AUTH_TAG, IO_TIMEZONE, provider variables
#     — passed through to `buzz-org-agent run` when launched
#   ORG_AGENT_STORE, BUZZ_ADMIN_BIN, ORG_AGENT_BIN
#
# A second run is idempotent: same pubkey, same live row, no duplicate member.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

NO_LAUNCH=0
COMMUNITY=""

usage() {
  echo "Usage: $0 <community> [--no-launch]" >&2
  echo "  <community>  communities.host (e.g. localhost:3000)" >&2
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-launch) NO_LAUNCH=1; shift ;;
    -h|--help) usage ;;
    --*) echo "unknown option: $1" >&2; usage ;;
    *)
      if [[ -n "${COMMUNITY}" ]]; then
        echo "error: extra argument: $1" >&2
        usage
      fi
      COMMUNITY="$1"
      shift
      ;;
  esac
done

[[ -n "${COMMUNITY}" ]] || usage

if [[ -f "${REPO_ROOT}/.env" ]]; then
  set -o allexport
  # shellcheck disable=SC1091
  source "${REPO_ROOT}/.env"
  set +o allexport
fi

STORE_ROOT="${ORG_AGENT_STORE:-${XDG_STATE_HOME:-${HOME}/.local/state}/buzz/org-agents}"
# Hosts are filesystem-safe (`localhost:3000`); flatten any leftover slash.
STORE="${STORE_ROOT}/${COMMUNITY//\//_}"

resolve_bin() {
  local env_name="$1"
  local crate="$2"
  local bin="$3"
  local override="${!env_name:-}"
  if [[ -n "${override}" && -x "${override}" ]]; then
    printf '%s' "${override}"
    return
  fi
  for candidate in \
    "${REPO_ROOT}/target/debug/${bin}" \
    "${REPO_ROOT}/target/release/${bin}"
  do
    if [[ -x "${candidate}" ]]; then
      printf '%s' "${candidate}"
      return
    fi
  done
  echo "building ${crate}…" >&2
  (cd "${REPO_ROOT}" && cargo build -p "${crate}")
  printf '%s' "${REPO_ROOT}/target/debug/${bin}"
}

ADMIN_BIN="$(resolve_bin BUZZ_ADMIN_BIN buzz-admin buzz-admin)"
AGENT_BIN="$(resolve_bin ORG_AGENT_BIN buzz-org-agent buzz-org-agent)"

mkdir -p "${STORE}"
chmod 700 "${STORE_ROOT}" 2>/dev/null || true
chmod 700 "${STORE}"

PUBKEY="$("${ADMIN_BIN}" org hosted-agent mint --store "${STORE}")"
if [[ ! "${PUBKEY}" =~ ^[0-9a-f]{64}$ ]]; then
  echo "error: mint did not print a 64-char hex pubkey" >&2
  exit 1
fi

echo "community: ${COMMUNITY}"
echo "pubkey: ${PUBKEY}"
echo "store: ${STORE}"

MEMBER_OUT="$("${ADMIN_BIN}" add-member --pubkey "${PUBKEY}" --role member --host "${COMMUNITY}")"
echo "member: ${MEMBER_OUT}"

PROFILE_OUT="$("${ADMIN_BIN}" org hosted-agent publish-profile --secret-file "${STORE}/secret" --host "${COMMUNITY}")"
echo "profile: ${PROFILE_OUT}"

SET_OUT="$("${ADMIN_BIN}" org hosted-agent set --pubkey "${PUBKEY}" --host "${COMMUNITY}")"
echo "hosted-agent: ${SET_OUT}"

if [[ "${NO_LAUNCH}" -eq 1 ]]; then
  echo "launch: skipped"
  exit 0
fi

# Load the secret into this process only — never echo it, never pass it as an argv.
BUZZ_PRIVATE_KEY="$(cat "${STORE}/secret")"
export BUZZ_PRIVATE_KEY
export BUZZ_RELAY_URL="${BUZZ_RELAY_URL:-${RELAY_URL:-ws://localhost:3000}}"
export IO_TIMEZONE="${IO_TIMEZONE:-UTC}"
# BUZZ_AUTH_TAG and provider variables are inherited when set.

set +e
LAUNCH_OUT="$("${AGENT_BIN}" run 2>&1)"
LAUNCH_CODE=$?
set -e
echo "launch: ${LAUNCH_OUT}"
if [[ "${LAUNCH_CODE}" -ne 0 ]]; then
  echo "error: buzz-org-agent run exited ${LAUNCH_CODE}" >&2
  exit "${LAUNCH_CODE}"
fi
