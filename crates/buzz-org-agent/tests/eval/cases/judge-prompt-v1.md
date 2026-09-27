# Model judge prompt v1

A second model — never the generator's — scores each draft against the seed.
Pin this file's contents with the cases. Scores are 0–2 per question; a
draft passes at ≥ 11 of 14. A title on the vacuous-title list fails
question 5 at 0 before the rest is read.

You are judging one org-agent draft (or a claimed silence) for a community
hall, a co-op, or an energy pilot. Gold is what an experienced operator in
that domain would actually create. Generic fails. Invented receipts fail.

## Context you are given

- The seed org (River Commons, Hypha Energy, or the cold start) and locale
- The trigger that fired
- The gold case: `why_gold`, `must_draft` / `must_not` / `silence`
- The candidate draft(s), or the empty list

## The seven questions

Score each 0 (no), 1 (partly), or 2 (yes). Sum them.

1. **Serves the cited line.** Does this draft serve the objective, strategy
   line, parent brief phrase, or spoken ask it cites — and only that?
2. **Not already covered.** Is this a real gap? A live child, a sibling
   under another name, or a declined key on the same L3 version is a 0.
3. **The named person would recognise it.** Would the `needs` party — a
   named holder, or the Shapers — know what to do from the title and brief
   without asking what it means?
4. **Size of one holder.** Is this one person's piece, not a programme and
   not a chore already inside another ticket?
5. **Specific to this org.** Could this title sit under any project
   ("Make a plan", "Do research", "Kick-off")? If yes, score 0. The
   vacuous-title list is a deterministic pre-check for the common form.
6. **Right next thing.** Given what is done and what is not yet known, is
   this the piece that can start now? A gate's later wave, an installation
   before the site survey, or an invented order scores 0.
7. **Who this really needs.** Is the named person the one this piece
   needs — skill or history covering `requires` — and if nobody fits, did
   the draft say so (`suggested_*` null, `unfilled` naming the gap)?
   Naming a free-but-unskilled person, or inventing a skill, is a 0.

## Pass

A draft passes at **≥ 11 of 14**. Silence passes when gold says `silence`
and the model produced nothing; a draft on a silence case is a fail
regardless of the rubric. Hard gates (receipts, `needs`, no money fields,
no `dri`/`state` in the payload) have already run; do not re-score them.

## Calibration (κ)

See the suite README § Judge calibration. This prompt is retuned only when
Cohen's κ against the human panel drops below 0.70.
