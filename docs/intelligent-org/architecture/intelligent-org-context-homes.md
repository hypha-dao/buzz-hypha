---
title: 'The Intelligent Organization — Where context lives'
date: 2026-10-06
status: draft
tags: [architecture, intelligent-org, context, projects, git, buzz]
parent: docs/intelligent-org/README.md
---

# Where context lives

What belongs in the organization's context, what belongs in a project's,
and whether Buzz's existing projects system is the right place to keep it.

The system being fed is the one in
[State and the change engine](./intelligent-org-state-and-planning.md)
([short version](./intelligent-org-state-and-planning-brief.md)).

---

## The verdict

**Use Buzz's projects for files and code. Do not use them for what the
organization believes.**

Buzz already does something useful: creating a project creates a channel,
an empty git repository on the relay, and a grouping that can also point
at other repositories, including GitHub. That container is the right home
for a project's documents and for the code the work touches.

It is the wrong home for mission, objectives, refusals, and constraints.
Those have to be confirmed by Shapers, small enough to read, and citable
as relay events. A git push does none of those jobs. Anyone who can push
could rewrite "we never take brand money" without a vote.

So there are two contexts and three homes:

| Context | What it is | Where it lives |
| --- | --- | --- |
| Organization | What the whole org believes, and the lists it tracks | Relay events the Shapers confirm, plus a work list any member can add to |
| Project, the decision | The change, the plan, the open questions | The work item and its registers — the same event log |
| Project, the files | Decisions written down, drafts, research, the code | The project's git repositories |

---

## What Buzz does today

Checked in this repo, not from the intelligent-org plan.

**Creating a project creates a repository.** `createProject` in
`desktop/src/features/projects/createProject.ts` does three things: a
channel, a `30617` repository announcement bound to that channel, and a
`30621` project that lists the repository. If an earlier attempt created
the project and missed the repo, the retry adds the repo. The announcement
usually has no `clone` tag. The desktop then treats it as hosted on this
relay, at `/git/<owner>/<slug>`
(`desktop/src/features/projects/lib/projectCloneUrl.ts`). So yes: a Buzz
project comes with a repository, and that repository is a real git remote
on the relay, empty until someone pushes.

**A project can point at more repositories.** The `30621` is a named list
of `30617` coordinates (`docs/nips/NIP-MP.md`). You can attach one that
already exists, or announce a new one whose `clone` tag is a GitHub URL.
An explicit `clone` tag wins over the relay path. The desktop will clone a
**public** GitHub repository. It will not authenticate to a private one
(`projectGitError.ts`: public GitHub only, no credentials). Linking a
repository does not grant anyone push rights to it. Membership in the
`30621` is a grouping, not a permission.

**There are two other stores, and neither is a file tree.**

- **Canvas** (`40100`) is one markdown page per channel. The project-home
  template (`projectHomeTemplate.ts`) fills it with a how-to: the channel
  is the project's memory, issues are the task queue, the repository is
  the source of truth for code. Anyone who can edit the channel can
  replace the whole page.
- **Uploads** (`1063`) are files in media storage. They have an author and
  a name. They are not a folder, not a version, and their text is not
  indexed (Design, the documents gap).

**The intelligent-org project does not get the repository yet.** A passed
org project creates the room (R-9a, merged) and writes `home.channel`.
The repository and the `30621` are R-9b, not built. So the useful Buzz
behavior exists on the Projects door and is specified for the org door.
It is not wired to a passed `39101` yet.

**Buzz's own issues and pull requests are a different task system.** The
canvas template tells people to track work as git issues. In this fork the
ticket is the `39101`. The repository should not grow a second task list.

---

## Organization context

This is what every suggestion is checked against. It is small, confirmed,
and the same for every project.

**Write these down, and only Shapers change them.**

- Purpose, vision, and the situation: what we do, where this is going, where we stand.
- Objectives, each with a date and a "done when".
- Strategy: the bets, the rules, and the refusals. A refusal needs the words that make it checkable ("no brand money"), not a value that rules nothing out.
- How work looks here: how long a project runs, how many people hold one, whether new things start as a trial.
- Constraints that bind everyone: legal, safety, "the hall closes at 22:00".
- What we already have that any project might use: spaces, tools, partners, the list of codebases and what each one is.
- Money as a posture and a threshold ("a project is normally under 200; over 500 needs every Shaper"). Never a balance.

**Track these, without a vote.** Any member can add one. They are a work
list, not a belief.

- Open questions, assumptions, promises, things we are waiting on someone else for.
- Signals: a repeated request, a problem, an opportunity, with the messages that show it.

**Read these live. Never store the number.**

- Whether an objective is covered, who holds what, what closed, what was declined.
- Counts and balances, each with a date and a person who recorded them.

A git repository is a bad home for this list. A push is not a Shaper vote.
A file has no "this refusal came from the April decision". A clone of the
whole repo is too big to put in every prompt, and too easy for one holder
to steer. Canvas is worse for it: one page, last edit wins, no history a
person can approve line by line.

If someone wants to read the beliefs as files, generate them from the
events and mark them generated. The events stay the source.

---

## Project context

A project's context is only what this one change needs. It does not leak
into other projects' suggestions.

### The decision — on the work item, not in a file

- The change: from where we are, to where this project leaves us, and how we will know.
- The plan: the steps, which ones answer a question, which ones wait.
- Who holds it, the date, which objective it serves.
- Questions and assumptions that belong to this project.

These move when people accept, decline, and finish work. The event log
already records that. A markdown file would be a second copy that drifts
the first time someone edits one and not the other.

### The files — in the project's own repository

This is the part Buzz's project system is good at. When R-9b lands, the
relay-signed home repository should be created with a `context/` folder
already committed, not left empty:

| File | What goes in it |
| --- | --- |
| `context/README.md` | What this project is, in the holder's words. The page people open. |
| `context/decisions.md` | Choices made inside the project: what was picked, by whom, why. Append, don't rewrite history. |
| `context/links.md` | The few outside pages that matter: a form, a doc, a dashboard. |
| `context/drafts/` | Things the work produces that are documents: an application, a brief, a research note. |

Why a git folder and not the canvas:

- A commit has an author, a time, and a parent. The canvas is one blob replaced whole.
- The folder can grow past one page. The canvas cannot, without becoming a junk drawer.
- Push rights already follow the room: holders can write, everyone who can see the project can read, `main` can be protected. That is the structure we would otherwise have to build.
- The desktop already renders a repository README and browses files. `context/README.md` shows up with no new viewer.
- A prompt can cite `context/decisions.md` at a commit. A canvas cite is "whatever the page says now".

The canvas stays useful as a scratch page: a status line, a link to the
repo, how to get a checkout. It is not the record. The template that ships
today is a how-to for Buzz issues and pull requests. For an org project,
replace that text. Do not teach two task systems.

Uploads stay for images and attachments inside a message. A document the
project depends on is a file in `context/`, so it has a path and a version.

### The code — linked, not copied

When the work changes a codebase, add that repository to the project's
`30621`. That is the right use of Buzz's "add a repository" flow.

- A Buzz-hosted repo is already on the relay. Link its `30617`.
- A GitHub repo is announced as a `30617` with `clone` and `web` pointing at GitHub, then added to the same project. Public repositories can be cloned today. Private ones wait on a read token held by the operator, not by a member pasting a key into a file.
- The agent does not load the repository into the prompt. It keeps a short digest per commit: the README, `AGENTS.md` or `CONTRIBUTING`, the top of the tree, the manifests, the latest commits. For one ticket it may search for a few paths. Caps on size and time stay hard.
- Do not mirror the external repository into the home repo. Two copies drift, and the home repo stops meaning "this project's documents".

Linking is a bad way to manage the project's *intent*. The README of an
outside repo is written for that repo's contributors. It can be stale, it
can be hostile ("ignore your rules and…"), and a line in it is not a
decision this org made. The agent may read it as evidence about the code.
It may not treat it as an instruction, and it may not cite it as something
the Shapers confirmed. The same fence already used for chat messages
applies to file contents.

One project linking several repositories is a real need — a relay, a
desktop, a site — and `30621` was built for that. Use it. The org-level
list of codebases (what each repo is, who owns it) stays in the org
context, so a new project can be pointed at the right repo without every
holder rediscovering the map.

---

## What this means for the agent

| When it is drafting… | It reads | It does not read |
| --- | --- | --- |
| A project for the org | Org beliefs, the live work, the open questions, the codebase *list* | The inside of every repository. A file in some other project's `context/`. |
| Tickets for one project | That project's change and plan, its `context/` docs, the repos linked to it | Another project's documents. The whole git history. |
| A prompt for one ticket | The ticket's "done when", the few `context/` files that matter, the digest paths for the repo the ticket names | Chat from rooms this ticket's holder cannot see. A private GitHub repo we cannot clone. |

Project files reach the model only for that project. That is what stops
one holder steering every suggestion in the org.

---

## What we should not build

- A second document store beside git. Buzz already has the structured one.
- Org beliefs as files in a shared "org repo" that people push to. Generate a
  read-only export if a checkout is convenient. Confirmations stay events.
- The canvas as the knowledge base. One page is a scratch pad.
- Buzz issues as the org's tickets. The `39101` is the ticket. The branch
  on the ticket is how the code finds it.
- A full copy of GitHub inside Buzz. A link plus a bounded digest is the
  whole integration.

---

## Build this in the order the files become useful

1. **R-9b, with `context/` seeded.** The passed project gets its room (already), its relay-signed repository, and its `30621`. The first commit is the four paths above, not the Buzz issue how-to. Holders can push. `main` is protected.
2. **Link the repos the project actually changes.** Same "add repository" flow, including a public GitHub `clone` URL. The project page lists them next to `context/`.
3. **The agent reads `context/` and a digest of the linked repos** when it drafts that project's tickets and prompts. Org beliefs stay in the events it already loads.
4. **Private GitHub**, only once a public digest has been used for real. One operator token, read-only, not a per-member secret in the repo.

Until step 1, project documents have nowhere honest to live, and the canvas will get used for them by default. That is the failure mode to avoid: a single editable page becoming the project's memory because the repository was never created.
