# Local org onboarding (O-3a)

Dogfood path for creating a community on a local relay and walking the
Personal Assistant → Shaper → bootstrap flow before staging (O-2).

## What ships

1. New communities seed **only** `#welcome-everyone` (no `#general`, no
   private Block-era **Welcome** channel).
2. After create / first-run — and once per session when PA is still missing —
   desktop creates a **Personal Assistant** managed agent and opens that DM.
   Failure surfaces a toast with Retry (no silent Welcome fallback).
3. Guide panel: congratulate → sole Shaper vs others (board-like explanation)
   → **Bootstrap as first Shaper** publishes real `50001` (relay creates
   `#shapers`) → alone continues direction in chat / Overview; others mint
   an invite.
4. `org` preview feature is **on by default** (`preview-features.json`).
5. Org agent stays unlisted (Agents door); Agents still seeds no sample
   personas. The PA is the founder's own buzz-acp agent, marked with env
   `BUZZ_HYPHA_PERSONAL_ASSISTANT=1` (not a teams-store `teamId` — that was
   rejected by `create_managed_agent` and caused the Welcome fallback).

## How to try (Vlad's machine)

```bash
. ./bin/activate-hermit
cp -n .env.example .env
# Ensure relay key + infra
scripts/ensure-local-relay-key.sh .env
docker compose up -d   # Postgres + Redis
just relay             # ws://localhost:3000
just desktop-dev       # or full `just dev`
```

### AI keys (Personal Assistant chat)

Desktop agent defaults / onboarding must have a working runtime. For the
bundled **buzz-agent** sidecar typically:

| Provider   | Env |
| ---------- | --- |
| Anthropic  | `ANTHROPIC_API_KEY`, `ANTHROPIC_MODEL` |
| OpenAI-compat | `OPENAI_COMPAT_API_KEY`, `OPENAI_COMPAT_MODEL`, optional `OPENAI_COMPAT_BASE_URL` |
| OpenRouter | `OPENROUTER_API_KEY` |

Set these in the desktop agent env (Settings → Agents defaults, or shell
env before `just desktop-dev`). See `crates/buzz-agent/README.md`.

Without a runtime/key, create still lands on `#welcome-everyone` / private
Welcome fallback; the org guide will not open.

### Optional: hosted org agent + `#shapers` agent row

Bootstrap works without a hosted agent (`39103.agent` may be null). To also
provision the unlisted org agent:

```bash
scripts/org-agent-provision.sh localhost:3000 --no-launch
# Optional model for buzz-org-agent later:
# IO_MODEL_DRAFT=… IO_MODEL_FAST=…
```

Then in the PA guide tap **Bootstrap as first Shaper**, or:

```bash
buzz org bootstrap
```

## Protocol tension (intentional)

Design § Personal Assistant says the PA **is** the org agent's DM. Phase 0
has no HEAR, so O-3a uses a member-owned buzz-acp PA for live local AI and
keeps the org agent separate / unlisted. When A-2+ HEAR lands, fold this
guide into the org-agent DM and retire the temporary managed agent.
