# org-preview — clickable prototype of the intelligent organization

A static, in-browser walk-through of the five doors (Overview, Projects,
Decisions, My Work, My Profile) plus DMs, group chats, and the Personal
Assistant. Two sample orgs: **River Commons** and **Hypha Energy**. No backend,
no auth — all data is in `src/lib/data.ts` and state lives in the browser.

Live copy: [hypha-org-preview.vercel.app](https://hypha-org-preview.vercel.app)

The product documents this prototype renders are in
[`docs/intelligent-org`](../../docs/intelligent-org/README.md).

## Run

This is a self-contained Next.js 15 app. It is **not** part of the Buzz pnpm
workspace on purpose (so it does not touch `pnpm-lock.yaml` or the desktop /
web CI lanes). Use npm from inside this folder:

```bash
cd prototypes/org-preview
npm install
npm run dev        # http://localhost:3010
```

Node 20 or newer. Routes:

- `/` — Hypha Energy overview (the org door)
- `/onboarding` — first-login flow

`npm run build && npm start` serves the production build on the same port.
`vercel.json` deploys it as a standalone Next.js project if you point Vercel's
root directory at `prototypes/org-preview`.

## Layout

```
src/app/            Next.js routes and global styles
src/components/     workspace shell, chat, person chip, primitives
src/screens/        one file per door or view
src/lib/data.ts     the two sample orgs — projects, tickets, people, decisions
src/lib/store.tsx   in-browser state (who you are, what you accepted)
src/lib/assist-flows.ts  Personal Assistant scripted flows
```

## Provenance

Moved from `apps/org-preview` on the `feat/org-preview` branch of
[`hypha-dao/hypha-web`](https://github.com/hypha-dao/hypha-web). Git history for
the prototype stays there.
