//! Context bundle the judge reads. Recipes land in A-2; A-1 needs the shape.

use std::collections::BTreeSet;

use buzz_core::intelligent_org::HealthBand;
use serde::{Deserialize, Serialize};

/// A cited receipt (`e` / `a` / `ref`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Event id, address, or ref string.
    pub id: String,
}

/// A holder candidate the model was shown (§ 8.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// Pubkey.
    pub pubkey: String,
    /// Skill slugs on their `39105`.
    pub skills: Vec<String>,
    /// Items they held.
    pub items: Vec<String>,
    /// Open pieces they currently hold.
    pub open_count: u32,
    /// Cap, when set.
    pub open_limit: Option<u32>,
    /// NIP-43 member.
    pub is_member: bool,
    /// Profile version, for fingerprints.
    pub profile_version: u32,
}

/// The closed world THINK rendered and the judge enforces.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContextBundle {
    /// Every id the model was shown (`bundle.ids`).
    pub ids: BTreeSet<String>,
    /// Receipts that resolve (REQ-by-id, cached).
    pub resolved: BTreeSet<String>,
    /// Holder candidates.
    pub candidates: Vec<Candidate>,
    /// Generation of the object the job snapped.
    pub generation: String,
    /// Unix seconds — "now" for date gates.
    pub now: u64,
    /// Fingerprint of the current gap candidate.
    pub fingerprint: Option<String>,
    /// Fingerprint stored with a previous decline of this gap.
    pub declined_fingerprint: Option<String>,
    /// Expected health band (formula result) when judging a `50101`.
    pub expected_band: Option<HealthBand>,
    /// Factor names/values the formula produced, for numeral checks.
    pub factor_values: Vec<String>,
    /// Same-batch sibling drafts, for the sequence `gate` rule.
    pub batch: Vec<String>,
}
