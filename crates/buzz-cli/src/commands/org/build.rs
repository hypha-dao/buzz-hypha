//! Argument → event (or REQ filter) for every `buzz org` verb.
//!
//! Each write goes through one [`buzz_sdk`] `build_io_*`. Each read is one
//! REQ filter (or a two-filter REQ for `drafts show`, which needs the `50100`
//! and its `39104`). Nothing here emits `39100–39105` or `50014`.

use buzz_core::intelligent_org::{
    DeclineReason, DirectionLineInput, DirectionProposeContent, DirectionSlug, DraftDecision,
    DraftOrigin, DraftPayload, HealthBand, ProfileSetContent, ProjectProposeContent, RulesContent,
    TicketCreateContent, VoteChoice, VoteContent, WhyContent,
};
use buzz_core::kind::{
    KIND_IO_ACCEPT, KIND_IO_AGENT_NOTE, KIND_IO_DECLINE, KIND_IO_DIRECTION,
    KIND_IO_DIRECTION_PROPOSE, KIND_IO_DONE, KIND_IO_DRAFT, KIND_IO_DRAFT_DECIDE,
    KIND_IO_DRAFT_OUTCOME, KIND_IO_DRI_PROPOSE, KIND_IO_HEALTH, KIND_IO_HEALTH_RATE,
    KIND_IO_JOIN_PROPOSE, KIND_IO_MONEY_PROPOSE, KIND_IO_MONEY_RELEASED, KIND_IO_OFFER,
    KIND_IO_PROFILE, KIND_IO_PROFILE_SET, KIND_IO_PROJECT_PROPOSE, KIND_IO_PROPOSAL,
    KIND_IO_RELEASE, KIND_IO_REOPEN, KIND_IO_SET_DUE, KIND_IO_SHAPERS, KIND_IO_SHAPERS_PROPOSE,
    KIND_IO_SHAPER_ACCEPT, KIND_IO_SHAPER_STEP_DOWN, KIND_IO_TICKET_CREATE, KIND_IO_VOTE,
    KIND_IO_WORK_ITEM,
};
use buzz_sdk::{
    build_io_accept, build_io_agent_note, build_io_decline, build_io_direction_propose,
    build_io_done, build_io_draft, build_io_draft_decide, build_io_dri_propose,
    build_io_health_rate, build_io_offer, build_io_profile_set, build_io_project_propose,
    build_io_release, build_io_reopen, build_io_set_due, build_io_shaper_accept,
    build_io_shaper_step_down, build_io_shapers_propose, build_io_ticket_create, build_io_vote,
    DraftNeeds, DraftReceipt, IoDraft, Provenance, ShapersProposal,
};
use nostr::{EventBuilder, EventId};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::CliError;
use crate::validate::{parse_event_id, parse_uuid, validate_hex64};

use super::{
    DirectionCmd, DraftNeedsArg, DraftsCmd, HealthCmd, LedgerCmd, OrgCmd, ProfileCmd, ProposalsCmd,
    ShapersCmd, VoteChoiceArg, WorkCmd,
};

/// Default `limit` on every org REQ. Bounds the page the CLI will hold.
pub const DEFAULT_LIMIT: u32 = 200;

/// Person-signed command kinds (`50001–50021`) for a ledger trail.
const IO_COMMAND_KINDS: &[u32] = &[
    KIND_IO_SHAPERS_PROPOSE,
    KIND_IO_DIRECTION_PROPOSE,
    KIND_IO_VOTE,
    KIND_IO_PROJECT_PROPOSE,
    KIND_IO_TICKET_CREATE,
    KIND_IO_OFFER,
    KIND_IO_ACCEPT,
    KIND_IO_DECLINE,
    KIND_IO_DONE,
    KIND_IO_RELEASE,
    KIND_IO_SET_DUE,
    KIND_IO_DRAFT_DECIDE,
    KIND_IO_MONEY_PROPOSE,
    KIND_IO_MONEY_RELEASED,
    KIND_IO_DRI_PROPOSE,
    KIND_IO_JOIN_PROPOSE,
    KIND_IO_HEALTH_RATE,
    KIND_IO_REOPEN,
    KIND_IO_SHAPER_ACCEPT,
    KIND_IO_SHAPER_STEP_DOWN,
    KIND_IO_PROFILE_SET,
];

/// Wave 6 — `org progress note` is C-4. Do not change this string.
pub const PROGRESS_NOT_IMPLEMENTED: &str = "not implemented";

/// What a verb produces: one signed command, one REQ, or a fixed refusal.
#[derive(Debug)]
pub enum OrgPlan {
    /// Sign and `POST /events`.
    Write(EventBuilder),
    /// `POST /query`. `tree_root` is a client-side fence on `39101` (`d` or
    /// `root` tag): the relay cannot express the multi-letter `root` tag.
    Query {
        /// One or two Nostr filters (OR).
        filters: Vec<Value>,
        /// Keep `39101` whose `d` or `root` equals this uuid.
        tree_root: Option<Uuid>,
    },
    /// `org progress note` until wave 6.
    NotImplemented,
}

/// Map a parsed `org` verb onto one builder or one REQ filter.
///
/// `me` is the signer's hex pubkey. Bootstrap uses it as the self-`p`;
/// `my-work`, `profile show` without `-p`, and `drafts list --needs me`
/// put it in the filter.
pub fn plan(cmd: &OrgCmd, me: &str) -> Result<OrgPlan, CliError> {
    match cmd {
        OrgCmd::Bootstrap { why } => plan_bootstrap(me, why.as_deref()),
        OrgCmd::Shapers(sub) => plan_shapers(sub, me),
        OrgCmd::Direction(sub) => plan_direction(sub),
        OrgCmd::Proposals(sub) => plan_proposals(sub),
        OrgCmd::Work(sub) => plan_work(sub, me),
        OrgCmd::Drafts(sub) => plan_drafts(sub, me),
        OrgCmd::Health(sub) => plan_health(sub),
        OrgCmd::Profile(sub) => plan_profile(sub, me),
        OrgCmd::Progress(_) => Ok(OrgPlan::NotImplemented),
        OrgCmd::Ledger(sub) => plan_ledger(sub),
        OrgCmd::Tally => Ok(query(
            json!({
                "kinds": [KIND_IO_AGENT_NOTE],
                "#t": ["tally"],
                "limit": DEFAULT_LIMIT,
            }),
            None,
        )),
    }
}

fn query(filter: Value, tree_root: Option<Uuid>) -> OrgPlan {
    OrgPlan::Query {
        filters: vec![filter],
        tree_root,
    }
}

fn write(builder: Result<EventBuilder, buzz_sdk::SdkError>) -> Result<OrgPlan, CliError> {
    Ok(OrgPlan::Write(builder.map_err(sdk)?))
}

fn sdk(err: buzz_sdk::SdkError) -> CliError {
    CliError::Usage(err.to_string())
}

fn why_content(why: Option<&str>) -> WhyContent {
    WhyContent {
        why: why.filter(|s| !s.is_empty()).map(str::to_string),
    }
}

fn draft_id(hex: Option<&str>) -> Result<Option<EventId>, CliError> {
    hex.map(parse_event_id).transpose()
}

/// First `name` tag as a UUID. Mirrors `handlers/intelligent_org::uuid_tag`.
///
/// `50003` and `50019` store a proposal UUID in `e`. nostr's `Tags::event_ids`
/// and `as_standardized` drop that value; never use them here.
pub fn uuid_tag<'a, I>(tags: I, name: &str) -> Option<Uuid>
where
    I: IntoIterator<Item = &'a [String]>,
{
    tags.into_iter().find_map(|tag| {
        if tag.first().map(String::as_str) != Some(name) {
            return None;
        }
        tag.get(1).and_then(|v| Uuid::parse_str(v).ok())
    })
}

/// Tags of a query-result event as string slices — the same shape `uuid_tag` reads.
pub fn json_tag_slices(event: &Value) -> Vec<Vec<String>> {
    event
        .get("tags")
        .and_then(Value::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| {
                    tag.as_array().map(|parts| {
                        parts
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Keep a `39101` when `--root` matches its `d` or `root` tag (both UUIDs).
pub fn work_item_matches_root(event: &Value, root: Uuid) -> bool {
    let tags = json_tag_slices(event);
    let slices: Vec<&[String]> = tags.iter().map(Vec::as_slice).collect();
    uuid_tag(slices.iter().copied(), "d") == Some(root)
        || uuid_tag(slices.iter().copied(), "root") == Some(root)
}

fn plan_bootstrap(me: &str, why: Option<&str>) -> Result<OrgPlan, CliError> {
    // Owner self-add. vote_agree is false: §6.4 executes immediately; the
    // subject rule is waived. `allow_self_tagging` is inside the SDK builder.
    write(build_io_shapers_propose(
        &ShapersProposal::Add {
            pubkey: me,
            why: why_content(why),
        },
        false,
    ))
}

fn plan_shapers(cmd: &ShapersCmd, me: &str) -> Result<OrgPlan, CliError> {
    match cmd {
        ShapersCmd::List { limit } => Ok(query(
            json!({
                "kinds": [KIND_IO_SHAPERS],
                "limit": limit.unwrap_or(DEFAULT_LIMIT),
            }),
            None,
        )),
        ShapersCmd::Add { pubkey, why } => {
            validate_hex64(pubkey)?;
            write(build_io_shapers_propose(
                &ShapersProposal::Add {
                    pubkey,
                    why: why_content(why.as_deref()),
                },
                true,
            ))
        }
        ShapersCmd::Remove { pubkey, why } => {
            validate_hex64(pubkey)?;
            write(build_io_shapers_propose(
                &ShapersProposal::Remove {
                    pubkey,
                    why: why_content(why.as_deref()),
                },
                true,
            ))
        }
        ShapersCmd::Rules { json } => {
            let rules: RulesContent = serde_json::from_str(json)
                .map_err(|e| CliError::Usage(format!("invalid rules JSON: {e}")))?;
            write(build_io_shapers_propose(
                &ShapersProposal::Rules(rules),
                true,
            ))
        }
        ShapersCmd::Agent { pubkey, why } => {
            if let Some(p) = pubkey {
                validate_hex64(p)?;
            }
            write(build_io_shapers_propose(
                &ShapersProposal::Agent {
                    pubkey: pubkey.as_deref(),
                    why: why_content(why.as_deref()),
                },
                true,
            ))
        }
        ShapersCmd::Accept { proposal } => {
            let id = parse_uuid(proposal)?;
            write(build_io_shaper_accept(id))
        }
        ShapersCmd::StepDown { why } => {
            let _ = me;
            write(build_io_shaper_step_down(&why_content(why.as_deref())))
        }
    }
}

fn plan_direction(cmd: &DirectionCmd) -> Result<OrgPlan, CliError> {
    match cmd {
        DirectionCmd::Show { slug, limit } => {
            let mut filter = json!({
                "kinds": [KIND_IO_DIRECTION],
                "limit": limit.unwrap_or(DEFAULT_LIMIT),
            });
            if let Some(slug) = slug {
                filter["#d"] = json!([slug.as_slug()]);
            }
            Ok(query(filter, None))
        }
        DirectionCmd::History { slug, limit } => Ok(query(
            json!({
                "kinds": [KIND_IO_DIRECTION_PROPOSE],
                "#d": [slug.as_slug()],
                "limit": limit.unwrap_or(DEFAULT_LIMIT),
            }),
            None,
        )),
        DirectionCmd::Propose {
            slug,
            base,
            body,
            lines,
            draft,
            why,
        } => {
            let lines = match lines.as_deref() {
                None => None,
                Some(raw) => Some(
                    serde_json::from_str::<Vec<DirectionLineInput>>(raw)
                        .map_err(|e| CliError::Usage(format!("invalid --lines JSON: {e}")))?,
                ),
            };
            let content = DirectionProposeContent {
                body: body.clone(),
                lines,
                why: why.clone(),
            };
            write(build_io_direction_propose(
                slug.to_core(),
                *base,
                &content,
                draft_id(draft.as_deref())?,
                true,
            ))
        }
    }
}

fn plan_proposals(cmd: &ProposalsCmd) -> Result<OrgPlan, CliError> {
    match cmd {
        ProposalsCmd::List {
            kind,
            status,
            limit,
        } => {
            let mut filter = json!({
                "kinds": [KIND_IO_PROPOSAL],
                "limit": limit.unwrap_or(DEFAULT_LIMIT),
            });
            if let Some(kind) = kind {
                filter["#t"] = json!([kind.clone()]);
            }
            if let Some(status) = status {
                filter["#s"] = json!([status.clone()]);
            }
            Ok(query(filter, None))
        }
        ProposalsCmd::Show { id, limit } => {
            let id = parse_uuid(id)?;
            Ok(query(
                json!({
                    "kinds": [KIND_IO_PROPOSAL],
                    "#d": [id.to_string()],
                    "limit": limit.unwrap_or(1),
                }),
                None,
            ))
        }
        ProposalsCmd::Vote { id, choice, reason } => {
            let id = parse_uuid(id)?;
            write(build_io_vote(
                id,
                choice.to_core(),
                &VoteContent {
                    reason: reason.clone(),
                },
            ))
        }
        ProposalsCmd::ProposeProject {
            title,
            brief,
            due_at,
            objective_ref,
            suggested_dri,
            draft,
        } => {
            if let Some(p) = suggested_dri {
                validate_hex64(p)?;
            }
            let content = ProjectProposeContent {
                title: title.clone(),
                brief: brief.clone(),
                due_at: *due_at,
                objective_ref: objective_ref.clone(),
                suggested_dri: suggested_dri.clone(),
            };
            write(build_io_project_propose(
                &content,
                draft_id(draft.as_deref())?,
                true,
            ))
        }
        ProposalsCmd::ProposeDri {
            item,
            pubkey,
            why,
            draft,
        } => {
            let item = parse_uuid(item)?;
            validate_hex64(pubkey)?;
            write(build_io_dri_propose(
                item,
                pubkey,
                &why_content(why.as_deref()),
                draft_id(draft.as_deref())?,
                true,
            ))
        }
    }
}

fn plan_work(cmd: &WorkCmd, me: &str) -> Result<OrgPlan, CliError> {
    match cmd {
        WorkCmd::Tree { root, limit } => {
            let root = root.as_deref().map(parse_uuid).transpose()?;
            Ok(query(
                json!({
                    "kinds": [KIND_IO_WORK_ITEM],
                    "limit": limit.unwrap_or(DEFAULT_LIMIT),
                }),
                root,
            ))
        }
        WorkCmd::Show { item, limit } => {
            let item = parse_uuid(item)?;
            Ok(query(
                json!({
                    "kinds": [KIND_IO_WORK_ITEM],
                    "#d": [item.to_string()],
                    "limit": limit.unwrap_or(1),
                }),
                None,
            ))
        }
        WorkCmd::Create {
            parent,
            title,
            brief,
            due_at,
            offer_to,
            after,
            draft,
        } => {
            let parent = parse_uuid(parent)?;
            if let Some(p) = offer_to {
                validate_hex64(p)?;
            }
            let after = after
                .iter()
                .map(|s| parse_uuid(s).map(|u| u.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            let content = TicketCreateContent {
                title: title.clone(),
                brief: brief.clone(),
                due_at: *due_at,
                after,
            };
            write(build_io_ticket_create(
                parent,
                offer_to.as_deref(),
                &content,
                draft_id(draft.as_deref())?,
            ))
        }
        WorkCmd::Offer {
            item,
            pubkey,
            draft,
        } => {
            let item = parse_uuid(item)?;
            validate_hex64(pubkey)?;
            write(build_io_offer(item, pubkey, draft_id(draft.as_deref())?))
        }
        WorkCmd::Accept { item } => write(build_io_accept(parse_uuid(item)?)),
        WorkCmd::Decline { item } => write(build_io_decline(parse_uuid(item)?)),
        WorkCmd::Done {
            item,
            receipt,
            draft,
        } => write(build_io_done(
            parse_uuid(item)?,
            draft_id(receipt.as_deref())?,
            draft_id(draft.as_deref())?,
        )),
        WorkCmd::Release { item, why } => write(build_io_release(
            parse_uuid(item)?,
            &why_content(why.as_deref()),
        )),
        WorkCmd::SetDue { item, due, why } => write(build_io_set_due(
            parse_uuid(item)?,
            *due,
            &why_content(why.as_deref()),
        )),
        WorkCmd::Reopen { item, why } => write(build_io_reopen(
            parse_uuid(item)?,
            &why_content(why.as_deref()),
        )),
        WorkCmd::MyWork { limit } => Ok(query(
            json!({
                "kinds": [KIND_IO_WORK_ITEM],
                "#p": [me],
                "limit": limit.unwrap_or(DEFAULT_LIMIT),
            }),
            None,
        )),
    }
}

fn plan_drafts(cmd: &DraftsCmd, me: &str) -> Result<OrgPlan, CliError> {
    match cmd {
        DraftsCmd::List { needs, limit } => {
            let mut filter = json!({
                "kinds": [KIND_IO_DRAFT, KIND_IO_DRAFT_OUTCOME],
                "limit": limit.unwrap_or(DEFAULT_LIMIT),
            });
            match needs {
                Some(DraftNeedsArg::Me) => {
                    filter["kinds"] = json!([KIND_IO_DRAFT]);
                    filter["#n"] = json!([me]);
                }
                Some(DraftNeedsArg::Shaper) => {
                    filter["kinds"] = json!([KIND_IO_DRAFT]);
                    filter["#n"] = json!(["shaper"]);
                }
                None => {}
            }
            Ok(query(filter, None))
        }
        DraftsCmd::Show { id, limit } => {
            let id = parse_event_id(id)?;
            let hex = id.to_hex();
            Ok(OrgPlan::Query {
                filters: vec![
                    json!({ "ids": [hex], "limit": 1 }),
                    json!({
                        "kinds": [KIND_IO_DRAFT_OUTCOME],
                        "#d": [hex],
                        "limit": limit.unwrap_or(1),
                    }),
                ],
                tree_root: None,
            })
        }
        DraftsCmd::Decide {
            id,
            outcome,
            reason,
        } => {
            let id = parse_event_id(id)?;
            write(build_io_draft_decide(
                id,
                outcome.to_core(),
                reason.map(super::DeclineReasonArg::to_core),
            ))
        }
        DraftsCmd::Publish { json } => plan_draft_publish(json),
    }
}

#[derive(Debug, Deserialize)]
struct DraftPublishInput {
    needs: String,
    t: buzz_core::intelligent_org::DraftKind,
    #[serde(rename = "move")]
    agent_move: u8,
    origin: DraftOrigin,
    gap: String,
    receipts: Vec<DraftReceiptJson>,
    #[serde(default)]
    shadow: bool,
    expiration: Option<u64>,
    prompt: Option<String>,
    model: Option<String>,
    trace: Option<String>,
    payload: Value,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DraftReceiptJson {
    Event {
        event: String,
    },
    Coordinate {
        coordinate: String,
    },
    LineRef {
        #[serde(rename = "ref")]
        line_ref: String,
    },
}

fn plan_draft_publish(raw: &str) -> Result<OrgPlan, CliError> {
    let input: DraftPublishInput = serde_json::from_str(raw)
        .map_err(|e| CliError::Usage(format!("invalid draft JSON: {e}")))?;
    let payload_json = serde_json::to_string(&input.payload)
        .map_err(|e| CliError::Usage(format!("draft payload: {e}")))?;
    let payload = DraftPayload::parse(input.t, &payload_json)
        .map_err(|e| CliError::Usage(format!("draft payload: {e}")))?;
    let receipts: Vec<DraftReceipt<'_>> = input
        .receipts
        .iter()
        .map(|r| match r {
            DraftReceiptJson::Event { event } => parse_event_id(event).map(DraftReceipt::Event),
            DraftReceiptJson::Coordinate { coordinate } => {
                Ok(DraftReceipt::Coordinate(coordinate.as_str()))
            }
            DraftReceiptJson::LineRef { line_ref } => Ok(DraftReceipt::LineRef(line_ref.as_str())),
        })
        .collect::<Result<_, _>>()?;
    let needs = if input.needs == "shaper" {
        DraftNeeds::Shaper
    } else {
        validate_hex64(&input.needs)?;
        DraftNeeds::Member(input.needs.as_str())
    };
    let draft = IoDraft {
        needs,
        payload: &payload,
        agent_move: input.agent_move,
        origin: input.origin,
        gap: &input.gap,
        receipts: &receipts,
        shadow: input.shadow,
        expiration: input.expiration,
        provenance: Provenance {
            prompt: input.prompt.as_deref(),
            model: input.model.as_deref(),
            trace: input.trace.as_deref(),
        },
    };
    write(build_io_draft(&draft))
}

fn plan_health(cmd: &HealthCmd) -> Result<OrgPlan, CliError> {
    match cmd {
        HealthCmd::Show { item, limit } => {
            let item = parse_uuid(item)?;
            Ok(query(
                json!({
                    "kinds": [KIND_IO_HEALTH],
                    "#i": [item.to_string()],
                    "limit": limit.unwrap_or(DEFAULT_LIMIT),
                }),
                None,
            ))
        }
        HealthCmd::Rate { item, week, band } => write(build_io_health_rate(
            parse_uuid(item)?,
            week,
            band.to_core(),
        )),
    }
}

fn plan_profile(cmd: &ProfileCmd, me: &str) -> Result<OrgPlan, CliError> {
    match cmd {
        ProfileCmd::Show { pubkey, limit } => {
            let d = match pubkey {
                Some(p) => {
                    validate_hex64(p)?;
                    p.as_str()
                }
                None => me,
            };
            Ok(query(
                json!({
                    "kinds": [KIND_IO_PROFILE],
                    "#d": [d],
                    "limit": limit.unwrap_or(1),
                }),
                None,
            ))
        }
        ProfileCmd::Set {
            about,
            skill,
            limit,
            draft,
        } => {
            let content = ProfileSetContent {
                about: about.clone(),
                skills: skill.clone(),
                open_limit: *limit,
            };
            write(build_io_profile_set(&content, draft_id(draft.as_deref())?))
        }
        ProfileCmd::WhoCan { skill, limit } => Ok(query(
            json!({
                "kinds": [KIND_IO_PROFILE],
                "#k": [skill],
                "limit": limit.unwrap_or(DEFAULT_LIMIT),
            }),
            None,
        )),
    }
}

fn plan_ledger(cmd: &LedgerCmd) -> Result<OrgPlan, CliError> {
    match cmd {
        LedgerCmd::List { item, since, limit } => {
            let mut filter = json!({
                "kinds": IO_COMMAND_KINDS,
                "limit": limit.unwrap_or(DEFAULT_LIMIT),
            });
            if let Some(item) = item {
                let item = parse_uuid(item)?;
                filter["#i"] = json!([item.to_string()]);
            }
            if let Some(since) = since {
                filter["since"] = json!(since);
            }
            Ok(query(filter, None))
        }
        LedgerCmd::Note { json } => {
            let mut value: Value = serde_json::from_str(json)
                .map_err(|e| CliError::Usage(format!("invalid note JSON: {e}")))?;
            let item = value
                .as_object_mut()
                .and_then(|obj| obj.remove("item"))
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| CliError::Usage("note item must be a uuid string".into()))
                        .and_then(parse_uuid)
                })
                .transpose()?;
            let note: buzz_core::intelligent_org::AgentNote = serde_json::from_value(value)
                .map_err(|e| CliError::Usage(format!("invalid note JSON: {e}")))?;
            write(build_io_agent_note(&note, item))
        }
    }
}

impl super::DirectionSlugArg {
    fn as_slug(self) -> &'static str {
        match self {
            Self::Mission => "mission",
            Self::Vision => "vision",
            Self::Objectives => "objectives",
            Self::Strategy => "strategy",
        }
    }

    fn to_core(self) -> DirectionSlug {
        match self {
            Self::Mission => DirectionSlug::Mission,
            Self::Vision => DirectionSlug::Vision,
            Self::Objectives => DirectionSlug::Objectives,
            Self::Strategy => DirectionSlug::Strategy,
        }
    }
}

impl VoteChoiceArg {
    fn to_core(self) -> VoteChoice {
        match self {
            Self::Agree => VoteChoice::Agree,
            Self::Decline => VoteChoice::Decline,
        }
    }
}

impl super::DraftDecisionArg {
    fn to_core(self) -> DraftDecision {
        match self {
            Self::Accept => DraftDecision::Accept,
            Self::Decline => DraftDecision::Decline,
        }
    }
}

impl super::DeclineReasonArg {
    fn to_core(self) -> DeclineReason {
        match self {
            Self::AlreadyCovered => DeclineReason::AlreadyCovered,
            Self::NotWhatTheLineMeant => DeclineReason::NotWhatTheLineMeant,
            Self::TooBig => DeclineReason::TooBig,
            Self::TooSmall => DeclineReason::TooSmall,
            Self::WrongHolder => DeclineReason::WrongHolder,
            Self::NotNow => DeclineReason::NotNow,
            Self::Other => DeclineReason::Other,
        }
    }
}

impl super::HealthBandArg {
    fn to_core(self) -> HealthBand {
        match self {
            Self::Struggling => HealthBand::Struggling,
            Self::Wobbly => HealthBand::Wobbly,
            Self::Healthy => HealthBand::Healthy,
        }
    }
}
