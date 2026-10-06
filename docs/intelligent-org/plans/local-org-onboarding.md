# Local org onboarding (O-3a)

Dogfood path for creating a community on a local relay and walking the
Personal Assistant → Shaper → bootstrap flow before staging (O-2).

## What ships

1. New communities seed **only** `#welcome-everyone` (no `#general`, no
   private Block-era **Welcome** channel).
2. After create / first-run, desktop bootstraps the founder (`50001`) if
   `#shapers` does not exist yet, then opens the **org agent** DM. That
   agent is the hosted one (`39103.agent`), not a member-owned assistant.
   Failure surfaces a toast with Retry (no silent Welcome fallback, no
   Personal Assistant).
3. The org agent opens by saying what it can do, then replies.
   There is no guide card under the composer. The operator's model key
   (`OPENAI_COMPAT_*`, or `VENICE_API_KEY`) is enough — members do not
   configure a runtime.
4. `org` preview feature is **on by default** (`preview-features.json`).
5. Org agent stays unlisted (Agents door); Agents still seeds no sample
   personas. Launch it with `scripts/org-agent-provision.sh` (or the
   operator supervisor in production). `run` stays connected.

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

### AI keys (org agent chat)

The hosted `buzz-org-agent run` process reads the operator environment.
Members never set these. For the bundled chat path:

| Provider   | Env |
| ---------- | --- |
| Anthropic  | `ANTHROPIC_API_KEY`, `ANTHROPIC_MODEL` |
| OpenAI-compat | `OPENAI_COMPAT_API_KEY`, `OPENAI_COMPAT_MODEL`, optional `OPENAI_COMPAT_BASE_URL` |
| OpenRouter | `OPENROUTER_API_KEY` |

Set these in the relay operator's `.env` before
`scripts/org-agent-provision.sh`. See `crates/buzz-agent/README.md`.

Without a hosted agent, create still lands on `#welcome-everyone`.

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

## Protocol

Design § Personal Assistant says that DM **is** the org agent. `buzz-org-agent run`
posts the opening lines and answers in that DM with the operator's model.
In that DM, and in `#shapers` as soon as the room exists, it can tag a
direction proposal, a project proposal, a ticket, done, or a DRI. With one
Shaper it answers every message in `#shapers`. With more than one it hears
every message and answers when the line asks it something or is about
direction or work. The member's client signs the command. THINK / JUDGE /
ROUTE stay on their own slices.
The temporary member-owned Personal Assistant is retired on the next
desktop session: autostart is turned off and a running copy is stopped.
