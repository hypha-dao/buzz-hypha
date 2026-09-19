//! Human-authored evaluation cases (Development plan E-2).
//!
//! Files live under `tests/eval/cases/`. They are not generated: `generate.mjs`
//! does not emit them and `just org-fixtures-check` does not see them. Each
//! suite file is a JSON object with a `cases` array. Gold is intent-level
//! (`gap`, `needs`, one-line `intent`) plus optional typed payloads that
//! parse as [`buzz_core::intelligent_org`] types.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use buzz_core::intelligent_org::{
    DirectionDraft, DoneDraft, DraftKind, DraftPayload, DriDraft, HealthBand, HealthRead, LineOp,
    ObjectivesDraft, ProfileDraft, ProjectDraft, ReviewDraft, TicketDraft,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::FixtureError;

/// Where the cases live: `<crate>/tests/eval/cases`.
pub fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("eval")
        .join("cases")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, FixtureError> {
    let text = fs::read_to_string(path).map_err(|source| FixtureError::Io {
        path: path.to_owned(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| FixtureError::Json {
        path: path.to_owned(),
        source,
    })
}

/// Positive / negative / adversarial (AI evaluation § How we test it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Polarity {
    /// A draft (or health read, or answer) must appear.
    Positive,
    /// The right answer is nothing.
    Negative,
    /// A hard edge: overlap, locale, fencing, invented order.
    Adversarial,
}

/// Which seed (and optional sequence snapshot) the case replays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedRef {
    /// `river`, `energy`, or `cold`.
    pub org: String,
    /// Locale of the seed file. Defaults to `en`.
    #[serde(default = "default_locale")]
    pub locale: String,
    /// Sequence name when the case layers on an E-1 sequence snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    /// `before`, `gate`, `outcome_a`, or `outcome_b`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

fn default_locale() -> String {
    "en".into()
}

/// The deterministic trigger that fires the job (Org agent § 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger {
    /// Trigger name: `direction_confirmed`, `item_accepted`, `child_done_unblocks`, …
    pub kind: String,
    /// Direction slug when the trigger is a `39100` head.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Item uuid, when the trigger names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// Manifest item key, when that is the stable name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_key: Option<String>,
    /// Fixture person name of the speaker (talk-derived jobs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    /// Manifest room key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    /// The utterance, when HEAR is in play.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// State the seed does not yet hold. A-2+ applies these as fixture deltas;
/// E-2 records them so gold is reviewable without inventing events.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imagine {
    /// Why the overlay exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Extra objective or strategy lines, not in the E-1 seed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub direction_lines: Vec<ImagineLine>,
    /// Skill slugs added to a named member's `39105`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profile_skills: Vec<ImagineSkill>,
    /// A `gap` key the Shapers declined on the current L3 version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declined_gap: Option<String>,
}

/// An imagined direction line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImagineLine {
    /// `objectives` or `strategy`.
    pub slug: String,
    /// Line id.
    pub id: String,
    /// The line.
    pub text: String,
}

/// An imagined skill on a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImagineSkill {
    /// Fixture person name.
    pub person: String,
    /// Skill slug (kebab-case, matching who-is-needed).
    pub skill: String,
}

/// Evidence the gold holder must cite.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchedGold {
    /// Skill slugs from their `39105`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    /// Quoted span from `about`, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// Prior item uuids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
}

/// One draft the gold says must exist (AI evaluation § The harness).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpectedDraft {
    /// `t` tag.
    pub kind: DraftKind,
    /// One-line intent a reviewer can disagree with.
    pub intent: String,
    /// `needs` tag: `shaper` or a pubkey / person name.
    pub needs: String,
    /// Dedupe `gap` key, when the move has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<String>,
    /// `origin` tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Suggested DRI pubkey or fixture name; JSON `null` means nobody.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_dri: Option<Option<String>>,
    /// Suggested holder pubkey or fixture name; JSON `null` means nobody.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_holder: Option<Option<String>>,
    /// What the piece needs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    /// Named gap when nobody fits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unfilled: Option<String>,
    /// Evidence for the named person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched: Option<MatchedGold>,
    /// Phrase in the parent brief.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub covers: Option<String>,
    /// `true` when this piece is the gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<bool>,
    /// Live or done sibling ids / keys the draft follows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<String>,
    /// Plan keys drafted now (sequence cases).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub draft_now: Vec<String>,
    /// Plan keys that stay held.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub held: Vec<String>,
    /// Words the brief must carry from a gate outcome.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_mention: Vec<String>,
    /// Objective line the project serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_ref: Option<String>,
    /// `follow_up` or `no_further_work`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
    /// Redraw operations; each parses as [`LineOp`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ops: Vec<Value>,
    /// Optional full payload; must parse as [`DraftPayload`] for `kind`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

/// Something that must not be drafted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MustNot {
    /// Why this would be wrong.
    pub why: String,
    /// Optional `gap` key that must stay absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<String>,
    /// Optional title / phrase that must not appear.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Health gold (move 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthGold {
    /// Expected band; parses as [`HealthBand`].
    pub band: HealthBand,
    /// Root item uuid or manifest key.
    pub item: String,
    /// Facts the sentences must cover.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_cover: Vec<String>,
    /// Claims that fail the judge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fails: Vec<String>,
    /// `true` when the gold is "too early to read".
    #[serde(default)]
    pub too_early: bool,
    /// Optional full `50101` content; parses as [`HealthRead`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

/// J10 answer gold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerGold {
    /// Language of the answer.
    #[serde(default = "default_locale")]
    pub language: String,
    /// Live numbers the answer refused to state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub live: Vec<String>,
    /// `true` when the ledger cannot answer.
    #[serde(default)]
    pub not_in_the_record: bool,
    /// Optional `{ answer, live }` object matching Org agent § 8.6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

/// J8 `io_done` gold (Protocol §5.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoDoneGold {
    /// The item that closes.
    pub item: String,
    /// The receipt is the holder's message, not the command.
    pub receipt_is_message: bool,
}

/// The gold a reviewer signs off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gold {
    /// `true` when the right output is nothing.
    pub silence: bool,
    /// Drafts that must exist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_draft: Vec<ExpectedDraft>,
    /// Drafts / titles / gaps that must not exist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_not: Vec<MustNot>,
    /// Move 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthGold>,
    /// J10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<AnswerGold>,
    /// J8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub io_done: Option<IoDoneGold>,
    /// J9 vague ask: HELP menu, not a guess.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_menu: Option<bool>,
    /// Refusal that names who may act.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

/// One evaluation case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalCase {
    /// Stable id, unique across suites.
    pub id: String,
    /// Org-agent § 1 job id (`J1`, `J1b`, …).
    pub job: String,
    /// `river`, `energy`, or `cold`.
    pub org: String,
    /// Seed locale.
    #[serde(default = "default_locale")]
    pub locale: String,
    /// Positive / negative / adversarial.
    pub polarity: Polarity,
    /// The trigger.
    pub trigger: Trigger,
    /// Which seed / snapshot.
    pub seed: SeedRef,
    /// Overlay the seed does not hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imagine: Option<Imagine>,
    /// Expected output.
    pub gold: Gold,
    /// Domain reasoning a reviewer can disagree with.
    pub why_gold: String,
}

/// A suite file under `tests/eval/cases/<suite>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuiteFile {
    /// Suite id (Eval § 1–5 names).
    pub suite: String,
    /// Default job when a case omits a more specific one.
    pub job: String,
    /// The cases.
    pub cases: Vec<EvalCase>,
}

/// Suites E-2 ships. The four moves, then Eval § 5.
pub const SUITE_FILES: &[&str] = &[
    "direction-to-projects",
    "projects-to-tickets",
    "completion-to-direction",
    "project-health",
    "dri-suggestion",
    "strategy-from-rejection",
    "direction-from-talk",
    "talk-to-work",
    "done-from-talk",
    "assistant-flows",
    "ask-the-org",
    "profile-draft",
];

/// Org agent § 1 jobs whose stages include a model call.
pub const MODEL_OUTPUT_JOBS: &[&str] = &[
    "J1", "J1b", "J2", "J3b", "J3c", "J3d", "J4", "J6", "J7", "J8b", "J9", "J10", "J11",
];

/// Every row of Org agent § 1.
pub const ORG_AGENT_JOBS: &[&str] = &[
    "J1", "J1b", "J2", "J3a", "J3b", "J3c", "J3d", "J4", "J5", "J6", "J7", "J8", "J8b", "J9",
    "J10", "J11", "J12", "J13", "J14", "J15", "J16", "J17",
];

/// Load `cases/<suite>.json`.
pub fn load_suite(suite: &str) -> Result<SuiteFile, FixtureError> {
    let path = cases_dir().join(format!("{suite}.json"));
    if !path.is_file() {
        return Err(FixtureError::Missing(format!("cases/{suite}.json")));
    }
    let file: SuiteFile = read_json(&path)?;
    if file.suite != suite {
        return Err(FixtureError::Missing(format!(
            "cases/{suite}.json suite field is {}",
            file.suite
        )));
    }
    Ok(file)
}

/// Load every suite file, in [`SUITE_FILES`] order.
pub fn load_all_suites() -> Result<BTreeMap<String, SuiteFile>, FixtureError> {
    let mut out = BTreeMap::new();
    for name in SUITE_FILES {
        out.insert((*name).to_string(), load_suite(name)?);
    }
    Ok(out)
}

/// The vacuous-title list (AI evaluation § The bar is expertise).
pub fn load_vacuous_titles() -> Result<Vec<String>, FixtureError> {
    let path = cases_dir().join("vacuous-titles.json");
    read_json(&path)
}

/// Parse every typed gold payload on a case. The first error names the field.
pub fn parse_case_gold(case: &EvalCase) -> Result<(), String> {
    if case.why_gold.trim().is_empty() {
        return Err("why_gold is empty".into());
    }
    if case.gold.silence
        && (!case.gold.must_draft.is_empty()
            || case.gold.health.is_some()
            || case.gold.io_done.is_some()
            || case
                .gold
                .answer
                .as_ref()
                .is_some_and(|a| !a.not_in_the_record && a.payload.is_some())
            || case.gold.help_menu == Some(true))
    {
        return Err("silence=true but gold names an output".into());
    }
    if !case.gold.silence
        && case.gold.must_draft.is_empty()
        && case.gold.health.is_none()
        && case.gold.answer.is_none()
        && case.gold.io_done.is_none()
        && case.gold.help_menu.is_none()
        && case.gold.refusal.is_none()
    {
        return Err("silence=false but gold names no output".into());
    }
    for (i, draft) in case.gold.must_draft.iter().enumerate() {
        if draft.intent.trim().is_empty() {
            return Err(format!("must_draft[{i}].intent is empty"));
        }
        for (j, op) in draft.ops.iter().enumerate() {
            serde_json::from_value::<LineOp>(op.clone())
                .map_err(|e| format!("must_draft[{i}].ops[{j}]: {e}"))?;
        }
        if let Some(payload) = &draft.payload {
            parse_draft_payload(draft.kind, payload)
                .map_err(|e| format!("must_draft[{i}].payload: {e}"))?;
        }
    }
    if let Some(health) = &case.gold.health {
        if let Some(payload) = &health.payload {
            serde_json::from_value::<HealthRead>(payload.clone())
                .map_err(|e| format!("health.payload: {e}"))?;
        }
    }
    if let Some(answer) = &case.gold.answer {
        if let Some(payload) = &answer.payload {
            let obj = payload
                .as_object()
                .ok_or_else(|| "answer.payload must be an object".to_string())?;
            if !obj.contains_key("answer") || !obj.contains_key("live") {
                return Err("answer.payload needs `answer` and `live`".into());
            }
        }
    }
    Ok(())
}

fn parse_draft_payload(kind: DraftKind, value: &Value) -> Result<DraftPayload, String> {
    let raw = serde_json::to_string(value).map_err(|e| e.to_string())?;
    match kind {
        DraftKind::Project => {
            let _: ProjectDraft =
                serde_json::from_str(&raw).map_err(|e| format!("ProjectDraft: {e}"))?;
        }
        DraftKind::Dri => {
            let _: DriDraft = serde_json::from_str(&raw).map_err(|e| format!("DriDraft: {e}"))?;
        }
        DraftKind::Ticket => {
            let _: TicketDraft =
                serde_json::from_str(&raw).map_err(|e| format!("TicketDraft: {e}"))?;
        }
        DraftKind::Done => {
            let _: DoneDraft = serde_json::from_str(&raw).map_err(|e| format!("DoneDraft: {e}"))?;
        }
        DraftKind::Review => {
            let _: ReviewDraft =
                serde_json::from_str(&raw).map_err(|e| format!("ReviewDraft: {e}"))?;
        }
        DraftKind::Objectives => {
            let _: ObjectivesDraft =
                serde_json::from_str(&raw).map_err(|e| format!("ObjectivesDraft: {e}"))?;
        }
        DraftKind::Direction => {
            let _: DirectionDraft =
                serde_json::from_str(&raw).map_err(|e| format!("DirectionDraft: {e}"))?;
        }
        DraftKind::Profile => {
            let _: ProfileDraft =
                serde_json::from_str(&raw).map_err(|e| format!("ProfileDraft: {e}"))?;
        }
        DraftKind::Money => {
            return Err("money drafts are next-version; E-2 has no gold payload".into());
        }
    }
    DraftPayload::parse(kind, &raw).map_err(|e| e.to_string())
}
