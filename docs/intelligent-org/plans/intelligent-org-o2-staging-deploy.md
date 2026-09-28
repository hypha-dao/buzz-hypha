# Intelligent-org O-2 — Staging relay deploy (operator runbook)

Slice **O-2** from the [development plan](./intelligent-org-development-plan.md):
staging relay with the R-wave migration; desktop build with the `org` gate on
for the team. Spec: [Phase 0 § Deploy](./intelligent-org-phase-0.md). Prove:
`buzz org bootstrap` on the staging community; `39103` visible in Overview.

This file is the **operator checklist**. In-repo prep is verified by
`just org-staging-check`. Live prove is `just org-staging-smoke` once
credentials exist. Do not mark the slice `merged` until the live prove
passes on the real staging community.

---

## What is already in the repo

| Piece | Where |
| ----- | ----- |
| Migrations `0045` (`io_*`) and `0046` (DM roster fence) | `migrations/` — applied at relay boot when `BUZZ_AUTO_MIGRATE=true` (chart default) |
| Handlers / CLI / desktop Overview | wave-1 R-* + C-2 + D-0/D-1 on `main` |
| Hosted-agent provisioner | `scripts/org-agent-provision.sh` (O-1) |
| Team desktop: `org` gate on by default | `preview-features.json` → `org.defaultEnabled: true` |
| Prep check (no network) | `just org-staging-check` / `scripts/check-org-staging-prep.sh` |
| Live smoke | `just org-staging-smoke` / `scripts/org-staging-smoke.sh` |

Sample personas stay unseeded (D-5) — Hypha default, not a flag.

---

## Deploy steps (human / operator)

### 1. Publish a relay image that includes this fork's org commits

The checked-in staging-dev image workflow
(`.github/workflows/staging-dev-relay-image.yml`) **only runs on
`block/buzz`** and refuses every other repository. Hypha's org work lives on
`hypha-dao/buzz-hypha`. Pick one path and record which:

1. **Promote into `block/buzz`** (or a Block-owned branch the staging
   chart pins), then dispatch `Staging dev relay image` from `block/buzz`
   `main` with `target_ref` = the SHA that contains migrations 0045/0046
   and the org handlers; or
2. **Stand up a Hypha-owned staging relay** that builds from
   `hypha-dao/buzz-hypha` (compose / Helm) and points the team desktop at
   that host.

Until one of those exists, there is no image to roll.

### 2. Roll the staging chart / compose

- Image digest or tag from step 1.
- `migrate.autoMigrate: true` (default) so 0045/0046 apply on boot, **or**
  a controlled `buzz-admin migrate` job first (see
  `deploy/charts/buzz/README.md`).
- Confirm `/_status` reports the expected `build.source_sha`.
- Confirm Postgres has `_sqlx_migrations` rows for versions `45` and `46`.

Block's shared staging host used by `just staging` today is
`wss://sprout-oss.stage.blox.sqprod.co` — it sits behind **Cloudflare
Access** (Block SSO). An agent without SSO cookies or a service token
cannot reach NIP-11 or bootstrap.

### 3. Provision the hosted agent (O-1) for the dogfood community

```bash
# operator store + DATABASE_URL / RELAY_URL / BUZZ_RELAY_PRIVATE_KEY set
scripts/org-agent-provision.sh <community.host>
```

### 4. Desktop for the team

Any desktop build from this fork already enables the Org doors
(`org.defaultEnabled: true`). Team members who previously toggled Org off
in Experimental features keep that override until they clear it. No
separate flavoured binary is required for Phase 0.

`just staging` still points at Block's shared staging URL — override
`BUZZ_RELAY_URL` when the dogfood community lives elsewhere.

### 5. Smoke (slice prove)

```bash
export BUZZ_RELAY_URL=wss://<staging-host>
export BUZZ_PRIVATE_KEY=<community-owner>   # hex or nsec
# export BUZZ_AUTH_TAG=...                  # if required
cargo build -p buzz-cli --release
just org-staging-smoke
```

Expected: bootstrap accepted (or duplicate if already run); 
`org shapers list` returns a `kind:39103`. Then open Overview in the
desktop — Shapers card renders that event.

O-3 (second Shaper, invites, four directions) starts only after this
smoke is green.

---

## Blockers (as of this slice's in-repo PR)

Exact gaps that prevent finishing O-2 from this environment. Strike and date
when cleared.

1. **No deploy path from `hypha-dao/buzz-hypha` into the staging relay image.**
   `Staging dev relay image` exits unless `github.repository == block/buzz`.
   Nobody has named a Hypha-owned staging cluster or a promotion SHA on
   `block/buzz` that carries the R-wave.
2. **No cluster / registry credentials in the agent environment.**
   No `kubectl`, no AWS/ECR profile, no Helm values for a Hypha staging
   release — cannot `helm upgrade` or push an image.
3. **Staging HTTP is behind Cloudflare Access.**
   Unauthenticated `GET https://sprout-oss.stage.blox.sqprod.co/` returns
   the Access login HTML. Bootstrap and NIP-11 need SSO or a service token
   held by a Block/Hypha operator — not present here.
4. **Dogfood community host and owner key are not in this environment.**
   Readiness D9 names the Hypha team community and two Shapers; the owner
   `BUZZ_PRIVATE_KEY` and the community `Host` are operator secrets.
5. **Hosted-agent provider env (D10) is out of scope for the smoke** but
   still required before O-1 launch on staging (`IO_MODEL_*`, Mesh/OpenAI
   gateway). Tracked for A-2 shadow, not this prove.

When 1–4 clear, re-run `just org-staging-smoke`, paste the log into the
progress row, flip O-2 to `merged`, and open O-3.

---

## Local / CI

```bash
. ./bin/activate-hermit
just org-staging-check          # always — in-repo prep
just org-staging-smoke          # only with staging credentials (exit 2 = blocked)
```
