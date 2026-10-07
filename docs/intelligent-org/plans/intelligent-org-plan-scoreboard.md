# Plan scoreboard

One row per org per wave. A wave that drops a hard gate, or drops a judge
score by more than five points, is reverted and marked reverted.

The live planner is `dm_chat::serve` (`ensure_project_steps`,
`think_project_steps`, `ensure_held_plans`, `publish_due_tickets`) and the
job loop's `draft_ready_tickets`. Both now require a code file to be in the
digest they were given. There is no model key in this environment
(`OPENAI_COMPAT_API_KEY`, `VENICE_API_KEY`, and `OPENAI_API_KEY` are unset),
so no wave has a judge score and the corpus counts below are not claimed.

| Wave | Org | Projects | Tickets | Prompts | Hard gates | Judge | System change | Kept |
| ---- | --- | -------- | ------- | ------- | ---------- | ----- | ------------- | ---- |
| 1 | — | — | — | — | accept publishes every step, including steps that wait; how and done-when survive a cleared `plans` map | not run | Publish the full step list and a `50104` with each ticket. Recover how-lines from the ticket drafts. | kept |
| 2 | buzz (digest only) | — | — | — | a code path outside the digest publishes nothing; the buzz digest names 80 tracked files at `132d0d0de6e38e9ac3ea9ba9cd57956a9a9b3360` | not run | Put the home-repo listing and the `39106` file list in the planner prompt. Ground steps before publish. The relay stores `commit` and `files` on a repository. | kept |

## Still open

The corpus is not started. Do not fill it with handwritten tickets. The next
session needs a model key, then runs the production planner on seeds and
scores with a second model.

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
