//! Loader for the evaluation fixtures under `tests/eval/fixtures/`
//! (Development plan E-1; AI evaluation § Test data).
//!
//! The fixtures are signed Nostr events in relay order, generated from the
//! prototype's `data.ts` by `tests/eval/fixtures/generate.mjs` and checked
//! in. Three kinds of file:
//!
//! - **Seeds** — `orgs/<org>/seed.json` (`en`) and `seed.<locale>.json`, one
//!   event per line, plus `manifest.json` (the ids a case needs by name) and
//!   `health-gold.json` (the `50101` gold for move 4, not state).
//! - **Sequences** — `sequences/<name>/sequence.json` describes a multi-step
//!   brief and its gold; the snapshot files are deltas applied over the org's
//!   seed, in the order `snapshots` lists them.
//! - **Who is needed** — `who-is-needed/<org>.json`: per requirement, who in
//!   the seed meets it and the answer the holder suggestion must give.
//!
//! Evaluation **cases** (Development plan E-2) live next to the fixtures, under
//! `tests/eval/cases/`. They are human-authored gold, not generated. See
//! [`cases`].
//!
//! [`decode`] turns any fixture event into the `buzz_core::intelligent_org`
//! type for its kind. Protocol §4 tag checks live in [`crate::inbound`]
//! and run from [`crate::state::OrgState::apply`]; this module loads files
//! and calls through.

pub mod cases;
mod decode;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use buzz_core::Event;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::inbound::ApplyError;
use crate::state::OrgState;

pub use cases::{
    cases_dir, load_all_suites, load_suite, load_vacuous_titles, parse_case_gold, EvalCase, Gold,
    Polarity, SuiteFile, MODEL_OUTPUT_JOBS, ORG_AGENT_JOBS, SUITE_FILES,
};
pub use decode::{decode, roundtrip, CommandContent, Decoded};

/// Where the fixtures live: `<crate>/tests/eval/fixtures`.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("eval")
        .join("fixtures")
}

/// What can go wrong loading or decoding a fixture.
#[derive(Debug, Error)]
pub enum FixtureError {
    /// A fixture file could not be read.
    #[error("read {path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// A fixture file is not the JSON it should be.
    #[error("parse {path}: {source}")]
    Json {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: serde_json::Error,
    },
    /// An event's content does not deserialize as its kind's type, its tag
    /// set is not Protocol §4's, or it does not re-serialize to itself.
    #[error("event {id} (kind {kind}): {reason}")]
    Event {
        /// The event id.
        id: String,
        /// The event kind.
        kind: u32,
        /// What is wrong.
        reason: String,
    },
    /// A name (org, sequence, snapshot stage) the fixtures do not have.
    #[error("no fixture named {0}")]
    Missing(String),
}

impl From<ApplyError> for FixtureError {
    fn from(err: ApplyError) -> Self {
        match err {
            ApplyError::Event { id, kind, reason } => Self::Event { id, kind, reason },
        }
    }
}

/// Apply every event through [`OrgState::apply`] (the harness loader).
pub fn apply_events(state: &mut OrgState, events: &[Event]) -> Result<(), FixtureError> {
    for event in events {
        state.apply(event).map_err(FixtureError::from)?;
    }
    Ok(())
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

/// Load one event-list file.
pub fn load_events(path: &Path) -> Result<Vec<Event>, FixtureError> {
    read_json(path)
}

fn subdirs(dir: &Path) -> Result<Vec<String>, FixtureError> {
    let entries = fs::read_dir(dir).map_err(|source| FixtureError::Io {
        path: dir.to_owned(),
        source,
    })?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| FixtureError::Io {
            path: dir.to_owned(),
            source,
        })?;
        if entry.path().is_dir() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

// ── seeds ────────────────────────────────────────────────────────────────────

/// A room in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestRoom {
    /// The NIP-29 group id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// `true` for a DM.
    pub dm: bool,
}

/// A direction head in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDirection {
    /// The head version.
    pub version: u32,
    /// The `39100` event id.
    pub event: String,
    /// Line id → text.
    pub lines: BTreeMap<String, String>,
}

/// A proposal in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestProposal {
    /// The proposal uuid.
    pub id: String,
    /// `direction`, `project`, `shapers`, …
    pub kind: String,
    /// `open`, `passed`, `rejected`.
    pub status: String,
}

/// `orgs/<org>/manifest.json` — the ids a case needs, by the names the
/// prototype uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The org: `river`, `energy`, `cold`.
    pub org: String,
    /// The locales the seed ships in.
    pub locales: Vec<String>,
    /// The fixture's present.
    pub now: u64,
    /// The relay's pubkey (signer of every state event).
    pub relay: String,
    /// The org agent's pubkey.
    pub agent: String,
    /// The harness's reader (`personas.you`).
    pub reader: String,
    /// Fixture name → pubkey.
    pub people: BTreeMap<String, String>,
    /// The Shaper set, in order.
    pub shapers: Vec<String>,
    /// Room key → room.
    pub rooms: BTreeMap<String, ManifestRoom>,
    /// Slug → head.
    pub direction: BTreeMap<String, ManifestDirection>,
    /// Item key (`<project>/<ticket-slug>/…`) → uuid.
    pub items: BTreeMap<String, String>,
    /// Proposal key → proposal.
    pub proposals: BTreeMap<String, ManifestProposal>,
    /// Kind → how many events of it the `en` seed holds.
    pub counts: BTreeMap<String, u32>,
}

/// One org's seed in every locale, with its manifest and health gold.
#[derive(Debug, Clone)]
pub struct OrgFixture {
    /// The org name.
    pub name: String,
    /// The manifest.
    pub manifest: Manifest,
    /// Locale → seed events, in relay order.
    pub seeds: BTreeMap<String, Vec<Event>>,
    /// Locale → `50101` gold (absent for an org with no health story).
    pub health: BTreeMap<String, Vec<Event>>,
}

/// The seed file for `locale`: `seed.json` for `en`, else `seed.<locale>.json`.
fn locale_file(stem: &str, locale: &str) -> String {
    if locale == "en" {
        format!("{stem}.json")
    } else {
        format!("{stem}.{locale}.json")
    }
}

/// The org names under `orgs/`.
pub fn org_names() -> Result<Vec<String>, FixtureError> {
    subdirs(&fixtures_dir().join("orgs"))
}

/// Load `orgs/<name>/`.
pub fn load_org(name: &str) -> Result<OrgFixture, FixtureError> {
    let dir = fixtures_dir().join("orgs").join(name);
    if !dir.is_dir() {
        return Err(FixtureError::Missing(format!("orgs/{name}")));
    }
    let manifest: Manifest = read_json(&dir.join("manifest.json"))?;
    let mut seeds = BTreeMap::new();
    let mut health = BTreeMap::new();
    for locale in &manifest.locales {
        seeds.insert(
            locale.clone(),
            load_events(&dir.join(locale_file("seed", locale)))?,
        );
        let gold = dir.join(locale_file("health-gold", locale));
        if gold.is_file() {
            health.insert(locale.clone(), load_events(&gold)?);
        }
    }
    Ok(OrgFixture {
        name: name.to_owned(),
        manifest,
        seeds,
        health,
    })
}

// ── sequences ────────────────────────────────────────────────────────────────

/// The item a sequence's brief lives on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceParent {
    /// Item uuid.
    pub id: String,
    /// Its root.
    pub root: String,
    /// Its depth.
    pub depth: u32,
    /// Title.
    pub title: String,
    /// The multi-step brief.
    pub brief: String,
    /// The holder's pubkey — the `needs` of every draft.
    pub holder: String,
    /// The holder's fixture name.
    pub holder_name: String,
}

/// One piece of the gold plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanPiece {
    /// Short key the outcomes refer to.
    pub key: String,
    /// The phrase in the brief.
    pub piece: String,
    /// 1-based position.
    pub order: u32,
    /// Keys of the pieces it follows.
    #[serde(default)]
    pub after: Vec<String>,
    /// `true` when its outcome decides the later pieces.
    #[serde(default)]
    pub gate: bool,
    /// `after <piece>` when held before the gate; `null` when it can start.
    pub held: Option<String>,
    /// The live item already covering it, when the piece is not drafted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub covered_by: Option<String>,
    /// The draft's title, for drafted pieces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What the piece needs, for drafted pieces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<Vec<String>>,
    /// The gold holder for a drafted piece; `null` when nobody here fits.
    #[serde(default)]
    pub suggested_holder: Option<String>,
    /// Why nobody fits, when a drafted piece has no `suggested_holder`.
    #[serde(default)]
    pub unfilled: Option<String>,
}

/// What happened at the gate: the drafts the agent made and the tickets
/// they became.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateStage {
    /// Piece key → `50100` event id.
    pub drafts: BTreeMap<String, String>,
    /// Piece key → item uuid.
    pub items: BTreeMap<String, String>,
    /// The gate's item uuid.
    pub gate_item: String,
    /// Who held the gate.
    pub gate_holder: String,
}

/// A piece the next wave must draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextWavePiece {
    /// Plan key, or a new key for a re-scoped piece.
    pub key: String,
    /// The phrase.
    pub piece: String,
    /// Title, for a piece not in the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What it needs.
    #[serde(default)]
    pub requires: Vec<String>,
    /// `true` when the re-scoped piece is a new gate.
    #[serde(default)]
    pub gate: bool,
    /// The plan piece this one replaces, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces: Option<String>,
    /// Item uuids the draft's `after` must name.
    #[serde(default)]
    pub after: Vec<String>,
    /// Words the draft's brief must carry from the outcome.
    #[serde(default)]
    pub must_mention: Vec<String>,
}

/// A piece the outcome moved under a different parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Elsewhere {
    /// The phrase.
    pub piece: String,
    /// The item it belongs under.
    pub under: String,
    /// Why.
    pub why: String,
}

/// One gate outcome and the next wave it calls for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceOutcome {
    /// The delta file.
    pub file: String,
    /// The agent trigger that re-runs the move.
    pub trigger: String,
    /// The gate holder's message the `io_done` cites.
    pub receipt: String,
    /// What they said.
    pub said: String,
    /// The drafts the next wave must produce.
    pub next_wave: Vec<NextWavePiece>,
    /// Plan keys that stay held.
    #[serde(default)]
    pub held: Vec<String>,
    /// Plan keys that are live or done and must not be drafted again.
    #[serde(default)]
    pub not_redrafted: Vec<String>,
    /// Pieces the outcome placed under another parent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elsewhere: Vec<Elsewhere>,
    /// What the judge fails.
    #[serde(default)]
    pub fails: Vec<String>,
    /// The domain reasoning.
    pub why_gold: String,
}

/// `sequences/<name>/sequence.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceCase {
    /// The sequence name.
    pub name: String,
    /// The org it layers on.
    pub org: String,
    /// The seed file, relative to the fixtures dir.
    pub seed: String,
    /// Stage → delta files, in order, applied after the seed.
    pub snapshots: BTreeMap<String, Vec<String>>,
    /// The item whose brief is the plan.
    pub parent: SequenceParent,
    /// The trigger for the "before" case.
    pub trigger: String,
    /// The gate piece's key.
    pub gate: String,
    /// The domain reasoning for the order.
    pub why_gold: String,
    /// The plan.
    pub plan: Vec<PlanPiece>,
    /// Plan keys drafted before the gate.
    pub draft_now: Vec<String>,
    /// The gate stage's ids.
    pub gate_stage: GateStage,
    /// `a` and `b`.
    pub outcomes: BTreeMap<String, SequenceOutcome>,
}

/// A sequence: its case and every delta file, loaded.
#[derive(Debug, Clone)]
pub struct SequenceFixture {
    /// The case.
    pub case: SequenceCase,
    /// The org's `en` seed.
    pub seed: Vec<Event>,
    /// File name → events.
    pub deltas: BTreeMap<String, Vec<Event>>,
}

impl SequenceFixture {
    /// The events of `stage` in relay order: the seed, then its deltas.
    pub fn snapshot(&self, stage: &str) -> Result<Vec<Event>, FixtureError> {
        let files = self
            .case
            .snapshots
            .get(stage)
            .ok_or_else(|| FixtureError::Missing(format!("{}/{stage}", self.case.name)))?;
        let mut out = self.seed.clone();
        for file in files {
            let delta = self
                .deltas
                .get(file)
                .ok_or_else(|| FixtureError::Missing(format!("{}/{file}", self.case.name)))?;
            out.extend(delta.iter().cloned());
        }
        Ok(out)
    }
}

/// The sequence names under `sequences/`.
pub fn sequence_names() -> Result<Vec<String>, FixtureError> {
    subdirs(&fixtures_dir().join("sequences"))
}

/// Load `sequences/<name>/`.
pub fn load_sequence(name: &str) -> Result<SequenceFixture, FixtureError> {
    let dir = fixtures_dir().join("sequences").join(name);
    if !dir.is_dir() {
        return Err(FixtureError::Missing(format!("sequences/{name}")));
    }
    let case: SequenceCase = read_json(&dir.join("sequence.json"))?;
    let seed = load_events(&fixtures_dir().join(&case.seed))?;
    let mut deltas = BTreeMap::new();
    for file in case.snapshots.values().flatten() {
        if !deltas.contains_key(file) {
            deltas.insert(file.clone(), load_events(&dir.join(file))?);
        }
    }
    Ok(SequenceFixture { case, seed, deltas })
}

// ── who is needed ────────────────────────────────────────────────────────────

/// A member with the required skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// Fixture name.
    pub name: String,
    /// Pubkey.
    pub pubkey: String,
    /// Their `open_limit`.
    pub open_limit: Option<u32>,
    /// The items they hold.
    pub open_items: Vec<String>,
    /// `true` when they hold `open_limit` items.
    pub at_limit: bool,
}

/// The answer a holder suggestion must give.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhoIsNeededGold {
    /// The pubkey, or `null`.
    pub suggested_holder: Option<String>,
    /// The skill slugs matched, when someone is suggested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_skills: Option<Vec<String>>,
    /// The gap, when nobody fits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unfilled: Option<String>,
}

/// One requirement and who meets it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirement {
    /// The skill slug.
    pub requires: String,
    /// `one`, `two`, or `nobody`.
    pub expect: String,
    /// Who has the skill, sorted by name.
    pub candidates: Vec<Candidate>,
    /// The gold answer.
    pub gold: WhoIsNeededGold,
}

/// `who-is-needed/<org>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhoIsNeeded {
    /// The org.
    pub org: String,
    /// The requirements.
    pub requirements: Vec<Requirement>,
}

/// Load `who-is-needed/<org>.json`.
pub fn load_who_is_needed(org: &str) -> Result<WhoIsNeeded, FixtureError> {
    let path = fixtures_dir()
        .join("who-is-needed")
        .join(format!("{org}.json"));
    if !path.is_file() {
        return Err(FixtureError::Missing(format!("who-is-needed/{org}")));
    }
    read_json(&path)
}
