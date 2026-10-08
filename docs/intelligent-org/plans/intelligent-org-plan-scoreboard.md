# Plan scoreboard

One row per org per wave. A wave that drops a hard gate, or drops a judge
score by more than five points, is reverted and marked reverted.

The live planner is `dm_chat::serve` (`ensure_project_steps`,
`think_project_steps`, `ensure_held_plans`, `publish_due_tickets`) and the
job loop's `draft_ready_tickets`. Both now require a code file to be in the
digest they were given. Wave 4 called `think_project_steps` then
`take_due_tickets` with `claude-sonnet-4-6` on Venice. The judge was a
different model, `llama-3.3-70b`. The key is not stored in the repo.

| Wave | Org | Projects | Tickets | Prompts | Hard gates | Judge | System change | Kept |
| ---- | --- | -------- | ------- | ------- | ---------- | ----- | ------------- | ---- |
| 1 | — | — | — | — | accept publishes every step, including steps that wait; how and done-when survive a cleared `plans` map | not run | Publish the full step list and a `50104` with each ticket. Recover how-lines from the ticket drafts. | kept |
| 2 | buzz (digest only) | — | — | — | a code path outside the digest publishes nothing; the buzz digest names 80 tracked files at `132d0d0de6e38e9ac3ea9ba9cd57956a9a9b3360` | not run | Put the home-repo listing and the `39106` file list in the planner prompt. Ground steps before publish. The relay stores `commit` and `files` on a repository. | kept |
| 3 | — | — | — | — | accept-time code prompts name `repo@commit`; a vacuous title publishes nothing; a missing skill sets `unfilled` and no holder | not run | Keep the digest commit on the accept path. Reject the vacuous-title list in `step_ready`. Parse `requires` and match it to a profile skill. | kept |
| 4 | river | 1 | 6 | 6 | every step published, including steps that wait; how and done-when present; titles are not the project; Ana, Tomasz, and the Ferreira family are in the mission | 13/14 | `think_project_steps` on the River direction, then `take_due_tickets`. Generator `claude-sonnet-4-6`. | kept |
| 4 | buzz | 1 | 3 | 3 | code files stayed in `digests/buzz.json`; each prompt names `hypha-dao/buzz-hypha@132d0d0d…` | first six questions already sum to 11; the written total was cut off | Same path with the buzz digest. A dropped connection on the first try; the 120s timeout and a 2s retry then passed. | kept |

## Still open

Two orgs, two projects, nine tickets. That is not the corpus. Do not fill
the rest with handwritten tickets. The next session repeats this live path
on the seeds below and holds eight orgs out of tuning.

Buzz digest the planner may name: `crates/buzz-org-agent/tests/eval/digests/buzz.json`.

## Next orgs

Add these as seeds (mission, vision, situation, objectives with how we will
know, strategy bets/rules/refusals, people, underway, declined, constraints).
Software orgs also carry a digest. Hold at least eight out of tuning.

1. Community hall — weekday demand unknown, so the project is a four-week trial, not "open the hall".
2. Energy co-op — a refusal that kills the obvious project.
3. Farm co-op — an objective already covered, so the right output is no project.
4. Clinic — a missing skill, so a ticket says nobody here can hold it.
5. School.
6. Research lab.
7. Design studio.
8. Local government.
9. Open-source maintainer group — a codebase whose obvious file is the wrong layer.
10. Software co-op — two people with the same skill.
11. This buzz repository — plan from `digests/buzz.json`.
12. A declined idea that must not come back, on one of the orgs above.

Then orgs unlike those, until there are 30. Held-out bar: 80% of projects,
80% of tickets, 80% of prompts. Hard gates stay at 100%.
