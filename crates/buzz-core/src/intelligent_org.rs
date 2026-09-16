//! Intelligent organization — content schemas.
//!
//! One Rust type per content schema in
//! `docs/intelligent-org/architecture/intelligent-org-protocol.md` §4: the
//! relay-signed state kinds (`39100–39105`), the person-signed command
//! contents (`50001–50021`, §4.8), and the agent's drafts and reads
//! (`50100–50103`). Kind numbers live in [`crate::kind`]; tag letters in
//! [`tag`]. Every type derives `serde` (the wire format) and
//! [`schemars::JsonSchema`] (the agent's structured-output schemas and the
//! fixture validator).
//!
//! These are shapes, not rules. Field-level limits are exposed as constants;
//! the executor (relay) enforces them. `Fields not listed are ignored`
//! (Protocol §4), so no type denies unknown fields. Timestamps are Unix
//! seconds; pubkeys are 64-char lowercase hex; ids are lowercase UUIDs or
//! 64-char event ids — all carried as `String` so the schema stays plain.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 64-char lowercase hex pubkey.
pub type PubkeyHex = String;
/// 64-char lowercase hex event id.
pub type EventIdHex = String;
/// Lowercase RFC 4122 UUID of a work item or proposal.
pub type ObjectId = String;
/// Unix seconds.
pub type Timestamp = u64;

/// The single-letter filter tags every `io` event uses (Readiness D4, D5, D11).
///
/// Content JSON keeps the full field names; only the tag names are letters,
/// because the relay's filter type indexes single letters only.
pub mod tag {
    /// Work item UUID on commands, `50101`, `50102`, and `39102`.
    pub const ITEM: &str = "i";
    /// Parent item UUID on `39101` and on `ticket`/`done` drafts.
    pub const PARENT: &str = "u";
    /// Object status on `39101`, `39102`, `39104`.
    pub const STATUS: &str = "s";
    /// The party a `50100` draft needs: a pubkey or `shaper`.
    pub const NEEDS: &str = "n";
    /// One skill slug per tag on `39105` and matched-skill tags on `50100`.
    pub const SKILL: &str = "k";
    /// Type: item kind on `39101`/`39102`, draft kind on `50100`, note type on `50103`.
    pub const TYPE: &str = "t";
    /// Value of [`NEEDS`] when any Shaper may act.
    pub const NEEDS_SHAPER: &str = "shaper";
    /// Fourth element of `["e", <draft>, "", "draft"]` — the command settles that draft.
    pub const MARKER_DRAFT: &str = "draft";
    /// Fourth element of `["e"|"a", <id>, "", "receipt"]`.
    pub const MARKER_RECEIPT: &str = "receipt";
    /// Fourth element of `["p", <pubkey>, "", "needs"]` on `50100`.
    pub const MARKER_NEEDS: &str = "needs";
    /// Fourth element of `["p", <pubkey>, "", "suggested"]` on `50100`.
    pub const MARKER_SUGGESTED: &str = "suggested";
    /// Fourth element of `["p", <pubkey>, "", "offered"]` on `39101`.
    pub const MARKER_OFFERED: &str = "offered";
    /// Fourth element of `["p", <pubkey>, "", "subject"]` on `39102`.
    pub const MARKER_SUBJECT: &str = "subject";
    /// Fourth element of `["p", <pubkey>, "", "eligible"]` on `39102`.
    pub const MARKER_ELIGIBLE: &str = "eligible";
}

// ── §4.1 kind:39100 — direction artifact ─────────────────────────────────────

/// The four direction artifacts; the `d` tag of a `39100`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DirectionSlug {
    /// Why the organization exists.
    Mission,
    /// What it is becoming.
    Vision,
    /// Numbered, dated lines the work tree serves (`objective_ref` targets).
    Objectives,
    /// Numbered lines: how the objectives get met.
    Strategy,
}

/// One numbered line of an `objectives` or `strategy` artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DirectionLine {
    /// 1-based position in this version.
    pub n: u32,
    /// Stable across versions when carried forward; fresh when added.
    pub id: String,
    /// The line.
    pub text: String,
    /// Target date, when the line has one.
    pub date: Option<Timestamp>,
}

/// Content of `kind:39100` — the latest confirmed version of one artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DirectionArtifact {
    /// Which artifact.
    pub slug: DirectionSlug,
    /// Version number; a new proposal makes `n + 1`.
    pub version: u32,
    /// Markdown: the statement and the paragraph or two behind it.
    pub body: String,
    /// `objectives` and `strategy` only; empty for `mission` and `vision`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<DirectionLine>,
    /// The Shaper whose vote met the rule.
    pub confirmed_by: PubkeyHex,
    /// When the rule was met.
    pub confirmed_at: Timestamp,
    /// Who opened the proposal.
    pub proposed_by: PubkeyHex,
    /// The passed `39102`, which holds the diff and the talk it came from.
    pub proposal: ObjectId,
    /// Event id of the previous `39100` head; absent on v1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<EventIdHex>,
}

// ── §4.2 kind:39101 — work item ──────────────────────────────────────────────

/// Work item state machine (Protocol §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemState {
    /// Real, nobody holds it.
    Open,
    /// Named person has not answered.
    Offered,
    /// Held — in progress means a holder.
    Accepted,
    /// A root in the last fifth of its run; still held.
    InReview,
    /// Closed.
    Done,
}

/// `project` (a root) or `ticket` (has a parent); the `t` tag of a `39101`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkItemKind {
    /// A root of the tree, created by a passed `project` proposal.
    Project,
    /// A child, created by `io_ticket_create` from the parent's holder.
    Ticket,
}

/// How a `done` item was closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ClosedBy {
    /// An `io_done` from the holder (or relayed by the agent, §5.5).
    Dri,
    /// A root that reached `due_at`.
    Rule,
    /// Recorded for completeness; a release is not a close.
    Release,
}

/// Value of `offered_by` when the offer came from the agent's draft, not a person.
pub const OFFERED_BY_AGENT: &str = "agent";

/// Counts of direct children by state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChildrenCounts {
    /// Children in `open`.
    pub open: u32,
    /// Children in `offered`.
    pub offered: u32,
    /// Children in `accepted` (or `in_review`).
    pub accepted: u32,
    /// Children in `done`.
    pub done: u32,
}

/// A project's home (Protocol §6.7), written when its proposal passes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectHome {
    /// The project's NIP-29 room.
    pub channel: String,
    /// NIP-34 announcement coordinate `30617:<relay>:<slug>`; absent on a relay without object storage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// NIP-MP project coordinate `30621:<relay>:<slug>`; absent with `repo`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

/// Content of `kind:39101` — one node of the work tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkItem {
    /// Item UUID; the `d` tag.
    pub id: ObjectId,
    /// Parent UUID; `null` for a project.
    pub parent: Option<ObjectId>,
    /// Root UUID; equals `id` for a project.
    pub root: ObjectId,
    /// 0 for a root.
    pub depth: u32,
    /// Ancestor UUIDs, root first.
    #[serde(default)]
    pub path: Vec<ObjectId>,
    /// Short name.
    pub title: String,
    /// Markdown: what this is for and what done looks like.
    pub brief: String,
    /// See [`WorkItemState`].
    pub state: WorkItemState,
    /// The holder; required when `state ∈ {accepted, in_review}`.
    pub dri: Option<PubkeyHex>,
    /// Who the open offer names.
    pub offered_to: Option<PubkeyHex>,
    /// A pubkey, or [`OFFERED_BY_AGENT`].
    pub offered_by: Option<String>,
    /// When the open offer was made.
    pub offered_at: Option<Timestamp>,
    /// End date (root) or estimated completion (child).
    pub due_at: Timestamp,
    /// When the root went live; roots only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<Timestamp>,
    /// `objectives@<version>#<line-id>` this root serves.
    pub objective_ref: Option<String>,
    /// The command that created it; the proposal for a root.
    pub created_from: EventIdHex,
    /// The `50100` it was promoted from, if any.
    pub draft: Option<EventIdHex>,
    /// The `io_done`, or the chat message it cited (§5.5).
    pub done_receipt: Option<EventIdHex>,
    /// How it closed; `null` while live.
    pub closed_by: Option<ClosedBy>,
    /// Direct children by state.
    #[serde(default)]
    pub children: ChildrenCounts,
    /// Roots only (§6.7); a child reads its root's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home: Option<ProjectHome>,
    /// Children only: the convention branch `io/<uuid[..4]>-<slug>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Children only: siblings this piece sensibly follows. Order, not a lock (D13).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<ObjectId>,
    /// Newest `50102` for this item, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_progress: Option<EventIdHex>,
}

// ── §4.3 kind:50100 — draft payloads ─────────────────────────────────────────

/// The draft kinds; the `t` tag of a `50100` (D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DraftKind {
    /// A new root; also the follow-up inside a `review`.
    Project,
    /// A holder for an unheld item.
    Dri,
    /// A child under a held parent.
    Ticket,
    /// "Is this done?" to the holder.
    Done,
    /// A root entering its last fifth.
    Review,
    /// A redraw of the objectives as line operations.
    Objectives,
    /// Mission, vision, or a strategy change heard in talk.
    Direction,
    /// A member's own About & skills, drafted from their DM.
    Profile,
    /// Reserved for the next version.
    Money,
}

/// Where a draft came from; the `origin` tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DraftOrigin {
    /// Heard in a room.
    Talk,
    /// Drafted from the gap between direction and the tree.
    Gap,
}

/// How well an objective line is served by the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Served {
    /// No live item serves it.
    Not,
    /// Something serves part of it.
    Partly,
    /// Covered.
    Served,
}

/// One objective line and what serves it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ObjectiveGap {
    /// `objectives@<version>#<line-id>`.
    #[serde(rename = "ref")]
    pub line_ref: String,
    /// How well it is served.
    pub served: Served,
    /// Items that serve it.
    #[serde(default)]
    pub by: Vec<ObjectId>,
}

/// The fit of a suggested holder, as receipts from their `39105` and history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HolderMatch {
    /// Skill slugs from the person's profile the piece matched on.
    #[serde(default)]
    pub skills: Vec<String>,
    /// A quoted span from their `about`, if one fit.
    pub about: Option<String>,
    /// Items they held before.
    #[serde(default)]
    pub items: Vec<ObjectId>,
}

/// `t = project` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectDraft {
    /// Short name.
    pub title: String,
    /// Markdown brief.
    pub brief: String,
    /// The objective line it serves.
    pub objective_ref: Option<String>,
    /// Proposed end date.
    pub due_at: Timestamp,
    /// Suggested holder, or `null` with the missing capability named in `why`.
    pub suggested_dri: Option<PubkeyHex>,
    /// One line.
    pub why: String,
    /// The gap analysis that produced it.
    #[serde(default)]
    pub gaps: Vec<ObjectiveGap>,
    /// Evidence for `suggested_dri`, when named.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<HolderMatch>,
}

/// `t = dri` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DriDraft {
    /// The unheld item.
    pub item: ObjectId,
    /// The suggested holder.
    pub suggested: PubkeyHex,
    /// One line; adds nothing not in `matched`.
    pub why: String,
    /// The fit as receipts.
    pub matched: HolderMatch,
    /// Event ids backing the suggestion.
    #[serde(default)]
    pub evidence: Vec<EventIdHex>,
}

/// One piece of the ordered plan for a parent brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CoveragePiece {
    /// The phrase in the parent brief this piece answers to.
    pub piece: String,
    /// The live or done item already covering it, if any.
    pub covered_by: Option<ObjectId>,
    /// 1-based position in the plan.
    pub order: u32,
    /// Pieces this one follows.
    #[serde(default)]
    pub after: Vec<String>,
    /// `after <piece>` when a predecessor is neither live nor done; a held piece is never drafted.
    pub held: Option<String>,
}

/// `t = ticket` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TicketDraft {
    /// The held parent.
    pub parent: ObjectId,
    /// Short name.
    pub title: String,
    /// Markdown brief.
    pub brief: String,
    /// Estimated completion.
    pub due_at: Timestamp,
    /// What this piece needs from whoever holds it: skill slugs or one-line capabilities.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Suggested holder, or `null`.
    pub suggested_holder: Option<PubkeyHex>,
    /// Why nobody here fits; required when `requires` is non-empty and `suggested_holder` is `null`.
    pub unfilled: Option<String>,
    /// The phrase in the parent brief this answers to.
    pub covers: String,
    /// Live or done siblings this piece follows; empty when it can start now. Carries into `39101.after`.
    #[serde(default)]
    pub after: Vec<ObjectId>,
    /// `true` when its outcome decides what the later pieces are.
    #[serde(default)]
    pub gate: bool,
    /// The whole ordered plan for the parent brief.
    #[serde(default)]
    pub coverage: Vec<CoveragePiece>,
    /// Evidence for `suggested_holder`, when named.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<HolderMatch>,
}

/// `t = done` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DoneDraft {
    /// The held item.
    pub item: ObjectId,
    /// `last child closed` or `heard on a call`.
    pub why: String,
    /// The message it was heard in, if any.
    pub heard: Option<EventIdHex>,
}

/// One sentence with the ledger rows that ground it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GroundedSentence {
    /// The sentence.
    pub text: String,
    /// Event ids it rests on.
    #[serde(default)]
    pub rows: Vec<EventIdHex>,
}

/// What a `review` draft recommends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReviewRecommendation {
    /// Open a follow-up project.
    FollowUp {
        /// The follow-up, as a `project` payload.
        project: ProjectDraft,
    },
    /// Nothing more to do here.
    NoFurtherWork {
        /// One line.
        why: String,
    },
}

/// `t = review` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReviewDraft {
    /// The root entering its last fifth.
    pub item: ObjectId,
    /// The review brief, sentence by sentence, each grounded.
    pub brief: Vec<GroundedSentence>,
    /// What to do next.
    pub recommendation: ReviewRecommendation,
}

/// One operation on the objectives list — a redraw, never a rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum LineOp {
    /// Drop a line.
    Strike {
        /// The line id.
        id: String,
        /// One line.
        why: String,
    },
    /// Move a line's date.
    Move {
        /// The line id.
        id: String,
        /// The new date.
        date: Timestamp,
        /// One line.
        why: String,
    },
    /// Add a line.
    Add {
        /// The new text.
        text: String,
        /// Its date, when it has one.
        date: Option<Timestamp>,
        /// One line.
        why: String,
        /// The message it came from, if heard.
        source: Option<EventIdHex>,
    },
}

/// `t = objectives` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ObjectivesDraft {
    /// The version the ops apply to.
    pub base_version: u32,
    /// The operations.
    pub ops: Vec<LineOp>,
}

/// `t = direction` payload — mission, vision, or a strategy change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DirectionDraft {
    /// Which artifact.
    pub slug: DirectionSlug,
    /// The version it replaces.
    pub base_version: u32,
    /// The whole new body.
    pub body: String,
    /// The full new line list (`objectives`/`strategy`).
    #[serde(default)]
    pub lines: Vec<DirectionLine>,
    /// Unified or line-op summary.
    pub diff: String,
    /// Messages it was heard in.
    #[serde(default)]
    pub heard: Vec<EventIdHex>,
}

/// `t = profile` payload — settles through that member's own `io_profile_set`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProfileDraft {
    /// The member; must equal the draft's `needs`.
    pub pubkey: PubkeyHex,
    /// Proposed About.
    pub about: String,
    /// Proposed skills, as the person's words.
    #[serde(default)]
    pub skills: Vec<String>,
    /// Proposed open-pieces limit.
    pub open_limit: Option<u32>,
    /// Messages it was heard in.
    #[serde(default)]
    pub heard: Vec<EventIdHex>,
}

/// An amount someone agreed to in talk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AgreedAmount {
    /// Decimal string.
    pub amount: String,
    /// The message where it was agreed.
    pub heard: EventIdHex,
}

/// `t = money` payload — reserved for the next version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MoneyDraft {
    /// The done item.
    pub item: ObjectId,
    /// Who gets paid.
    pub payee: PubkeyHex,
    /// Decimal string.
    pub amount: String,
    /// Currency code.
    pub currency: String,
    /// What was agreed in talk, if anything.
    pub agreed: Option<AgreedAmount>,
}

/// The content of a `50100`, typed by its `t` tag.
///
/// The draft kind is a tag, not a content field, so parsing dispatches on
/// the tag through [`DraftPayload::parse`]. Serialization is transparent —
/// the inner payload is the content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum DraftPayload {
    /// `t = project`.
    Project(ProjectDraft),
    /// `t = dri`.
    Dri(DriDraft),
    /// `t = ticket`.
    Ticket(TicketDraft),
    /// `t = done`.
    Done(DoneDraft),
    /// `t = review`.
    Review(ReviewDraft),
    /// `t = objectives`.
    Objectives(ObjectivesDraft),
    /// `t = direction`.
    Direction(DirectionDraft),
    /// `t = profile`.
    Profile(ProfileDraft),
    /// `t = money`.
    Money(MoneyDraft),
}

impl DraftPayload {
    /// Parse a `50100` content string as the payload for `kind`.
    pub fn parse(kind: DraftKind, content: &str) -> Result<Self, serde_json::Error> {
        Ok(match kind {
            DraftKind::Project => Self::Project(serde_json::from_str(content)?),
            DraftKind::Dri => Self::Dri(serde_json::from_str(content)?),
            DraftKind::Ticket => Self::Ticket(serde_json::from_str(content)?),
            DraftKind::Done => Self::Done(serde_json::from_str(content)?),
            DraftKind::Review => Self::Review(serde_json::from_str(content)?),
            DraftKind::Objectives => Self::Objectives(serde_json::from_str(content)?),
            DraftKind::Direction => Self::Direction(serde_json::from_str(content)?),
            DraftKind::Profile => Self::Profile(serde_json::from_str(content)?),
            DraftKind::Money => Self::Money(serde_json::from_str(content)?),
        })
    }

    /// The `t` tag value this payload belongs under.
    pub fn kind(&self) -> DraftKind {
        match self {
            Self::Project(_) => DraftKind::Project,
            Self::Dri(_) => DraftKind::Dri,
            Self::Ticket(_) => DraftKind::Ticket,
            Self::Done(_) => DraftKind::Done,
            Self::Review(_) => DraftKind::Review,
            Self::Objectives(_) => DraftKind::Objectives,
            Self::Direction(_) => DraftKind::Direction,
            Self::Profile(_) => DraftKind::Profile,
            Self::Money(_) => DraftKind::Money,
        }
    }
}

// ── §4.4 kind:39102 — proposal ───────────────────────────────────────────────

/// What a proposal decides; the `t` tag of a `39102`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProposalKind {
    /// A new version of a direction artifact.
    Direction,
    /// A new root of the tree.
    Project,
    /// A holder for an unheld item.
    Dri,
    /// Reserved: a release from the treasury.
    Money,
    /// Reserved: a new member.
    Join,
    /// The Shaper set, its rules, or its agent.
    Shapers,
}

/// Proposal state machine (Protocol §5.3); the `s` tag of a `39102`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProposalStatus {
    /// Waiting on votes.
    Open,
    /// Rule met and executed.
    Passed,
    /// Can no longer pass.
    Rejected,
    /// `expires_at` passed.
    Expired,
    /// `money` only: the chain confirmed the release.
    Settled,
}

/// A Shaper's vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum VoteChoice {
    /// For.
    Agree,
    /// Against.
    Decline,
}

/// One recorded vote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Vote {
    /// The Shaper.
    pub p: PubkeyHex,
    /// Their choice.
    pub vote: VoteChoice,
    /// When.
    pub at: Timestamp,
    /// The `io_vote` event id.
    pub receipt: EventIdHex,
}

/// What a passed proposal produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Executed {
    /// Object type, e.g. `work_item`, `direction`, `shapers`.
    pub kind: String,
    /// Object id.
    pub id: String,
}

/// `money` settlement record — next version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Settlement {
    /// Chain transaction id.
    pub tx: String,
    /// Chain name.
    pub chain: String,
    /// Contract address.
    pub contract: String,
    /// When confirmed.
    pub at: Timestamp,
    /// The `io_money_released` event id.
    pub receipt: EventIdHex,
    /// Set when the chain rejected the release; the bridge retries.
    pub error: Option<String>,
}

/// How many eligible Shapers must agree: `majority`, `all`, or an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DecisionRule {
    /// A named rule.
    Named(NamedRule),
    /// At least N of `eligible` (capped at `eligible.len()` when resolved).
    AtLeast(u32),
}

/// The two named decision rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NamedRule {
    /// More than half of `eligible`.
    Majority,
    /// Every eligible Shaper.
    All,
}

impl DecisionRule {
    /// `majority` — the default for every proposal kind.
    pub const MAJORITY: Self = Self::Named(NamedRule::Majority);
    /// `all` — always applied to `shapers/rules` and `shapers/agent`.
    pub const ALL: Self = Self::Named(NamedRule::All);

    /// Resolve the rule to `needed` against the eligible set at opening
    /// (Protocol §4.5): `majority` is more than half, `all` is every one, an
    /// integer is capped at `eligible`. With one eligible Shaper every rule
    /// resolves to 1; with none, to 0.
    pub fn needed_for(self, eligible: u32) -> u32 {
        if eligible == 0 {
            return 0;
        }
        match self {
            Self::Named(NamedRule::Majority) => eligible / 2 + 1,
            Self::Named(NamedRule::All) => eligible,
            Self::AtLeast(n) => n.clamp(1, eligible),
        }
    }
}

impl Default for DecisionRule {
    fn default() -> Self {
        Self::MAJORITY
    }
}

/// Content of `kind:39102` — one proposal and its votes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Proposal {
    /// Proposal UUID; the `d` tag.
    pub id: ObjectId,
    /// What it decides.
    pub kind: ProposalKind,
    /// Where it is.
    pub status: ProposalStatus,
    /// Who opened it.
    pub opened_by: PubkeyHex,
    /// When.
    pub opened_at: Timestamp,
    /// `opened_at + 39103.decision_window_secs`.
    pub expires_at: Timestamp,
    /// The `50100` it settled, if any.
    pub draft: Option<EventIdHex>,
    /// The command content that opened it, verbatim.
    #[serde(default)]
    pub payload: serde_json::Value,
    /// The rule that applied when it opened.
    pub rule: DecisionRule,
    /// The rule resolved against `eligible`, at opening.
    pub needed: u32,
    /// Shapers at opening minus the subject; a later change does not move the bar.
    #[serde(default)]
    pub eligible: Vec<PubkeyHex>,
    /// Votes so far; one per Shaper.
    #[serde(default)]
    pub votes: Vec<Vote>,
    /// When it passed, was rejected, or expired.
    pub decided_at: Option<Timestamp>,
    /// What passing produced.
    pub executed: Option<Executed>,
    /// `money` only.
    pub settlement: Option<Settlement>,
}

// ── §4.5 kind:39103 — Shapers and rules ──────────────────────────────────────

/// The `op` tag of an `io_shapers_propose`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ShapersOp {
    /// Offer a seat to `p`.
    Add,
    /// Remove `p`.
    Remove,
    /// Replace `rules` and windows; always passes under `all`.
    Rules,
    /// Replace the org agent (`p`) or return to the hosted default (no `p`); always `all`.
    Agent,
}

/// A passed `shapers/add` the named person has not yet accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OfferedSeat {
    /// The person.
    pub p: PubkeyHex,
    /// The passed proposal.
    pub proposal: ObjectId,
    /// When it passed; lapses after `offer_window_secs`.
    pub at: Timestamp,
}

/// The rule per proposal kind. Every field defaults to `majority`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DecisionRules {
    /// For `direction` proposals.
    #[serde(default)]
    pub direction: DecisionRule,
    /// For `project` proposals.
    #[serde(default)]
    pub project: DecisionRule,
    /// For `dri` proposals.
    #[serde(default)]
    pub dri: DecisionRule,
    /// For `shapers` add and remove; `rules` and `agent` are always `all`.
    #[serde(default)]
    pub shapers: DecisionRule,
    /// Later; inert while money is not enabled.
    #[serde(default)]
    pub money: DecisionRule,
    /// Later; inert while join is not enabled.
    #[serde(default)]
    pub join: DecisionRule,
}

/// Default `decision_window_secs`: 7 days.
pub const DEFAULT_DECISION_WINDOW_SECS: u64 = 604_800;
/// Default `offer_window_secs`: 3 days.
pub const DEFAULT_OFFER_WINDOW_SECS: u64 = 259_200;

/// Content of `kind:39103` — the Shaper set and decision rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Shapers {
    /// The community owner when `39103` was first written.
    pub founder: PubkeyHex,
    /// The current set; never empty.
    pub shapers: Vec<PubkeyHex>,
    /// Passed adds not yet accepted.
    #[serde(default)]
    pub offered: Vec<OfferedSeat>,
    /// The private `#shapers` channel the relay keeps in sync.
    pub room: Option<String>,
    /// The community's org agent; `null` on a sovereign relay until the Shapers name one.
    pub agent: Option<PubkeyHex>,
    /// `true` while `agent` is the relay operator's hosted default.
    #[serde(default)]
    pub agent_hosted: bool,
    /// The rule per proposal kind.
    #[serde(default)]
    pub rules: DecisionRules,
    /// An open proposal that has not passed by then expires.
    pub decision_window_secs: u64,
    /// Unanswered offers renotify at half, return at the end.
    pub offer_window_secs: u64,
    /// When last written.
    pub updated_at: Timestamp,
    /// The command that produced this version.
    pub receipt: EventIdHex,
}

// ── §4.6 kind:39104 — draft outcome ──────────────────────────────────────────

/// Draft outcome (Protocol §5.4); the `s` tag of a `39104`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DraftOutcomeStatus {
    /// Written at birth for every non-shadow draft.
    Open,
    /// A command carrying the draft tag with an equal payload.
    Accepted,
    /// A command carrying the draft tag with a different payload.
    Amended,
    /// An `io_draft_decide` decline.
    Declined,
    /// The `expiration` tag passed.
    Expired,
    /// Shown to nobody; recorded for evaluation only.
    Shadow,
}

/// Why a draft was declined; the `reason` tag of an `io_draft_decide`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeclineReason {
    /// Something already serves this.
    AlreadyCovered,
    /// The agent misread the line.
    NotWhatTheLineMeant,
    /// Should be split.
    TooBig,
    /// Not worth a piece.
    TooSmall,
    /// The named person is not the one.
    WrongHolder,
    /// Right, but later.
    NotNow,
    /// Anything else.
    Other,
}

/// Content of `kind:39104` — the outcome of one draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DraftOutcome {
    /// The `50100` event id; the `d` tag.
    pub draft: EventIdHex,
    /// Where it is.
    pub status: DraftOutcomeStatus,
    /// Who decided; `null` while open.
    pub decided_by: Option<PubkeyHex>,
    /// When; `null` while open.
    pub decided_at: Option<Timestamp>,
    /// The decline reason, if declined.
    pub reason: Option<DeclineReason>,
    /// The command or state the draft turned into.
    pub result: Option<EventIdHex>,
}

// ── §4.7 kind:50101 — health read ────────────────────────────────────────────

/// The three health bands; the `band` tag of a `50101` and an `io_health_rate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthBand {
    /// Needs a Shaper's eye.
    Struggling,
    /// Watch it.
    Wobbly,
    /// Leave it alone.
    Healthy,
}

/// One weighted factor with the rows it counted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HealthFactor {
    /// Fixed by the formula version, e.g. `overdue`, `stalled`.
    pub name: String,
    /// The measured value.
    pub value: f64,
    /// Its weight in the formula.
    pub weight: f64,
    /// Event ids it counted.
    #[serde(default)]
    pub rows: Vec<EventIdHex>,
}

/// Content of `kind:50101` — one project's health for one week.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HealthRead {
    /// The root.
    pub item: ObjectId,
    /// ISO week, e.g. `2026-W38`.
    pub week: String,
    /// 0.0–1.0.
    pub pct: f64,
    /// The band `pct` falls in.
    pub band: HealthBand,
    /// The factors, each grounded.
    pub factors: Vec<HealthFactor>,
    /// The prose, each sentence grounded.
    pub sentences: Vec<GroundedSentence>,
    /// Formula version, e.g. `health-weights@1`.
    pub formula: String,
}

// ── §4.7b kind:50102 — progress note ─────────────────────────────────────────

/// The agent's read of where held work stands; never a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProgressHint {
    /// Moving.
    Progressing,
    /// A candidate for a nudge to the parent's holder.
    Blocked,
    /// A candidate for a `done` draft to the holder.
    Ready,
}

/// One commit a note covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommitRef {
    /// Full sha.
    pub sha: String,
    /// First line of the message.
    pub title: String,
}

/// Work seen in the tree but not pushed; informative only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Uncommitted {
    /// Files changed.
    pub files: u32,
}

/// Maximum `commit` tags on one `50102`.
pub const PROGRESS_MAX_COMMITS: usize = 50;
/// Maximum `summary` length in chars.
pub const PROGRESS_SUMMARY_MAX_CHARS: usize = 1200;

/// Content of `kind:50102` — a progress note on a held item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProgressNote {
    /// The held item.
    pub item: ObjectId,
    /// The holder; equals the signer or the signer's NIP-OA owner.
    pub dri: PubkeyHex,
    /// Window start.
    pub from: Timestamp,
    /// Window end.
    pub to: Timestamp,
    /// Markdown, ≤ [`PROGRESS_SUMMARY_MAX_CHARS`].
    pub summary: String,
    /// The agent's read.
    pub hint: ProgressHint,
    /// The work branch, e.g. `refs/heads/io/7f3a-weekday-hall`.
    #[serde(rename = "ref")]
    pub git_ref: String,
    /// The ref's tip when the note was written.
    pub head: String,
    /// Commits covered, newest first.
    #[serde(default)]
    pub commits: Vec<CommitRef>,
    /// Files changed across `commits`.
    #[serde(default)]
    pub files_changed: u32,
    /// Unpushed work seen in the tree.
    pub uncommitted: Option<Uncommitted>,
    /// Relay-set at ingest: `head` matched the `30618` oid. Absent from the client's note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_verified: Option<bool>,
    /// Relay-set from the push hook, never by the client (D12).
    pub merged_into: Option<String>,
}

// ── §4.7c kind:50103 — agent note ────────────────────────────────────────────

/// The note types; the `t` tag of a `50103`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentNoteKind {
    /// The judge refused a draft before publish.
    DraftDropped,
    /// A trigger ran out of retries or was coalesced away.
    TriggerSkipped,
    /// The agent hit a cap and went to shadow.
    BudgetExhausted,
    /// The Friday rolling tally.
    Tally,
}

/// Counts for one agent move in a tally.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MoveTally {
    /// Drafts published.
    pub opened: u32,
    /// Settled `accepted`.
    pub accepted: u32,
    /// Settled `amended`.
    pub amended: u32,
    /// Declined, by reason.
    #[serde(default)]
    pub declined: BTreeMap<String, u32>,
    /// Dropped by the judge, by reason code.
    #[serde(default)]
    pub dropped: BTreeMap<String, u32>,
    /// Recorded in shadow.
    pub shadow: u32,
}

/// Health counts in a tally.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HealthTally {
    /// `50101` reads published.
    pub reads: u32,
    /// Shaper ratings received.
    pub rated: u32,
    /// Ratings that agreed with the read's band.
    pub agreed: u32,
}

/// Content of `kind:50103` — what the agent did *not* publish, and its tally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "note", rename_all = "snake_case")]
pub enum AgentNote {
    /// The judge refused a draft before publish.
    DraftDropped {
        /// Which move (1–4).
        #[serde(rename = "move")]
        agent_move: u8,
        /// The dedupe key it would have carried.
        gap: String,
        /// The judge's reason code.
        reason: String,
        /// The draft kind it would have been.
        kind: DraftKind,
        /// A pubkey or `shaper`.
        needs: String,
        /// The agent's job id.
        trace: Option<String>,
    },
    /// A trigger ran out of retries, or was coalesced away under load.
    TriggerSkipped {
        /// Which move (1–4).
        #[serde(rename = "move")]
        agent_move: u8,
        /// The trigger name, e.g. `direction_confirmed`.
        trigger: String,
        /// What it was about, e.g. `objectives@4`.
        object: String,
        /// `model_timeout` or `queue_full`.
        reason: String,
        /// The agent's job id.
        trace: Option<String>,
    },
    /// The agent hit a cap and went to shadow.
    BudgetExhausted {
        /// `calls_per_hour` or `tokens_per_day`.
        budget: String,
        /// When it resumes.
        until: Timestamp,
    },
    /// Friday, rolling four weeks.
    Tally {
        /// ISO week.
        week: String,
        /// Window length.
        window_weeks: u32,
        /// Per move (`"1"`, `"2"`, `"3"`).
        #[serde(default)]
        moves: BTreeMap<String, MoveTally>,
        /// Health reads and ratings.
        health: HealthTally,
        /// Drafts nobody decided in five days.
        open_older_than_5d: u32,
        /// `50100`s the relay refused for an unresolved receipt.
        receipt_rejected: u32,
    },
}

impl AgentNote {
    /// The `t` tag value for this note.
    pub fn kind(&self) -> AgentNoteKind {
        match self {
            Self::DraftDropped { .. } => AgentNoteKind::DraftDropped,
            Self::TriggerSkipped { .. } => AgentNoteKind::TriggerSkipped,
            Self::BudgetExhausted { .. } => AgentNoteKind::BudgetExhausted,
            Self::Tally { .. } => AgentNoteKind::Tally,
        }
    }
}

// ── §4.7a kind:39105 — org profile ───────────────────────────────────────────

/// One skill: the relay's kebab-case slug and the person's label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Skill {
    /// Kebab-case of `label`, deduplicated; the `k` tag.
    pub slug: String,
    /// The person's words.
    pub label: String,
}

/// `about` ≤ this many chars.
pub const PROFILE_ABOUT_MAX_CHARS: usize = 1_000;
/// At most this many skills.
pub const PROFILE_MAX_SKILLS: usize = 20;
/// Each skill label ≤ this many chars.
pub const PROFILE_SKILL_LABEL_MAX_CHARS: usize = 40;
/// `open_limit`, when set, is within `1..=50`.
pub const PROFILE_OPEN_LIMIT_RANGE: std::ops::RangeInclusive<u32> = 1..=50;

/// Content of `kind:39105` — one member's org profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OrgProfile {
    /// The member; the `d` tag.
    pub pubkey: PubkeyHex,
    /// Incremented on every `io_profile_set`.
    pub version: u32,
    /// Free text.
    pub about: String,
    /// The person's skills.
    #[serde(default)]
    pub skills: Vec<Skill>,
    /// How many open pieces at once; `null` = no limit.
    pub open_limit: Option<u32>,
    /// When last written.
    pub updated_at: Timestamp,
    /// The `io_profile_set` event id.
    pub receipt: EventIdHex,
}

// ── §4.8 command contents ────────────────────────────────────────────────────

/// `{}` — the content of commands whose tags say everything.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EmptyContent {}

/// `{ why?: "" }` — `50001` add/remove/agent, `50010`, `50011`, `50015`, `50018`, `50020`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WhyContent {
    /// One line, optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// `50001 op=rules` content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RulesContent {
    /// The new rule per kind.
    pub rules: DecisionRules,
    /// New decision window, if changing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_window_secs: Option<u64>,
    /// New offer window, if changing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_window_secs: Option<u64>,
}

/// A line in a proposed direction version: carried forward (`id` set) or new.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DirectionLineInput {
    /// The existing line id when carried forward; absent for a new line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The line.
    pub text: String,
    /// Target date, when it has one.
    pub date: Option<Timestamp>,
}

/// `50002` content — the whole new version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DirectionProposeContent {
    /// The new body.
    pub body: String,
    /// The full new line list (`objectives`/`strategy`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<DirectionLineInput>>,
    /// One line, optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// `50003` content.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VoteContent {
    /// A decline reason of `direction` may trigger a strategy draft.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// `50004` content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectProposeContent {
    /// Short name.
    pub title: String,
    /// Markdown brief.
    pub brief: String,
    /// End date.
    pub due_at: Timestamp,
    /// The objective line it serves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objective_ref: Option<String>,
    /// If named, the root opens `offered` to them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_dri: Option<PubkeyHex>,
}

/// `50005` content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TicketCreateContent {
    /// Short name.
    pub title: String,
    /// Markdown brief.
    pub brief: String,
    /// Estimated completion.
    pub due_at: Timestamp,
    /// Live or done siblings this piece follows; validated, never a lock (D13).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<ObjectId>,
}

/// The `outcome` tag of an `io_draft_decide`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DraftDecision {
    /// A standalone accept (no other command follows).
    Accept,
    /// Decline, with a [`DeclineReason`].
    Decline,
}

/// `50013` content — reserved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MoneyProposeContent {
    /// Decimal string.
    pub amount: String,
    /// Currency code.
    pub currency: String,
    /// Free text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// What was agreed in talk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreed: Option<AgreedAmount>,
}

/// `50014` content — reserved; signed by the treasury bridge key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MoneyReleasedContent {
    /// Chain name.
    pub chain: String,
    /// Contract address.
    pub contract: String,
    /// Decimal string.
    pub amount: String,
    /// Currency code.
    pub currency: String,
}

/// `50016` content — reserved.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct JoinProposeContent {
    /// Free text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// `50021` content — the whole profile; `pubkey` is the signer, never a field.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProfileSetContent {
    /// Free text, ≤ [`PROFILE_ABOUT_MAX_CHARS`].
    pub about: String,
    /// Labels in the person's words; the relay derives slugs.
    #[serde(default)]
    pub skills: Vec<String>,
    /// Within [`PROFILE_OPEN_LIMIT_RANGE`], or absent for no limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_limit: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde_json::{json, Value};

    /// Parse `input`, serialize, parse again: the typed value must survive
    /// and the schema must generate without panicking. Returns the parsed
    /// value and what it serialized to.
    fn reparse<T>(input: Value) -> (T, Value)
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug + JsonSchema,
    {
        let parsed: T = serde_json::from_value(input).expect("parse");
        let emitted = serde_json::to_value(&parsed).expect("serialize");
        let reparsed: T = serde_json::from_value(emitted.clone()).expect("reparse");
        assert_eq!(parsed, reparsed);
        let schema = serde_json::to_value(schemars::schema_for!(T)).expect("schema");
        assert!(
            schema.get("$schema").is_some(),
            "schema for {}",
            std::any::type_name::<T>()
        );
        (parsed, emitted)
    }

    /// [`reparse`] plus: every non-null key the spec example carries must
    /// come back unchanged (`null` keys marked "absent" may be dropped).
    fn round_trip<T>(input: Value) -> T
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug + JsonSchema,
    {
        let (parsed, emitted) = reparse::<T>(input.clone());
        for (key, value) in input.as_object().into_iter().flatten() {
            if !value.is_null() {
                assert_eq!(emitted.get(key), Some(value), "field `{key}` changed");
            }
        }
        parsed
    }

    const PK: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const EV: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const ID: &str = "7f3a0000-0000-4000-8000-000000000001";

    #[test]
    fn direction_artifact_round_trips_and_v1_has_no_prev() {
        let v1: DirectionArtifact = round_trip(json!({
            "slug": "objectives", "version": 1, "body": "md",
            "lines": [{ "n": 1, "id": "l_7f3a", "text": "Hall booked", "date": 1780000000 }],
            "confirmed_by": PK, "confirmed_at": 1, "proposed_by": PK, "proposal": ID
        }));
        assert_eq!(v1.slug, DirectionSlug::Objectives);
        assert_eq!(v1.prev, None);
        let mission: DirectionArtifact = round_trip(json!({
            "slug": "mission", "version": 2, "body": "md", "confirmed_by": PK,
            "confirmed_at": 1, "proposed_by": PK, "proposal": ID, "prev": EV
        }));
        assert!(mission.lines.is_empty());
        assert!(!serde_json::to_string(&mission)
            .unwrap()
            .contains("\"lines\""));
    }

    #[test]
    fn work_item_root_and_child_round_trip() {
        let root: WorkItem = round_trip(json!({
            "id": ID, "parent": null, "root": ID, "depth": 0, "path": [],
            "title": "Weekday hall", "brief": "md", "state": "in_review", "dri": PK,
            "offered_to": null, "offered_by": null, "offered_at": null,
            "due_at": 1785000000, "approved_at": 1757900000,
            "objective_ref": "objectives@3#l_7f3a", "created_from": EV, "draft": EV,
            "done_receipt": null, "closed_by": null,
            "children": { "open": 1, "offered": 0, "accepted": 2, "done": 3 },
            "home": { "channel": ID, "repo": "30617:r:weekday-hall", "project": "30621:r:weekday-hall" }
        }));
        assert_eq!(root.state, WorkItemState::InReview);
        assert_eq!(root.children.done, 3);
        assert!(root.after.is_empty() && root.branch.is_none());

        let child: WorkItem = round_trip(json!({
            "id": ID, "parent": ID, "root": ID, "depth": 1, "path": [ID], "title": "Permit",
            "brief": "", "state": "done", "dri": PK, "offered_by": "agent", "due_at": 1,
            "created_from": EV, "closed_by": "dri", "done_receipt": EV,
            "branch": "io/7f3a-permit", "after": [ID], "last_progress": EV
        }));
        assert_eq!(child.offered_by.as_deref(), Some(OFFERED_BY_AGENT));
        assert_eq!(child.closed_by, Some(ClosedBy::Dri));
        assert_eq!(child.after, vec![ID.to_string()]);
        assert_eq!(child.children, ChildrenCounts::default());
    }

    #[test]
    fn ticket_draft_carries_sequence_and_who_is_needed_fields() {
        let draft: TicketDraft = round_trip(json!({
            "parent": ID, "title": "Permit", "brief": "md", "due_at": 1,
            "requires": ["grant-writing"], "suggested_holder": null,
            "unfilled": "nobody here writes grants", "covers": "get the permit",
            "after": [ID], "gate": true,
            "coverage": [
                { "piece": "permit", "covered_by": null, "order": 1, "after": [], "held": null },
                { "piece": "build", "covered_by": ID, "order": 2, "after": ["permit"], "held": "after permit" }
            ]
        }));
        assert_eq!(draft.requires, vec!["grant-writing"]);
        assert!(draft.gate && draft.suggested_holder.is_none());
        assert_eq!(draft.coverage[1].held.as_deref(), Some("after permit"));
        assert!(draft.matched.is_none());

        // Absent optional fields default: a minimal ticket still parses.
        let minimal = DraftPayload::parse(
            DraftKind::Ticket,
            r#"{"parent":"p","title":"t","brief":"b","due_at":0,"covers":"c"}"#,
        )
        .expect("minimal ticket");
        match minimal {
            DraftPayload::Ticket(t) => {
                assert!(!t.gate && t.after.is_empty() && t.coverage.is_empty())
            }
            other => panic!("wrong kind {other:?}"),
        }
    }

    #[test]
    fn every_draft_kind_parses_through_its_tag() {
        let cases: Vec<(DraftKind, Value)> = vec![
            (
                DraftKind::Project,
                json!({ "title": "t", "brief": "b", "objective_ref": "objectives@3#l", "due_at": 0,
                "suggested_dri": PK, "why": "w", "gaps": [{ "ref": "objectives@3#l", "served": "partly", "by": [ID] }],
                "matched": { "skills": ["x"], "about": null, "items": [] } }),
            ),
            (
                DraftKind::Dri,
                json!({ "item": ID, "suggested": PK, "why": "w",
                "matched": { "skills": ["grant-writing"], "about": "span", "items": [ID] }, "evidence": [EV] }),
            ),
            (
                DraftKind::Done,
                json!({ "item": ID, "why": "last child closed", "heard": null }),
            ),
            (
                DraftKind::Review,
                json!({ "item": ID, "brief": [{ "text": "s", "rows": [EV] }],
                "recommendation": { "type": "no_further_work", "why": "done" } }),
            ),
            (
                DraftKind::Objectives,
                json!({ "base_version": 3, "ops": [
                { "op": "strike", "id": "l_c81d", "why": "" },
                { "op": "move", "id": "l_7f3a", "date": 5, "why": "" },
                { "op": "add", "text": "new", "date": 0, "why": "", "source": EV } ] }),
            ),
            (
                DraftKind::Direction,
                json!({ "slug": "strategy", "base_version": 4, "body": "", "lines": [],
                "diff": "d", "heard": [EV] }),
            ),
            (
                DraftKind::Profile,
                json!({ "pubkey": PK, "about": "", "skills": ["spanish"], "open_limit": 3, "heard": [EV] }),
            ),
            (
                DraftKind::Money,
                json!({ "item": ID, "payee": PK, "amount": "150", "currency": "USDC",
                "agreed": { "amount": "150", "heard": EV } }),
            ),
        ];
        for (kind, value) in cases {
            let payload = DraftPayload::parse(kind, &value.to_string()).expect("parse");
            assert_eq!(payload.kind(), kind);
            assert_eq!(serde_json::to_value(&payload).unwrap(), value, "{kind:?}");
        }
        // A review's follow-up nests a full project payload.
        let (review, _): (ReviewDraft, _) = reparse(json!({ "item": ID, "brief": [],
            "recommendation": { "type": "follow_up", "project": { "title": "t", "brief": "b", "due_at": 0, "why": "w" } } }));
        assert!(matches!(
            review.recommendation,
            ReviewRecommendation::FollowUp { .. }
        ));
        // The wrong tag for a payload is a parse error, not a silent coercion.
        assert!(DraftPayload::parse(DraftKind::Dri, r#"{"item":"x","why":"w"}"#).is_err());
    }

    #[test]
    fn proposal_round_trips_with_integer_and_named_rules() {
        let p: Proposal = round_trip(json!({
            "id": ID, "kind": "shapers", "status": "open", "opened_by": PK, "opened_at": 0,
            "expires_at": 604800, "draft": null, "payload": { "why": "x" }, "rule": "all",
            "needed": 2, "eligible": [PK, PK],
            "votes": [{ "p": PK, "vote": "agree", "at": 1, "receipt": EV }],
            "decided_at": null, "executed": null, "settlement": null
        }));
        assert_eq!(p.rule, DecisionRule::ALL);
        assert_eq!(p.votes[0].vote, VoteChoice::Agree);
        let settled: Proposal = round_trip(json!({
            "id": ID, "kind": "money", "status": "settled", "opened_by": PK, "opened_at": 0,
            "expires_at": 1, "payload": {}, "rule": 2, "needed": 2, "decided_at": 3,
            "executed": { "kind": "money", "id": ID },
            "settlement": { "tx": "0x", "chain": "base", "contract": "0x", "at": 4, "receipt": EV, "error": null }
        }));
        assert_eq!(settled.rule, DecisionRule::AtLeast(2));
        assert_eq!(settled.status, ProposalStatus::Settled);
    }

    #[test]
    fn decision_rule_resolves_needed_per_protocol() {
        // majority: more than half; all: every one; N capped at eligible;
        // one eligible → 1 for every rule; none → 0.
        for (rule, eligible, needed) in [
            (DecisionRule::MAJORITY, 1, 1),
            (DecisionRule::MAJORITY, 2, 2),
            (DecisionRule::MAJORITY, 3, 2),
            (DecisionRule::MAJORITY, 4, 3),
            (DecisionRule::MAJORITY, 0, 0),
            (DecisionRule::ALL, 1, 1),
            (DecisionRule::ALL, 5, 5),
            (DecisionRule::ALL, 0, 0),
            (DecisionRule::AtLeast(3), 5, 3),
            (DecisionRule::AtLeast(7), 5, 5),
            (DecisionRule::AtLeast(0), 5, 1),
            (DecisionRule::AtLeast(7), 1, 1),
            (DecisionRule::AtLeast(2), 0, 0),
        ] {
            assert_eq!(
                rule.needed_for(eligible),
                needed,
                "{rule:?} over {eligible}"
            );
        }
        assert_eq!(
            serde_json::to_value(DecisionRule::default()).unwrap(),
            json!("majority")
        );
        assert_eq!(
            serde_json::to_value(DecisionRule::AtLeast(3)).unwrap(),
            json!(3)
        );
        assert!(serde_json::from_value::<DecisionRule>(json!("most")).is_err());
    }

    #[test]
    fn shapers_round_trips_and_rules_default_to_majority() {
        let s: Shapers = round_trip(json!({
            "founder": PK, "shapers": [PK], "offered": [{ "p": PK, "proposal": ID, "at": 0 }],
            "room": ID, "agent": PK, "agent_hosted": true,
            "rules": { "direction": "majority", "project": "all", "dri": 1, "shapers": "majority",
                       "money": "majority", "join": "majority" },
            "decision_window_secs": 604800, "offer_window_secs": 259200, "updated_at": 0, "receipt": EV
        }));
        assert_eq!(s.rules.project, DecisionRule::ALL);
        assert_eq!(s.rules.dri, DecisionRule::AtLeast(1));
        assert_eq!(s.decision_window_secs, DEFAULT_DECISION_WINDOW_SECS);
        assert_eq!(s.offer_window_secs, DEFAULT_OFFER_WINDOW_SECS);
        // A sovereign relay with no hosted agent and partial rules (the
        // emitted `rules` fills the defaults, so no strict key check).
        let (bare, _): (Shapers, _) = reparse(json!({
            "founder": PK, "shapers": [PK], "rules": { "project": "all" },
            "decision_window_secs": 1, "offer_window_secs": 1, "updated_at": 0, "receipt": EV
        }));
        assert_eq!(bare.agent, None);
        assert!(!bare.agent_hosted);
        assert_eq!(bare.rules.direction, DecisionRule::MAJORITY);
        assert_eq!(bare.rules.project, DecisionRule::ALL);
    }

    #[test]
    fn draft_outcome_and_profile_round_trip() {
        let open: DraftOutcome = round_trip(json!({ "draft": EV, "status": "open",
            "decided_by": null, "decided_at": null, "reason": null, "result": null }));
        assert_eq!(open.status, DraftOutcomeStatus::Open);
        let declined: DraftOutcome = round_trip(json!({ "draft": EV, "status": "declined",
            "decided_by": PK, "decided_at": 1, "reason": "not_what_the_line_meant", "result": EV }));
        assert_eq!(declined.reason, Some(DeclineReason::NotWhatTheLineMeant));

        let profile: OrgProfile = round_trip(
            json!({ "pubkey": PK, "version": 2, "about": "I run the kitchen.",
            "skills": [{ "slug": "grant-writing", "label": "grant writing" }],
            "open_limit": 3, "updated_at": 0, "receipt": EV }),
        );
        assert_eq!(profile.skills[0].slug, "grant-writing");
        assert!(PROFILE_OPEN_LIMIT_RANGE.contains(&profile.open_limit.unwrap()));
    }

    #[test]
    fn health_read_round_trips_with_floats() {
        let h: HealthRead = round_trip(
            json!({ "item": ID, "week": "2026-W38", "pct": 0.62, "band": "wobbly",
            "factors": [{ "name": "overdue", "value": 2.0, "weight": 0.25, "rows": [EV] }],
            "sentences": [{ "text": "Two pieces are past their date.", "rows": [EV] }],
            "formula": "health-weights@1" }),
        );
        assert_eq!(h.band, HealthBand::Wobbly);
        assert_eq!(h.factors[0].weight, 0.25);
    }

    #[test]
    fn progress_note_round_trips_and_keeps_ref_field_name() {
        let n: ProgressNote = round_trip(json!({ "item": ID, "dri": PK, "from": 1, "to": 2,
            "summary": "md", "hint": "ready", "ref": "refs/heads/io/7f3a-weekday-hall", "head": "abc",
            "commits": [{ "sha": "abc", "title": "Add booking form" }], "files_changed": 7,
            "uncommitted": { "files": 2 }, "head_verified": true, "merged_into": null }));
        assert_eq!(n.hint, ProgressHint::Ready);
        assert_eq!(n.git_ref, "refs/heads/io/7f3a-weekday-hall");
        // A client's note carries no relay-set fields.
        let client: ProgressNote = round_trip(json!({ "item": ID, "dri": PK, "from": 1, "to": 2,
            "summary": "", "hint": "blocked", "ref": "refs/heads/x", "head": "h" }));
        assert_eq!(client.head_verified, None);
        assert!(!serde_json::to_string(&client)
            .unwrap()
            .contains("head_verified"));
        assert!(!serde_json::to_string(&client).unwrap().contains("git_ref"));
    }

    #[test]
    fn agent_note_variants_round_trip_through_the_note_field() {
        let dropped: AgentNote =
            round_trip(json!({ "note": "draft_dropped", "move": 2, "gap": "x#rota",
            "reason": "unmatched_skill", "kind": "ticket", "needs": PK, "trace": "t1" }));
        assert_eq!(dropped.kind(), AgentNoteKind::DraftDropped);
        let skipped: AgentNote = round_trip(json!({ "note": "trigger_skipped", "move": 1,
            "trigger": "direction_confirmed", "object": "objectives@4", "reason": "model_timeout", "trace": "t2" }));
        assert_eq!(skipped.kind(), AgentNoteKind::TriggerSkipped);
        let budget: AgentNote = round_trip(
            json!({ "note": "budget_exhausted", "budget": "calls_per_hour", "until": 1 }),
        );
        assert_eq!(budget.kind(), AgentNoteKind::BudgetExhausted);
        let tally: AgentNote = round_trip(
            json!({ "note": "tally", "week": "2026-W38", "window_weeks": 4,
            "moves": { "1": { "opened": 6, "accepted": 3, "amended": 1,
                              "declined": { "already_covered": 1, "not_now": 1 },
                              "dropped": { "unresolved_receipt": 0, "nag": 0, "duplicate": 0 }, "shadow": 0 } },
            "health": { "reads": 4, "rated": 8, "agreed": 7 }, "open_older_than_5d": 0, "receipt_rejected": 0 }),
        );
        assert_eq!(tally.kind(), AgentNoteKind::Tally);
        assert!(serde_json::from_str::<AgentNote>(r#"{"note":"unknown"}"#).is_err());
    }

    #[test]
    fn command_contents_round_trip() {
        assert_eq!(
            serde_json::to_value(EmptyContent::default()).unwrap(),
            json!({})
        );
        let _: EmptyContent = round_trip(json!({}));
        let why: WhyContent = round_trip(json!({ "why": "because" }));
        assert_eq!(why.why.as_deref(), Some("because"));
        assert_eq!(
            serde_json::to_value(WhyContent::default()).unwrap(),
            json!({})
        );
        let (rules, emitted): (RulesContent, _) =
            reparse(json!({ "rules": { "shapers": "all" }, "decision_window_secs": 1 }));
        assert_eq!(rules.rules.shapers, DecisionRule::ALL);
        assert_eq!(rules.offer_window_secs, None);
        assert_eq!(emitted["rules"]["direction"], json!("majority"));
        assert!(emitted.get("offer_window_secs").is_none());
        let dir: DirectionProposeContent = round_trip(json!({ "body": "b",
            "lines": [{ "id": "l_7f3a", "text": "kept", "date": null }, { "text": "new", "date": 1 }], "why": "w" }));
        assert_eq!(dir.lines.as_ref().map(Vec::len), Some(2));
        let _: VoteContent = round_trip(json!({ "reason": "too vague" }));
        let project: ProjectProposeContent =
            round_trip(json!({ "title": "t", "brief": "b", "due_at": 1,
            "objective_ref": "objectives@3#l", "suggested_dri": PK }));
        assert_eq!(project.suggested_dri.as_deref(), Some(PK));
        let ticket: TicketCreateContent =
            round_trip(json!({ "title": "t", "brief": "b", "due_at": 1, "after": [ID] }));
        assert_eq!(ticket.after, vec![ID.to_string()]);
        let _: MoneyProposeContent = round_trip(json!({ "amount": "1", "currency": "USDC",
            "agreed": { "amount": "1", "heard": EV } }));
        let _: MoneyReleasedContent =
            round_trip(json!({ "chain": "c", "contract": "k", "amount": "1", "currency": "USDC" }));
        let _: JoinProposeContent = round_trip(json!({ "note": "n" }));
        let profile: ProfileSetContent =
            round_trip(json!({ "about": "a", "skills": ["hosting events"], "open_limit": 3 }));
        assert_eq!(profile.skills, vec!["hosting events"]);
        assert_eq!(
            serde_json::to_value(ProfileSetContent::default()).unwrap(),
            json!({ "about": "", "skills": [] })
        );
    }

    #[test]
    fn tag_letters_are_single_letters_per_d11() {
        for letter in [
            tag::ITEM,
            tag::PARENT,
            tag::STATUS,
            tag::NEEDS,
            tag::SKILL,
            tag::TYPE,
        ] {
            assert_eq!(letter.len(), 1, "{letter:?} must be one letter");
        }
        assert_eq!(
            [
                tag::ITEM,
                tag::PARENT,
                tag::STATUS,
                tag::NEEDS,
                tag::SKILL,
                tag::TYPE
            ],
            ["i", "u", "s", "n", "k", "t"]
        );
    }
}
