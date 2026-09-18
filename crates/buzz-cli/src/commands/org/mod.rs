//! `buzz org` — the intelligent-organization CLI (Development plan C-2).
//!
//! Every verb maps to one [`buzz_sdk`] `build_io_*` or one REQ filter.
//! State kinds `39100–39105` and the treasury bridge's `50014` are never
//! emitted. `org progress note` is wave 6 and refuses with a fixed message.

mod build;

use clap::Subcommand;

use crate::client::{normalize_events, normalize_write_response, BuzzClient};
use crate::error::CliError;
use crate::OutputFormat;

pub use build::{plan, work_item_matches_root, OrgPlan, PROGRESS_NOT_IMPLEMENTED};

/// Top-level `buzz org` verbs (Development plan § CLI surface).
#[derive(Subcommand, Debug)]
pub enum OrgCmd {
    /// Owner self-add — the first Shaper seat (`50001 op=add`)
    Bootstrap {
        /// Optional one-line why
        #[arg(long)]
        why: Option<String>,
    },
    /// Shaper set, rules, and the hosted agent
    #[command(subcommand)]
    Shapers(ShapersCmd),
    /// Mission, vision, objectives, strategy
    #[command(subcommand)]
    Direction(DirectionCmd),
    /// Open and vote on proposals
    #[command(subcommand)]
    Proposals(ProposalsCmd),
    /// The work tree
    #[command(subcommand)]
    Work(WorkCmd),
    /// Agent drafts and person-signed drafts
    #[command(subcommand)]
    Drafts(DraftsCmd),
    /// Health reads and Shaper ratings
    #[command(subcommand)]
    Health(HealthCmd),
    /// About & skills
    #[command(subcommand)]
    Profile(ProfileCmd),
    /// Progress notes (wave 6 — not implemented)
    #[command(subcommand)]
    Progress(ProgressCmd),
    /// Command trail and agent notes
    #[command(subcommand)]
    Ledger(LedgerCmd),
    /// Friday tally (`50103` `#t=tally`)
    Tally,
}

/// `buzz org shapers …`
#[derive(Subcommand, Debug)]
pub enum ShapersCmd {
    /// Current `39103`
    List {
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Offer a seat (`50001 op=add`)
    Add {
        /// Pubkey to offer (64-char hex)
        pubkey: String,
        #[arg(long)]
        why: Option<String>,
    },
    /// Remove a Shaper (`50001 op=remove`)
    Remove {
        /// Pubkey to remove (64-char hex)
        pubkey: String,
        #[arg(long)]
        why: Option<String>,
    },
    /// Replace decision rules (`50001 op=rules`)
    Rules {
        /// `RulesContent` JSON
        json: String,
    },
    /// Name a self-run agent, or omit `p` to return to hosted (`50001 op=agent`)
    Agent {
        /// New agent pubkey; omit for the hosted default
        pubkey: Option<String>,
        #[arg(long)]
        why: Option<String>,
    },
    /// Take an offered seat (`50019`)
    Accept {
        /// Passed `shapers/add` proposal UUID
        proposal: String,
    },
    /// Leave the Shaper set (`50020`)
    #[command(name = "step-down")]
    StepDown {
        #[arg(long)]
        why: Option<String>,
    },
}

/// Direction slug.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum DirectionSlugArg {
    Mission,
    Vision,
    Objectives,
    Strategy,
}

/// `buzz org direction …`
#[derive(Subcommand, Debug)]
pub enum DirectionCmd {
    /// Live `39100` heads
    Show {
        /// One of mission, vision, objectives, strategy
        slug: Option<DirectionSlugArg>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// `50002` commands for a slug
    History {
        slug: DirectionSlugArg,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Propose a new version (`50002`)
    Propose {
        slug: DirectionSlugArg,
        /// Version this edit is based on
        #[arg(long)]
        base: u32,
        /// New body. Use '-' to read from stdin
        #[arg(long)]
        body: String,
        /// JSON array of `{id?, text, date?}` lines
        #[arg(long)]
        lines: Option<String>,
        /// Draft event id this command settles
        #[arg(long)]
        draft: Option<String>,
        #[arg(long)]
        why: Option<String>,
    },
}

/// Vote on a proposal.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum VoteChoiceArg {
    Agree,
    Decline,
}

/// `buzz org proposals …`
#[derive(Subcommand, Debug)]
pub enum ProposalsCmd {
    /// Live `39102`
    List {
        /// Proposal kind: direction, project, dri, shapers, money, join
        #[arg(long)]
        kind: Option<String>,
        /// Proposal status: open, passed, rejected, expired
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// One proposal by UUID
    Show {
        /// Proposal UUID (`d` tag)
        id: String,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Cast a vote (`50003`)
    Vote {
        /// Proposal UUID
        id: String,
        choice: VoteChoiceArg,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Open a project (`50004`)
    #[command(name = "propose-project")]
    ProposeProject {
        #[arg(long)]
        title: String,
        #[arg(long)]
        brief: String,
        #[arg(long)]
        due_at: u64,
        #[arg(long)]
        objective_ref: Option<String>,
        #[arg(long)]
        suggested_dri: Option<String>,
        #[arg(long)]
        draft: Option<String>,
    },
    /// Name a DRI (`50015`)
    #[command(name = "propose-dri")]
    ProposeDri {
        /// Work item UUID
        item: String,
        /// Proposed holder (64-char hex)
        pubkey: String,
        #[arg(long)]
        why: Option<String>,
        #[arg(long)]
        draft: Option<String>,
    },
}

/// `buzz org work …`
#[derive(Subcommand, Debug)]
pub enum WorkCmd {
    /// All `39101` (assembled client-side). `--root` fences after the REQ.
    Tree {
        /// Keep the root and its descendants
        #[arg(long)]
        root: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// One item
    Show {
        item: String,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Create a child (`50005`)
    Create {
        /// Parent item UUID
        parent: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        brief: String,
        #[arg(long)]
        due_at: u64,
        #[arg(long)]
        offer_to: Option<String>,
        #[arg(long = "after")]
        after: Vec<String>,
        #[arg(long)]
        draft: Option<String>,
    },
    /// Offer an item (`50006`)
    Offer {
        item: String,
        pubkey: String,
        #[arg(long)]
        draft: Option<String>,
    },
    /// Accept an offer (`50007`)
    Accept { item: String },
    /// Decline an offer (`50008`)
    Decline { item: String },
    /// Mark done (`50009`)
    Done {
        item: String,
        /// Chat message this done was relayed from
        #[arg(long)]
        receipt: Option<String>,
        #[arg(long)]
        draft: Option<String>,
    },
    /// Release a hold (`50010`)
    Release {
        item: String,
        #[arg(long)]
        why: Option<String>,
    },
    /// Move the due date (`50011`)
    #[command(name = "set-due")]
    SetDue {
        item: String,
        #[arg(long)]
        due: u64,
        #[arg(long)]
        why: Option<String>,
    },
    /// Undo a done within 7 days (`50018`)
    Reopen {
        item: String,
        #[arg(long)]
        why: Option<String>,
    },
    /// Items that name me (`#p`)
    #[command(name = "my-work")]
    MyWork {
        #[arg(long)]
        limit: Option<u32>,
    },
}

/// Who a draft list is filtered to.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum DraftNeedsArg {
    Me,
    Shaper,
}

/// Draft decide outcome.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum DraftDecisionArg {
    Accept,
    Decline,
}

/// Why a draft was declined.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum DeclineReasonArg {
    #[value(name = "already_covered")]
    AlreadyCovered,
    #[value(name = "not_what_the_line_meant")]
    NotWhatTheLineMeant,
    #[value(name = "too_big")]
    TooBig,
    #[value(name = "too_small")]
    TooSmall,
    #[value(name = "wrong_holder")]
    WrongHolder,
    #[value(name = "not_now")]
    NotNow,
    Other,
}

/// `buzz org drafts …`
#[derive(Subcommand, Debug)]
pub enum DraftsCmd {
    /// Open drafts and outcomes (`50100` + `39104`)
    List {
        #[arg(long, value_enum)]
        needs: Option<DraftNeedsArg>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// One draft and its `39104`
    Show {
        /// Draft event id (64-char hex)
        id: String,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Accept or decline without another command (`50012`)
    Decide {
        /// Draft event id
        id: String,
        outcome: DraftDecisionArg,
        #[arg(long, value_enum)]
        reason: Option<DeclineReasonArg>,
    },
    /// Person-signed `50100`
    Publish {
        /// Draft JSON (see TESTING.md). Use '-' to read from stdin
        json: String,
    },
}

/// Health band.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum HealthBandArg {
    Struggling,
    Wobbly,
    Healthy,
}

/// `buzz org health …`
#[derive(Subcommand, Debug)]
pub enum HealthCmd {
    /// Agent health reads (`50101`)
    Show {
        item: String,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// A Shaper's blind band (`50017`)
    Rate {
        item: String,
        /// ISO week, e.g. 2026-W38
        week: String,
        band: HealthBandArg,
    },
}

/// `buzz org profile …`
#[derive(Subcommand, Debug)]
pub enum ProfileCmd {
    /// Live `39105`
    Show {
        /// Member pubkey; omit for yourself
        pubkey: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Replace your profile (`50021`)
    Set {
        #[arg(long)]
        about: String,
        #[arg(long = "skill")]
        skill: Vec<String>,
        /// Open-work limit
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        draft: Option<String>,
    },
    /// Who lists a skill (`{kinds:[39105], "#k":[…]}`)
    #[command(name = "who-can")]
    WhoCan {
        skill: String,
        #[arg(long)]
        limit: Option<u32>,
    },
}

/// `buzz org progress …` — wave 6.
#[derive(Subcommand, Debug)]
pub enum ProgressCmd {
    /// Post a progress note (`50102`) — not implemented until wave 6
    Note,
}

/// `buzz org ledger …`
#[derive(Subcommand, Debug)]
pub enum LedgerCmd {
    /// Commands, optionally by item
    List {
        #[arg(long)]
        item: Option<String>,
        #[arg(long)]
        since: Option<i64>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Agent note (`50103`)
    Note {
        /// `AgentNote` JSON, optional `"item"` uuid field
        json: String,
    },
}

/// Run a parsed `org` verb.
pub async fn dispatch(
    cmd: OrgCmd,
    client: &BuzzClient,
    format: &OutputFormat,
) -> Result<(), CliError> {
    let cmd = materialize(cmd)?;
    let me = client.keys().public_key().to_hex();
    match plan(&cmd, &me)? {
        OrgPlan::Write(builder) => {
            let event = client.sign_event(builder)?;
            let resp = client.submit_event(event).await?;
            println!("{}", normalize_write_response(&resp));
            Ok(())
        }
        OrgPlan::Query { filters, tree_root } => {
            let resp = client.query_multi(&filters).await?;
            let mut events: Vec<serde_json::Value> =
                serde_json::from_str(&resp).unwrap_or_default();
            if let Some(root) = tree_root {
                events.retain(|e| work_item_matches_root(e, root));
            }
            let normalized = normalize_events(&events);
            println!("{}", format_events(&normalized, format));
            Ok(())
        }
        OrgPlan::NotImplemented => Err(CliError::Other(PROGRESS_NOT_IMPLEMENTED.into())),
    }
}

/// Resolve `-` stdin placeholders so `plan` stays pure.
fn materialize(cmd: OrgCmd) -> Result<OrgCmd, CliError> {
    use crate::validate::read_or_stdin;
    Ok(match cmd {
        OrgCmd::Direction(DirectionCmd::Propose {
            slug,
            base,
            body,
            lines,
            draft,
            why,
        }) => OrgCmd::Direction(DirectionCmd::Propose {
            slug,
            base,
            body: read_or_stdin(&body)?,
            lines: match lines {
                Some(raw) => Some(read_or_stdin(&raw)?),
                None => None,
            },
            draft,
            why,
        }),
        OrgCmd::Shapers(ShapersCmd::Rules { json }) => OrgCmd::Shapers(ShapersCmd::Rules {
            json: read_or_stdin(&json)?,
        }),
        OrgCmd::Drafts(DraftsCmd::Publish { json }) => OrgCmd::Drafts(DraftsCmd::Publish {
            json: read_or_stdin(&json)?,
        }),
        OrgCmd::Ledger(LedgerCmd::Note { json }) => OrgCmd::Ledger(LedgerCmd::Note {
            json: read_or_stdin(&json)?,
        }),
        other => other,
    })
}

fn format_events(normalized: &str, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Compact => {
            let events: Vec<serde_json::Value> =
                serde_json::from_str(normalized).unwrap_or_default();
            let compact: Vec<serde_json::Value> = events
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.get("id").cloned().unwrap_or_default(),
                        "content": e.get("content").cloned().unwrap_or_default(),
                        "created_at": e.get("created_at").cloned().unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::to_string(&compact).unwrap_or_default()
        }
        OutputFormat::Json => normalized.to_string(),
    }
}

#[cfg(test)]
mod tests;
