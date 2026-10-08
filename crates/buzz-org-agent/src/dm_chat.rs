//! Live conversation in every channel, and in each member's DM with the org agent.
//!
//! The DM with the org agent answers every line. Everywhere else — `#shapers`,
//! project rooms, and any other channel or DM — the agent hears the room and
//! answers only when the latest line asks it something or is about the work
//! it drafts.
//! The operator runs this process (Venice / `OPENAI_COMPAT_*` from the
//! environment). Members do not configure a runtime. The opening says what
//! the agent can do; later replies go through the operator's chat model.

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::Stdio;
use std::time::Duration;

use buzz_core::nip10::parse_thread_markers;
use buzz_ws_client::error::WsClientError;
use buzz_ws_client::message::RelayMessage;
use buzz_ws_client::NostrWsConnection;
use nostr::{Event, Keys, Tag};
use serde_json::{json, Value};

use crate::chat_act::{self, Board, RawAct, ResolvedAct};
use crate::config::Config;
use crate::relay::publish::{sign, sign_at, Permitted, PublishError};
use crate::state::{OrgState, Transition};

/// Sidebar / kind:0 name. Protocol §6.8.
pub const ORG_AGENT_NAME: &str = "Org. Agent";

/// Posted once into an empty agent DM. Same words as
/// `ORG_AGENT_FIRST_MESSAGE` in `desktop/src/features/org/orgAgentOpening.ts`.
pub const WELCOME_LINES: &[&str] = &[
    r#"Hey, I'm glad to connect! I'm a powerful AI agent, that can take your organization to the next level. View me as Elon Musk on steroids at your service.

Do you have time to set up your organization now?"#,
];

/// Openings this process used to post. They are not an answer.
const LEGACY_WELCOME_LINES: &[&str] = &[
    r#"I draft for this organization — what it is for, the work, who decides, and your profile. You decide what becomes real. Nothing I write changes the org until the right person agrees.

Direction — Mission, vision, where you stand, objectives, and strategy. I draft each one and say what's weak.
Work — Projects, tickets, and who should hold them. Only the named person accepts.
Shapers — Who decides, and how many of them must agree before something passes.
Profile — What you do, the work you want, and your links, so offers go to the right person.
Questions — Ask about anything the organization has already written down."#,
    "Hey. I'm Org. Agent.",
    "I draft, you decide. Are you shaping this alone, or with other people?",
    r#"I draft. You decide. Nothing is real until the right person agrees.

Direction — Mission, vision, objectives, and strategy.
Work — Projects, tickets, and who holds them.
Shapers — Who decides, and how many must agree.
Profile — About you, the work you want, and your links.
Questions — Answers from what's already written down.

Are you shaping this alone, or with other people?"#,
    r#"I draft. You decide. Nothing is real until the right person agrees.

Direction — Mission, vision, where you stand, objectives, and strategy, with an honest read on each.
Work — Projects, tickets, and who holds them.
Shapers — Who decides, and how many must agree.
Profile — About you, the work you want, and your links.
Questions — Answers from what's already written down.

Are you shaping this alone, or with other people?"#,
];

fn is_welcome_line(content: &str) -> bool {
    WELCOME_LINES.contains(&content) || LEGACY_WELCOME_LINES.contains(&content)
}

/// The current opener still needs posting. A legacy canned opening does not
/// count, and a DM that already has a person or a real agent reply is left
/// alone.
fn owes_welcome(room: &Room, me: &str) -> bool {
    if room.messages.iter().any(|line| {
        line.author.eq_ignore_ascii_case(me) && WELCOME_LINES.contains(&line.content.as_str())
    }) {
        return false;
    }
    !room.messages.iter().any(|line| {
        if line.author.eq_ignore_ascii_case(me) {
            !is_welcome_line(&line.content)
        } else {
            true
        }
    })
}

/// Enough turns for one walk through direction, mission to strategy, with
/// the org's situation still in view when objectives are drafted.
const MAX_HISTORY: usize = 24;

/// A `39000` whose identity is one other person is the member's DM with us.
/// The relay omits the org agent from that identity (Protocol §6.8). A DM
/// that still lists us plus exactly one other person is the same conversation.
pub fn is_agent_dm(my_pubkey: &str, p_tags: &[String]) -> bool {
    let mine = my_pubkey.trim().to_ascii_lowercase();
    let others: Vec<&str> = p_tags
        .iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty() && tag.to_ascii_lowercase() != mine)
        .collect();
    let lists_me = p_tags
        .iter()
        .any(|tag| tag.trim().to_ascii_lowercase() == mine);
    if lists_me {
        others.len() == 1
    } else {
        p_tags.len() == 1 && others.len() == 1
    }
}

/// Drop a reasoning fence some chat models emit before the sentence.
pub fn visible_reply(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    while let Some(start) = text.find("<think>") {
        if let Some(end) = text[start..].find("</think>") {
            let close = start + end + "</think>".len();
            text.replace_range(start..close, "");
        } else {
            text.truncate(start);
            break;
        }
    }
    chat_act::cap_say(&text)
}

fn tag_values(event: &Event, name: &str) -> Vec<String> {
    event
        .tags
        .iter()
        .filter_map(|tag| {
            let row = tag.as_slice();
            if row.first().map(String::as_str) == Some(name) {
                row.get(1).cloned()
            } else {
                None
            }
        })
        .collect()
}

fn kind_of(event: &Event) -> u32 {
    u32::from(event.kind.as_u16())
}

struct Line {
    id: String,
    author: String,
    content: String,
    at: u64,
    /// The message `p`-tags the org agent.
    mentions_me: bool,
    /// NIP-10 thread root when this line is a reply. Absent on a channel message.
    thread_root: Option<String>,
    /// Our own message that claimed an act without the tags that make it real.
    /// It does not count as an answer, so the next turn can still sign the act.
    hollow: bool,
    /// This message already carried a draft or act tag.
    acted: bool,
    /// The direction slug this message drafted, from its `direction` tag.
    direction: Option<String>,
    /// The text of that draft. The model only sees `content`, so this is
    /// how it knows which lines its earlier draft already held.
    draft_body: Option<String>,
}

/// A draft shown in one place — the channel, or one thread — until they agree.
struct HeldAct {
    act: ResolvedAct,
    thread_root: Option<String>,
}

struct Room {
    messages: Vec<Line>,
    /// A kind 40003 that arrived before the kind 9 it edits. The visible
    /// sentence is what the interview reads.
    pending_edits: HashMap<String, (u64, String)>,
    /// Newest edit applied to each message, so an older replay does not
    /// put the previous sentence back.
    edit_at: HashMap<String, u64>,
    welcomed: bool,
    /// Shown in chat, not signed, until the member says yes.
    pending: Option<HeldAct>,
    /// `created_at` of the newest human line we already decided not to answer.
    /// Stops a quiet `#shapers` line from calling the model again on every
    /// org refresh.
    quiet_until: u64,
    /// A hollow "opening the ticket" was already turned into a real act.
    ticket_recovered: bool,
    /// An untagged direction statement was already turned into a draft.
    direction_recovered: bool,
}

impl Room {
    fn push(
        &mut self,
        id: impl Into<String>,
        author: String,
        content: String,
        created_at: u64,
        mentions_me: bool,
        thread_root: Option<String>,
    ) {
        self.push_inner(
            id,
            author,
            content,
            created_at,
            mentions_me,
            thread_root,
            false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_acted(
        &mut self,
        id: impl Into<String>,
        author: String,
        content: String,
        created_at: u64,
        mentions_me: bool,
        thread_root: Option<String>,
        acted: bool,
    ) {
        self.push_inner(
            id,
            author,
            content,
            created_at,
            mentions_me,
            thread_root,
            acted,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_inner(
        &mut self,
        id: impl Into<String>,
        author: String,
        content: String,
        created_at: u64,
        mentions_me: bool,
        thread_root: Option<String>,
        acted: bool,
    ) {
        let id = id.into();
        if !id.is_empty() && self.messages.iter().any(|line| line.id == id) {
            return;
        }
        let content = if let Some((at, edited)) = self.pending_edits.remove(&id) {
            self.edit_at.insert(id.clone(), at);
            edited
        } else {
            content
        };
        let hollow = !acted && hollow_claim(&content);
        self.messages.push(Line {
            id,
            author,
            content,
            at: created_at,
            mentions_me,
            thread_root,
            hollow,
            acted,
            direction: None,
            draft_body: None,
        });
        self.messages.sort_by_key(|line| line.at);
        if self.messages.len() > MAX_HISTORY {
            let drop_n = self.messages.len() - MAX_HISTORY;
            self.messages.drain(0..drop_n);
        }
    }

    /// Replace the sentence a member sees. A later edit wins.
    fn apply_edit(&mut self, target: &str, at: u64, content: String) {
        if self
            .edit_at
            .get(target)
            .is_some_and(|previous| *previous > at)
        {
            return;
        }
        if let Some(line) = self.messages.iter_mut().find(|line| line.id == target) {
            line.content = content;
            line.hollow = !line.acted && hollow_claim(&line.content);
            self.edit_at.insert(target.to_string(), at);
            return;
        }
        if self
            .pending_edits
            .get(target)
            .is_some_and(|(previous, _)| *previous > at)
        {
            return;
        }
        self.pending_edits.insert(target.to_string(), (at, content));
        self.edit_at.insert(target.to_string(), at);
    }

    fn note_direction(&mut self, id: &str, slug: &str, body: Option<&str>) {
        if let Some(line) = self.messages.iter_mut().find(|line| line.id == id) {
            line.direction = Some(slug.to_string());
            line.draft_body = body
                .map(str::trim)
                .filter(|body| !body.is_empty())
                .map(str::to_string);
        }
    }
}

/// Body of our newest draft of `slug`, in this branch when there is one there.
fn previous_draft<'a>(
    room: &'a Room,
    me: &str,
    slug: &str,
    thread_root: Option<&str>,
) -> Option<&'a str> {
    let drafts = || {
        room.messages.iter().rev().filter(|line| {
            line.author.eq_ignore_ascii_case(me) && line.direction.as_deref() == Some(slug)
        })
    };
    drafts()
        .find(|line| in_branch(line, thread_root) && line.draft_body.is_some())
        .or_else(|| drafts().find(|line| line.draft_body.is_some()))
        .and_then(|line| line.draft_body.as_deref())
}

/// Objectives and strategy grow a line at a time over a conversation. When
/// the model sends only the new line, the earlier draft's lines come first.
/// A body that repeats any earlier line is already the whole list, and a
/// member asking to remove or replace lines gets exactly the new body.
fn carry_draft_lines(slug: &str, previous: Option<&str>, body: &str, said: &str) -> String {
    let Some(previous) = previous else {
        return body.to_string();
    };
    if slug != "objectives" && slug != "strategy" || asks_to_replace_lines(said) {
        return body.to_string();
    }
    let split = |text: &str| -> Vec<String> {
        text.lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect()
    };
    let key = |line: &str| -> String {
        line.chars()
            .filter(|ch| ch.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    };
    let earlier = split(previous);
    let incoming = split(body);
    let earlier_keys: Vec<String> = earlier.iter().map(|line| key(line)).collect();
    if incoming
        .iter()
        .any(|line| earlier_keys.contains(&key(line)))
    {
        return body.to_string();
    }
    earlier
        .into_iter()
        .chain(incoming)
        .collect::<Vec<_>>()
        .join("\n")
}

fn asks_to_replace_lines(said: &str) -> bool {
    const WORDS: &[&str] = &[
        "remove", "drop", "delete", "only", "replace", "instead", "change", "rewrite", "reword",
        "scrap", "cut", "without", "restart",
    ];
    let lower = said.to_lowercase();
    lower.contains("start over")
        || lower
            .split(|ch: char| !ch.is_alphanumeric())
            .any(|word| WORDS.contains(&word))
}

/// Where to pick the walk up once `slug` is confirmed: the branch of our
/// newest direction draft, when that draft was this slug. `(thread_root, draft_id)`.
fn confirmed_draft_branch(room: &Room, me: &str, slug: &str) -> Option<(Option<String>, String)> {
    let draft = room
        .messages
        .iter()
        .rev()
        .find(|line| line.author.eq_ignore_ascii_case(me) && line.direction.is_some())?;
    if draft.direction.as_deref() != Some(slug) || draft.id.is_empty() {
        return None;
    }
    Some((draft.thread_root.clone(), draft.id.clone()))
}

/// Thread root of a kind 9, when the event is a reply. A channel message has none.
fn thread_root_of(event: &Event) -> Option<String> {
    parse_thread_markers(&event.tags)
        .resolve()
        .map(|(root, _parent)| root)
}

/// Where a reply to `line` belongs. `parent` is `line`'s own id.
struct ReplyTarget {
    root: String,
    parent: String,
}

fn reply_target(line: &Line) -> Option<ReplyTarget> {
    let root = line.thread_root.clone()?;
    if line.id.is_empty() {
        return None;
    }
    Some(ReplyTarget {
        root,
        parent: line.id.clone(),
    })
}

/// The channel itself, or one thread: its root message plus every reply in it.
fn in_branch(line: &Line, thread_root: Option<&str>) -> bool {
    match thread_root {
        Some(root) => line.id == root || line.thread_root.as_deref() == Some(root),
        None => line.thread_root.is_none(),
    }
}

fn pending_for<'a>(room: &'a Room, thread_root: Option<&str>) -> Option<&'a ResolvedAct> {
    let held = room.pending.as_ref()?;
    if held.thread_root.as_deref() == thread_root {
        Some(&held.act)
    } else {
        None
    }
}

fn fresh_room(welcomed: bool) -> Room {
    Room {
        messages: Vec::new(),
        pending_edits: HashMap::new(),
        edit_at: HashMap::new(),
        welcomed,
        pending: None,
        quiet_until: 0,
        ticket_recovered: false,
        direction_recovered: false,
    }
}

/// Stay connected and talk in agent DMs. Returns only if the key or URL is
/// unusable; relay drops retry inside the loop.
pub async fn serve(cfg: &Config) -> Result<(), String> {
    // Two processes both answer the same DM. The second one is the extra bubble.
    let _run_lock = acquire_run_lock(cfg.state_dir.as_path())?;
    let url = cfg
        .relay_url
        .clone()
        .ok_or_else(|| "BUZZ_RELAY_URL is required".to_string())?;
    let secret = cfg
        .private_key
        .clone()
        .ok_or_else(|| "BUZZ_PRIVATE_KEY is required".to_string())?;
    let keys = Keys::parse(&secret).map_err(|error| format!("BUZZ_PRIVATE_KEY: {error}"))?;
    let me = keys.public_key().to_hex();
    let mut backoff = Duration::from_secs(1);
    loop {
        match session(&url, &keys, &me, cfg).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                eprintln!("org agent: {error}; retrying");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(60));
            }
        }
    }
}

fn talk_sub_id(channel: &str) -> String {
    format!("talk:{channel}")
}

fn channel_of_talk_sub(sub_id: &str) -> Option<&str> {
    sub_id.strip_prefix("talk:")
}

fn latest_human<'a>(room: &'a Room, me: &str) -> Option<&'a Line> {
    room.messages
        .iter()
        .rev()
        .find(|line| !line.author.eq_ignore_ascii_case(me))
}

/// A user message still needs an answer when nothing after it, in the same
/// place, is a real reply. A channel reply does not answer a thread, and a
/// thread reply does not answer the channel. The canned welcome lines do not
/// count. A line we already chose to leave quiet does not count either.
fn needs_reply(room: &Room, me: &str) -> bool {
    let Some(human) = latest_human(room, me) else {
        return false;
    };
    if human.at <= room.quiet_until {
        return false;
    }
    let user_at = human.at;
    let thread = human.thread_root.as_deref();
    !room.messages.iter().any(|line| {
        line.author.eq_ignore_ascii_case(me)
            && line.at >= user_at
            && line.thread_root.as_deref() == thread
            && !line.hollow
            && !is_welcome_line(&line.content)
    })
}

fn claims_opened_ticket(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    lower.contains("opening the ticket")
        || lower.contains("opening this ticket")
        || lower.contains("opening that ticket")
        || lower.contains("opening a ticket")
}

/// A sentence that says an act happened, without the tags that make it so.
fn hollow_claim(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    claims_opened_ticket(content)
        || lower.contains("opening the project")
        || lower.contains("opening a proposal")
        || lower.contains("dri proposal")
        || lower.contains("drafting a dri")
        || lower.starts_with("offering ")
        || lower.starts_with("publishing")
        || lower.starts_with("marking that done")
        || lower.starts_with("removing ")
}

fn event_has_act(event: &Event) -> bool {
    const NAMES: &[&str] = &[
        "ticket",
        "project",
        "done",
        "dri",
        "remove",
        "revise",
        "direction",
    ];
    event.tags.iter().any(|tag| {
        tag.as_slice()
            .first()
            .is_some_and(|name| NAMES.contains(&name.as_str()))
    })
}

fn channel_archived(event: &Event) -> bool {
    event.tags.iter().any(|tag| {
        let row = tag.as_slice();
        row.first().map(String::as_str) == Some("archived")
            && row.get(1).map(String::as_str) != Some("false")
    })
}

fn mark_quiet(room: &mut Room, me: &str) {
    if let Some(at) = latest_human(room, me).map(|line| line.at) {
        if at > room.quiet_until {
            room.quiet_until = at;
        }
    }
}

/// Leave this line unanswered. A publish announcement always qualifies,
/// including in the DM and when it follows a question.
fn declines_reply(
    content: &str,
    answers_every_line: bool,
    mentions_agent: bool,
    follows: bool,
) -> bool {
    if chat_act::is_proposal_announcement(content) {
        return true;
    }
    !should_answer_line(answers_every_line, content, mentions_agent) && !follows
}

/// The member's DM with the org agent answers every line. Every other room
/// answers when the line mentions the agent or is about the work it drafts.
fn should_answer_line(answers_every_line: bool, content: &str, mentions_agent: bool) -> bool {
    if chat_act::is_proposal_announcement(content) {
        return false;
    }
    if answers_every_line || mentions_agent {
        return true;
    }
    line_concerns_agent(content)
}

/// A line that answers the agent's question, or a draft waiting on yes.
fn follows_agent(room: &Room, me: &str, line: &Line) -> bool {
    let thread = line.thread_root.as_deref();
    if room
        .pending
        .as_ref()
        .is_some_and(|held| held.thread_root.as_deref() == thread)
    {
        return true;
    }
    let Some(previous) = room
        .messages
        .iter()
        .rev()
        .find(|earlier| earlier.at < line.at && in_branch(earlier, thread) && !earlier.hollow)
    else {
        return false;
    };
    previous.author.eq_ignore_ascii_case(me) && previous.content.contains('?')
}

fn line_concerns_agent(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    if lower.contains("org agent") || lower.contains("org. agent") {
        return true;
    }
    const PHRASES: &[&str] = &[
        "can you",
        "could you",
        "would you",
        "will you",
        "please",
        "let's",
        "lets ",
        "we should",
        "we need",
        "i want",
        "i need",
        "mark it done",
        "mark done",
        "it's done",
        "its done",
        "open a",
        "set the",
        "set our",
    ];
    if PHRASES.iter().any(|phrase| lower.contains(phrase)) {
        return true;
    }
    const WORDS: &[&str] = &[
        "mission",
        "vision",
        "objective",
        "objectives",
        "strategy",
        "project",
        "ticket",
        "dri",
        "holder",
        "proposal",
        "direction",
        "publish",
        "draft",
        "assign",
        "bootstrap",
        "shaper",
        "shapers",
    ];
    let tokens = tokenize(&lower);
    WORDS
        .iter()
        .any(|word| tokens.iter().any(|token| token == word))
}

fn tokenize(lower: &str) -> Vec<String> {
    lower
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn conversation_place(board: &Board, channel: &str, answers_every_line: bool) -> String {
    if answers_every_line {
        return "a private DM with one member. Answer them".to_string();
    }
    let room = if board.shapers_room() == Some(channel) {
        if board.several_shapers() {
            "#shapers, where the Shapers talk together"
        } else {
            "#shapers"
        }
    } else {
        "a community channel"
    };
    format!(
        "\
{room}. You hear every message. \
Reply only when the latest message asks you something, mentions you, \
or is about direction, a project, a ticket, a holder, or a decision you draft. \
When they are talking to each other and nothing is asked of you, \
return an empty say and leave direction, body, and act null. \
Do not narrate their conversation"
    )
}

fn stored_history(room: &Room, thread_root: Option<&str>) -> Vec<(String, String, u64)> {
    room.messages
        .iter()
        .filter(|line| in_branch(line, thread_root))
        .map(|line| (line.author.clone(), line.content.clone(), line.at))
        .collect()
}

fn model_history(room: &Room, me: &str, thread_root: Option<&str>) -> Vec<(String, String, u64)> {
    room.messages
        .iter()
        .filter(|line| in_branch(line, thread_root) && !line.hollow)
        .map(|line| {
            let mine = line.author.eq_ignore_ascii_case(me);
            let content = if line.mentions_me && !mine {
                format!("{}\n(You were mentioned.)", line.content)
            } else if let (true, Some(slug), Some(body)) =
                (mine, line.direction.as_deref(), line.draft_body.as_deref())
            {
                format!(
                    "{}\n(The {slug} draft card under this message read:\n{body})",
                    line.content
                )
            } else {
                line.content.clone()
            };
            (line.author.clone(), content, line.at)
        })
        .collect()
}

async fn session(url: &str, keys: &Keys, me: &str, cfg: &Config) -> Result<(), String> {
    let mut conn = NostrWsConnection::connect_authenticated(url, keys, None)
        .await
        .map_err(|error| format!("connect: {error}"))?;
    publish_profile(&mut conn, keys).await?;
    // A kinds-only REQ is registered as community-global. Channel events
    // (kind 9, kind 39000) are not delivered on it after the backlog, so a
    // DM opened later is invisible until we subscribe to that channel's `#h`.
    send_meta(&mut conn).await?;
    send_org(&mut conn).await?;
    conn.send_raw(&json!([
        "REQ",
        "opens",
        { "kinds": [41010], "#p": [me], "limit": 20 }
    ]))
    .await
    .map_err(|error| format!("subscribe opens: {error}"))?;

    let mut rooms: HashMap<String, Room> = HashMap::new();
    let mut agent_dms: HashSet<String> = HashSet::new();
    let mut watched: HashSet<String> = HashSet::new();
    let mut ready: HashSet<String> = HashSet::new();
    let mut asked_names: HashSet<String> = HashSet::new();
    let mut seen_heads: HashSet<String> = HashSet::new();
    let mut board = Board::default();
    let mut org = OrgState::new();
    let welcomed = load_welcomed(&cfg.state_dir);
    let started = std::time::Instant::now();
    let mut last_meta = std::time::Instant::now();
    let mut last_org = std::time::Instant::now();
    let mut saw_meta = false;
    let mut saw_org = false;
    let mut shapers_retry_at = std::time::Instant::now();

    loop {
        let message = match conn.next_event(Duration::from_secs(2)).await {
            Ok(message) => message,
            Err(WsClientError::Timeout) => {
                if !saw_meta && started.elapsed() > Duration::from_secs(45) {
                    return Err("timed out waiting for the relay backlog".into());
                }
                if saw_meta && last_meta.elapsed() >= Duration::from_secs(4) {
                    send_meta(&mut conn).await?;
                    last_meta = std::time::Instant::now();
                }
                if saw_meta && last_org.elapsed() >= Duration::from_secs(20) {
                    send_org(&mut conn).await?;
                    last_org = std::time::Instant::now();
                }
                if std::time::Instant::now() >= shapers_retry_at {
                    watch_shapers(&mut conn, &board, &mut watched).await?;
                    let missing: Vec<String> = rooms
                        .keys()
                        .filter(|id| !watched.contains(*id))
                        .cloned()
                        .collect();
                    for channel in missing {
                        watch_channel(&mut conn, &channel, &mut watched).await?;
                    }
                }
                if !saw_org && started.elapsed() >= Duration::from_secs(4) {
                    saw_org = true;
                    reply_open(&mut conn, keys, me, &board, &mut rooms, &ready, &agent_dms).await?;
                }
                continue;
            }
            Err(error) => return Err(error.to_string()),
        };
        match message {
            RelayMessage::Event {
                event,
                subscription_id: _,
            } => {
                if kind_of(&event) == 41010 {
                    send_meta(&mut conn).await?;
                    last_meta = std::time::Instant::now();
                    continue;
                }
                let confirmed = newly_confirmed(&event, &mut seen_heads, saw_org);
                let transitions = remember_org(&mut org, &event);
                if org.live && due_tickets(&transitions) {
                    if let Err(error) = publish_due_tickets(&mut conn, keys, &mut org, &board).await
                    {
                        eprintln!("org agent: tickets: {error}");
                    }
                }
                if observe_board(&event, &mut board) {
                    if kind_of(&event) == 39103 {
                        watch_shapers(&mut conn, &board, &mut watched).await?;
                    }
                    ask_missing_names(&mut conn, &board, &mut asked_names).await?;
                    if let Some(confirmed) = confirmed {
                        let listening: Vec<String> = rooms
                            .keys()
                            .filter(|channel| {
                                ready.contains(*channel)
                                    && (agent_dms.contains(*channel)
                                        || board.shapers_room() == Some(channel.as_str()))
                            })
                            .cloned()
                            .collect();
                        match confirmed {
                            ConfirmedAdvance::Direction(confirmed) => {
                                continue_after_confirm(
                                    &mut conn, keys, me, &board, &mut rooms, &listening, &confirmed,
                                )
                                .await?;
                            }
                            ConfirmedAdvance::Profile => {
                                continue_after_profile(
                                    &mut conn, keys, me, &board, &mut rooms, &listening,
                                )
                                .await?;
                            }
                            ConfirmedAdvance::Codebases => {
                                continue_after_codebases(
                                    &mut conn, keys, me, &board, &mut rooms, &listening,
                                )
                                .await?;
                            }
                        }
                    }
                    continue;
                }
                let channel = note_event(&event, me, &mut rooms, &mut agent_dms, &welcomed);
                if let Some(channel) = channel {
                    if !watched.contains(&channel) {
                        watch_channel(&mut conn, &channel, &mut watched).await?;
                    }
                    if saw_org
                        && kind_of(&event) == 9
                        && ready.contains(&channel)
                        && !event.pubkey.to_hex().eq_ignore_ascii_case(me)
                    {
                        let answers_every_line = agent_dms.contains(&channel);
                        reply_one(
                            &mut conn,
                            keys,
                            me,
                            &board,
                            &mut rooms,
                            &channel,
                            answers_every_line,
                        )
                        .await?;
                    }
                }
            }
            RelayMessage::Eose { subscription_id } => {
                if subscription_id == "meta" {
                    saw_meta = true;
                    let channels: Vec<String> = rooms.keys().cloned().collect();
                    for channel in channels {
                        watch_channel(&mut conn, &channel, &mut watched).await?;
                    }
                } else if let Some(channel) = channel_of_talk_sub(&subscription_id) {
                    let channel = channel.to_string();
                    if ready.insert(channel.clone()) {
                        if agent_dms.contains(&channel) {
                            greet_channel(&mut conn, keys, me, cfg, &mut rooms, &channel).await?;
                        }
                        if saw_org {
                            let answers_every_line = agent_dms.contains(&channel);
                            reply_one(
                                &mut conn,
                                keys,
                                me,
                                &board,
                                &mut rooms,
                                &channel,
                                answers_every_line,
                            )
                            .await?;
                        }
                    }
                } else if subscription_id == "org" {
                    saw_org = true;
                    org.mark_live();
                    if let Err(error) = publish_due_tickets(&mut conn, keys, &mut org, &board).await
                    {
                        eprintln!("org agent: tickets: {error}");
                    }
                    reply_open(&mut conn, keys, me, &board, &mut rooms, &ready, &agent_dms).await?;
                }
            }
            RelayMessage::Closed {
                subscription_id,
                message,
            } => {
                // The startup profile read closes its own REQ. That ack is not
                // a dropped room subscription.
                if subscription_id == "profile" {
                    continue;
                }
                if let Some(channel) = channel_of_talk_sub(&subscription_id) {
                    tracing::warn!(%message, channel, "org agent: channel subscription closed");
                    watched.remove(channel);
                    ready.remove(channel);
                    shapers_retry_at = std::time::Instant::now() + Duration::from_secs(30);
                    continue;
                }
                return Err(format!("relay closed the subscription: {message}"));
            }
            RelayMessage::Notice { message } => {
                tracing::info!(%message, "relay notice");
            }
            RelayMessage::Ok(ok) if !ok.accepted => {
                tracing::warn!(message = %ok.message, "relay rejected an event");
            }
            _ => {}
        }
    }
}

async fn reply_open(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    me: &str,
    board: &Board,
    rooms: &mut HashMap<String, Room>,
    ready: &HashSet<String>,
    agent_dms: &HashSet<String>,
) -> Result<(), String> {
    let channels: Vec<String> = rooms.keys().cloned().collect();
    for channel in channels {
        if ready.contains(&channel) {
            let answers_every_line = agent_dms.contains(&channel);
            reply_one(conn, keys, me, board, rooms, &channel, answers_every_line).await?;
        }
    }
    Ok(())
}

fn org_event(event: &Event) -> bool {
    matches!(
        kind_of(event),
        39100 | 39101 | 39102 | 39103 | 39105 | 39106 | 50005 | 50006 | 50100
    )
}

fn remember_org(org: &mut OrgState, event: &Event) -> Vec<Transition> {
    if !org_event(event) {
        return Vec::new();
    }
    match org.apply(event) {
        Ok(transitions) => transitions,
        Err(error) => {
            eprintln!("org agent: {error}");
            Vec::new()
        }
    }
}

fn due_tickets(transitions: &[Transition]) -> bool {
    transitions.iter().any(|transition| {
        matches!(
            transition,
            Transition::HolderSet { .. } | Transition::ItemDone { .. }
        )
    })
}

async fn publish_due_tickets(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    org: &mut OrgState,
    board: &Board,
) -> Result<(), String> {
    let digest = board.planner_digest();
    crate::plan::remember_digests_in(org, &digest);
    ensure_held_plans(org, &digest).await;
    let events = crate::plan::take_due_tickets(org, keys).map_err(|error| error.to_string())?;
    for event in events {
        let ok = conn
            .send_event(event)
            .await
            .map_err(|error| format!("ticket draft: {error}"))?;
        if !ok.accepted {
            eprintln!("org agent: ticket draft rejected: {}", ok.message);
        }
    }
    Ok(())
}

fn observe_board(event: &Event, board: &mut Board) -> bool {
    let kind = kind_of(event);
    if matches!(kind, 50005 | 50006) {
        let item = tag_values(event, "i").into_iter().next();
        board.note_command(
            kind,
            &event.id.to_hex(),
            &event.pubkey.to_hex(),
            item.as_deref(),
        );
        return true;
    }
    if !matches!(kind, 0 | 39100 | 39101 | 39102 | 39103 | 39105 | 39106) {
        return false;
    }
    let d = tag_values(event, "d").into_iter().next();
    board.observe(
        kind,
        &event.pubkey.to_hex(),
        event.created_at.as_secs(),
        d.as_deref(),
        &event.content,
    );
    true
}

/// Kind 0 for people the board already knows, when the broad names
/// backlog did not include their display name.
async fn ask_missing_names(
    conn: &mut NostrWsConnection,
    board: &Board,
    asked: &mut HashSet<String>,
) -> Result<(), String> {
    let missing: Vec<String> = board
        .unnamed_pubkeys()
        .into_iter()
        .filter(|pubkey| !asked.contains(pubkey))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let limit = missing.len();
    conn.send_raw(&json!([
        "REQ",
        "names-needed",
        { "kinds": [0], "authors": &missing, "limit": limit }
    ]))
    .await
    .map_err(|error| format!("subscribe names: {error}"))?;
    asked.extend(missing);
    Ok(())
}

async fn send_org(conn: &mut NostrWsConnection) -> Result<(), String> {
    conn.send_raw(&json!([
        "REQ",
        "org",
        { "kinds": [39100, 39101, 39102, 39103, 39105, 39106, 50005, 50006, 50100], "limit": 200 }
    ]))
    .await
    .map_err(|error| format!("subscribe org: {error}"))?;
    conn.send_raw(&json!([
        "REQ",
        "names",
        { "kinds": [0], "limit": 80 }
    ]))
    .await
    .map_err(|error| format!("subscribe names: {error}"))
}

async fn send_meta(conn: &mut NostrWsConnection) -> Result<(), String> {
    conn.send_raw(&json!([
        "REQ",
        "meta",
        { "kinds": [39000], "limit": 200 }
    ]))
    .await
    .map_err(|error| format!("subscribe metadata: {error}"))
}

async fn watch_shapers(
    conn: &mut NostrWsConnection,
    board: &Board,
    watched: &mut HashSet<String>,
) -> Result<(), String> {
    let Some(channel) = board.shapers_room() else {
        return Ok(());
    };
    if watched.contains(channel) {
        return Ok(());
    }
    let channel = channel.to_string();
    eprintln!("org agent: listening in #shapers ({channel})");
    watch_channel(conn, &channel, watched).await
}

async fn watch_channel(
    conn: &mut NostrWsConnection,
    channel: &str,
    watched: &mut HashSet<String>,
) -> Result<(), String> {
    if !watched.insert(channel.to_string()) {
        return Ok(());
    }
    conn.send_raw(&json!([
        "REQ",
        talk_sub_id(channel),
        { "kinds": [9, 40003], "#h": [channel], "limit": 80 }
    ]))
    .await
    .map_err(|error| format!("subscribe {channel}: {error}"))
}

fn note_event(
    event: &Event,
    me: &str,
    rooms: &mut HashMap<String, Room>,
    agent_dms: &mut HashSet<String>,
    welcomed: &HashSet<String>,
) -> Option<String> {
    match kind_of(event) {
        39000 => {
            if channel_archived(event) {
                return None;
            }
            let p_tags = tag_values(event, "p");
            let channel = tag_values(event, "d").into_iter().next()?;
            if is_agent_dm(me, &p_tags) {
                agent_dms.insert(channel.clone());
            }
            rooms
                .entry(channel.clone())
                .or_insert_with(|| fresh_room(welcomed.contains(&channel)));
            Some(channel)
        }
        9 => {
            let channel = tag_values(event, "h").into_iter().next()?;
            let room = rooms
                .entry(channel.clone())
                .or_insert_with(|| fresh_room(welcomed.contains(&channel)));
            let author = event.pubkey.to_hex();
            if author.eq_ignore_ascii_case(me) {
                room.welcomed = true;
            }
            let mentions_me = tag_values(event, "p")
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case(me));
            let id = event.id.to_hex();
            room.push_acted(
                id.clone(),
                author,
                event.content.clone(),
                event.created_at.as_secs(),
                mentions_me,
                thread_root_of(event),
                event_has_act(event),
            );
            if let Some(row) = event
                .tags
                .iter()
                .map(|tag| tag.as_slice())
                .find(|row| row.first().map(String::as_str) == Some("direction"))
            {
                if let Some(slug) = row.get(1) {
                    room.note_direction(&id, slug, row.get(2).map(String::as_str));
                }
            }
            Some(channel)
        }
        40003 => {
            let channel = tag_values(event, "h").into_iter().next()?;
            let target = tag_values(event, "e").into_iter().next()?;
            let room = rooms
                .entry(channel.clone())
                .or_insert_with(|| fresh_room(welcomed.contains(&channel)));
            room.apply_edit(&target, event.created_at.as_secs(), event.content.clone());
            Some(channel)
        }
        _ => None,
    }
}

async fn greet_channel(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    me: &str,
    cfg: &Config,
    rooms: &mut HashMap<String, Room>,
    channel: &str,
) -> Result<(), String> {
    let should = rooms
        .get(channel)
        .map(|room| owes_welcome(room, me))
        .unwrap_or(true);
    if !should {
        return Ok(());
    }
    for line in WELCOME_LINES {
        let id = publish_chat(
            conn,
            keys,
            channel,
            OutgoingChat {
                content: line,
                direction: None,
                body: None,
                from: None,
                extra: &[],
                thread: None,
                reply_to: None,
            },
        )
        .await?;
        if let Some(room) = rooms.get_mut(channel) {
            room.push(
                id,
                (*me).to_string(),
                (*line).to_string(),
                now_secs(),
                false,
                None,
            );
            room.welcomed = true;
        }
    }
    remember_welcomed(cfg.state_dir.as_path(), channel)
}

/// They said yes to a described ticket, and the reply that followed claimed
/// to open it without tags. A later real reply that names the title closes it.
fn unfinished_ticket(room: &Room, me: &str) -> Option<(String, chat_act::RawAct)> {
    let mut offer: Option<chat_act::RawAct> = None;
    let mut speaker: Option<String> = None;
    let mut agreed = false;
    let mut self_claim = false;
    let mut found: Option<(String, chat_act::RawAct)> = None;
    for line in &room.messages {
        if line.author.eq_ignore_ascii_case(me) {
            if line.hollow && claims_opened_ticket(&line.content) && agreed {
                if let (Some(act), Some(who)) = (offer.clone(), speaker.clone()) {
                    found = Some((who, claim_ticket_self(act, self_claim)));
                }
            }
            if let Some(act) = chat_act::ticket_from_say(&line.content) {
                offer = Some(act);
                agreed = false;
                self_claim = false;
            }
            if !line.hollow {
                if let Some((_, chat_act::RawAct::Ticket { title, .. })) = &found {
                    if line
                        .content
                        .to_ascii_lowercase()
                        .contains(&title.to_ascii_lowercase())
                    {
                        found = None;
                    }
                }
            }
        } else if offer.is_some()
            && (chat_act::agrees_to_publish(&line.content)
                || chat_act::asks_to_open_ticket(&line.content))
        {
            speaker = Some(line.author.clone());
            agreed = true;
            self_claim = chat_act::names_self_as_holder(&line.content);
        }
    }
    found
}

fn claim_ticket_self(act: chat_act::RawAct, claim: bool) -> chat_act::RawAct {
    if !claim {
        return act;
    }
    match act {
        chat_act::RawAct::Ticket {
            parent,
            title,
            brief,
            due_days,
            who,
        } if who.trim().is_empty() => chat_act::RawAct::Ticket {
            parent,
            title,
            brief,
            due_days,
            who: "me".into(),
        },
        other => other,
    }
}

fn agreed_ticket(room: &Room, me: &str, thread_root: Option<&str>) -> Option<chat_act::RawAct> {
    let previous = room.messages.iter().rev().find(|line| {
        in_branch(line, thread_root) && line.author.eq_ignore_ascii_case(me) && !line.hollow
    })?;
    chat_act::ticket_from_say(&previous.content)
}

async fn recover_unfinished_ticket(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    me: &str,
    board: &Board,
    rooms: &mut HashMap<String, Room>,
    channel: &str,
) -> Result<bool, String> {
    let (speaker, act) = {
        let Some(room) = rooms.get(channel) else {
            return Ok(false);
        };
        if room.ticket_recovered {
            return Ok(false);
        }
        let Some(found) = unfinished_ticket(room, me) else {
            return Ok(false);
        };
        found
    };
    let Some(resolved) = chat_act::resolve_act(&act, board, &speaker, now_secs()) else {
        if let Some(room) = rooms.get_mut(channel) {
            room.ticket_recovered = true;
        }
        return Ok(false);
    };
    let say = chat_act::annotate_act_say(&chat_act::published_say(&resolved), &resolved, board);
    let tags = chat_act::act_tags(&resolved, &speaker);
    let id = publish_chat(
        conn,
        keys,
        channel,
        OutgoingChat {
            content: &say,
            direction: None,
            body: None,
            from: Some(speaker.as_str()),
            extra: &tags,
            thread: None,
            reply_to: None,
        },
    )
    .await?;
    if let Some(room) = rooms.get_mut(channel) {
        room.ticket_recovered = true;
        room.pending = None;
        room.push_acted(id, me.to_string(), say, now_secs(), false, None, true);
    }
    Ok(true)
}

fn chat_turns<'a>(room: &'a Room, me: &str) -> Vec<chat_act::ChatTurn<'a>> {
    room.messages
        .iter()
        .map(|line| chat_act::ChatTurn {
            from_agent: line.author.eq_ignore_ascii_case(me),
            content: line.content.as_str(),
            acted: line.acted,
        })
        .collect()
}

/// The member already stated a direction and the agent never tagged a draft.
/// Post that draft once, without calling the model again.
async fn recover_missing_direction(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    me: &str,
    board: &Board,
    rooms: &mut HashMap<String, Room>,
    channel: &str,
) -> Result<bool, String> {
    let found = {
        let Some(room) = rooms.get(channel) else {
            return Ok(false);
        };
        if room.direction_recovered || needs_reply(room, me) {
            return Ok(false);
        }
        let turns = chat_turns(room, me);
        chat_act::missing_direction_draft(&turns)
    };
    if let Some(room) = rooms.get_mut(channel) {
        room.direction_recovered = true;
    }
    let Some((slug, body)) = found else {
        return Ok(false);
    };
    let shaped =
        chat_act::apply_interview(board, "", String::new(), Some(slug), Some(body.clone()));
    let Some(slug) = shaped.direction else {
        if shaped.say.is_empty() {
            return Ok(false);
        }
        let speaker = rooms
            .get(channel)
            .and_then(|room| latest_human(room, me).map(|line| line.author.clone()));
        let id = publish_chat(
            conn,
            keys,
            channel,
            OutgoingChat {
                content: &shaped.say,
                direction: None,
                body: None,
                from: speaker.as_deref(),
                extra: &[],
                thread: None,
                reply_to: None,
            },
        )
        .await?;
        if let Some(room) = rooms.get_mut(channel) {
            room.push_acted(
                id,
                me.to_string(),
                shaped.say,
                now_secs(),
                false,
                None,
                false,
            );
        }
        return Ok(true);
    };
    let body = shaped.body.unwrap_or(body);
    let speaker = rooms
        .get(channel)
        .and_then(|room| latest_human(room, me).map(|line| line.author.clone()));
    let say = chat_act::direction_draft_say(&slug);
    let id = publish_chat(
        conn,
        keys,
        channel,
        OutgoingChat {
            content: &say,
            direction: Some(slug.as_str()),
            body: Some(body.as_str()),
            from: speaker.as_deref(),
            extra: &[],
            thread: None,
            reply_to: None,
        },
    )
    .await?;
    if let Some(room) = rooms.get_mut(channel) {
        room.push_acted(
            id.clone(),
            me.to_string(),
            say,
            now_secs(),
            false,
            None,
            true,
        );
        room.note_direction(&id, &slug, Some(body.as_str()));
    }
    Ok(true)
}

async fn reply_one(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    me: &str,
    board: &Board,
    rooms: &mut HashMap<String, Room>,
    channel: &str,
    answers_every_line: bool,
) -> Result<(), String> {
    if recover_unfinished_ticket(conn, keys, me, board, rooms, channel).await? {
        return Ok(());
    }
    if recover_missing_direction(conn, keys, me, board, rooms, channel).await? {
        return Ok(());
    }
    let skip = {
        let Some(room) = rooms.get(channel) else {
            return Ok(());
        };
        if !needs_reply(room, me) {
            return Ok(());
        }
        latest_human(room, me).is_some_and(|line| {
            declines_reply(
                &line.content,
                answers_every_line,
                line.mentions_me,
                follows_agent(room, me, line),
            )
        })
    };
    if skip {
        if let Some(room) = rooms.get_mut(channel) {
            mark_quiet(room, me);
        }
        return Ok(());
    }
    let (history, model_lines, speaker, thread) = {
        let Some(room) = rooms.get(channel) else {
            return Ok(());
        };
        let human = latest_human(room, me);
        let speaker = human.map(|line| line.author.clone());
        let thread = human.and_then(reply_target);
        let thread_root = thread.as_ref().map(|target| target.root.as_str());
        (
            stored_history(room, thread_root),
            model_history(room, me, thread_root),
            speaker,
            thread,
        )
    };
    let mut place = conversation_place(board, channel, answers_every_line);
    if thread.is_some() {
        place.push_str(" The latest message is in a thread. Answer that thread.");
    }
    let mut reply = match complete_reply_retry(&model_lines, me, &place, &board.overview()).await {
        Ok(reply) => reply,
        Err(error) => {
            eprintln!("org agent: model call failed: {error}");
            return Ok(());
        }
    };
    // The latest human line is what they just said. An earlier yes must not
    // publish a new draft they have not seen.
    let latest_human = history.iter().rev().find_map(|(author, content, _)| {
        if author.eq_ignore_ascii_case(me) {
            None
        } else {
            Some(content.as_str())
        }
    });
    let just_agreed = latest_human.is_some_and(chat_act::agrees_to_publish);
    let ticket_agreed = latest_human.is_some_and(chat_act::asks_to_open_ticket);
    let model_resolved = reply.act.as_ref().and_then(|act| {
        let speaker = speaker.as_deref()?;
        chat_act::resolve_act(act, board, speaker, now_secs())
    });
    let thread_for_offer = thread.as_ref().map(|target| target.root.as_str());
    let described = if (just_agreed || ticket_agreed) && model_resolved.is_none() {
        rooms
            .get(channel)
            .and_then(|room| agreed_ticket(room, me, thread_for_offer))
    } else {
        None
    };
    let recovered = if model_resolved.is_none() {
        latest_human.and_then(|user| {
            let speaker_key = speaker.as_deref()?;
            if let Some(act) = chat_act::recover_project(user, &reply.say)
                .or_else(|| chat_act::recover_dri(user, &reply.say))
                .or_else(|| chat_act::recover_ticket(user, &reply.say))
            {
                return chat_act::resolve_act(&act, board, speaker_key, now_secs());
            }
            chat_act::project_draft_for_reply(
                reply.act.as_ref(),
                &reply.say,
                board,
                speaker_key,
                now_secs(),
            )
        })
    } else {
        None
    };
    let resolved = if model_resolved.is_some() {
        model_resolved
    } else if recovered.is_some() {
        recovered
    } else {
        described.as_ref().and_then(|act| {
            let speaker = speaker.as_deref()?;
            chat_act::resolve_act(act, board, speaker, now_secs())
        })
    };
    let removal_agreed = latest_human.is_some_and(chat_act::agrees_to_remove);
    let thread_root = thread.as_ref().map(|target| target.root.as_str());
    let pending = rooms
        .get(channel)
        .and_then(|room| pending_for(room, thread_root).cloned());
    let said = latest_human.unwrap_or("");
    let speaker_key = speaker.clone().unwrap_or_default();
    let resolved = resolved.map(|act| chat_act::offer_named_self(act, &speaker_key, said));
    let resolved = ensure_project_steps(resolved, &board.overview()).await;
    let pending = pending.map(|act| chat_act::offer_named_self(act, &speaker_key, said));
    let Turn { sign, draft, hold } = choose_turn(
        just_agreed,
        ticket_agreed,
        removal_agreed,
        resolved,
        pending,
    );
    if sign.is_some() || draft.is_some() || hold.is_some() {
        reply.direction = None;
        reply.body = None;
    } else if reply.direction.is_none() {
        if let Some(user) = latest_human {
            let turns = rooms
                .get(channel)
                .map(|room| chat_turns(room, me))
                .unwrap_or_default();
            if let Some((slug, body)) = chat_act::recover_direction(user, &reply.say)
                .or_else(|| chat_act::recover_confirmed_direction(&turns, user))
            {
                reply.say = chat_act::direction_draft_say(&slug);
                reply.direction = Some(slug);
                reply.body = Some(body);
            }
        }
    }
    if let Some(slug) = reply.direction.clone() {
        reply.say = chat_act::direction_reply_say(&reply.say, &slug);
        if let Some(body) = reply.body.as_deref() {
            let previous = rooms
                .get(channel)
                .and_then(|room| previous_draft(room, me, &slug, thread_root));
            let body = carry_draft_lines(&slug, previous, body, said);
            reply.say = chat_act::drop_restated_lines(&reply.say, &slug, &body);
            reply.body = Some(body);
        }
    }
    let mut profile_act = None;
    let mut codebase_tag: Option<String> = None;
    if reply.direction.is_some() || (sign.is_none() && draft.is_none() && hold.is_none()) {
        let shaped = chat_act::apply_interview(
            board,
            said,
            std::mem::take(&mut reply.say),
            reply.direction.take(),
            reply.body.take(),
        );
        reply.say = shaped.say;
        reply.direction = shaped.direction;
        reply.body = shaped.body;
        if let Some(items) = shaped.codebases {
            if let Ok(json) = serde_json::to_string(&items) {
                codebase_tag = Some(json);
            }
        }
        if sign.is_none() && draft.is_none() && hold.is_none() {
            if let Some(raw) = shaped.profile {
                if let Some(who) = speaker.as_deref() {
                    profile_act = chat_act::resolve_act(&raw, board, who, now_secs());
                }
            }
        }
    }
    reply.say = reply_sentence(&reply.say, draft.as_ref().or(hold.as_ref()), sign.as_ref());
    let invite_setup = reply
        .setup
        .as_ref()
        .is_some_and(|tag| tag.get(1).map(String::as_str) == Some("invite"));
    if invite_setup {
        reply.say = chat_act::invite_transition_say(&reply.say);
    }
    if let Some(act) = draft.as_ref().or(hold.as_ref()).or(sign.as_ref()) {
        reply.say = chat_act::annotate_act_say(&reply.say, act, board);
    }
    let saved_profile = matches!(
        sign.as_ref().or(profile_act.as_ref()),
        Some(chat_act::ResolvedAct::Profile { .. })
    );
    if saved_profile {
        if let Some(question) = board.question_after_people() {
            reply.say = chat_act::acceptance_transition("profile", "", Some(question));
        }
    } else if let Some(question) = chat_act::next_interview_question(board) {
        if chat_act::pitches_project(&reply.say) {
            reply.say = chat_act::acceptance_transition("strategy", "", Some(question));
        }
    }
    let act_tags = sign
        .as_ref()
        .or(draft.as_ref())
        .or(profile_act.as_ref())
        .and_then(|act| speaker.as_deref().map(|who| chat_act::act_tags(act, who)));
    if reply.direction.is_none() && act_tags.is_none() && chat_act::claims_project_draft(&reply.say)
    {
        reply.say = chat_act::without_false_project_draft(&reply.say);
    }
    if act_tags.is_none() && hollow_claim(&reply.say) {
        reply.say = if reply.say.to_ascii_lowercase().contains("dri") {
            "I didn't open that draft. Name the project and who should hold it.".to_string()
        } else {
            "I didn't open that. Name the project you hold and who the ticket is for.".to_string()
        };
    }
    if reply.say.is_empty() {
        if reply.direction.is_none() && act_tags.is_none() && reply.setup.is_none() {
            if let Some(room) = rooms.get_mut(channel) {
                mark_quiet(room, me);
            }
            return Ok(());
        }
        reply.say = "Got it.".to_string();
    }
    let mut extra = act_tags.clone().unwrap_or_default();
    if let Some(json) = codebase_tag.clone() {
        if !extra
            .iter()
            .any(|row| row.first().map(String::as_str) == Some("from"))
        {
            if let Some(who) = speaker.as_deref() {
                extra.push(vec!["from".to_string(), who.to_string()]);
            }
        }
        extra.push(vec!["codebases".to_string(), json]);
    }
    if let Some(row) = reply.setup.clone() {
        extra.push(row);
    }
    let id = publish_chat(
        conn,
        keys,
        channel,
        OutgoingChat {
            content: &reply.say,
            direction: reply.direction.as_deref(),
            body: reply.body.as_deref(),
            from: speaker.as_deref(),
            extra: &extra,
            thread: thread
                .as_ref()
                .map(|target| (target.root.as_str(), target.parent.as_str())),
            reply_to: thread.as_ref().and(speaker.as_deref()),
        },
    )
    .await?;
    if let Some(room) = rooms.get_mut(channel) {
        if sign.is_some() || draft.is_some() {
            room.pending = None;
        } else if let Some(act) = hold {
            room.pending = Some(HeldAct {
                act,
                thread_root: thread.as_ref().map(|target| target.root.clone()),
            });
        }
        let acted = act_tags.is_some() || reply.direction.is_some() || codebase_tag.is_some();
        room.push_acted(
            id.clone(),
            me.to_string(),
            reply.say,
            now_secs(),
            false,
            thread.as_ref().map(|target| target.root.clone()),
            acted,
        );
        if let Some(slug) = reply.direction.as_deref() {
            room.note_direction(&id, slug, reply.body.as_deref());
        }
    }
    if invite_setup {
        let follow = chat_act::MISSION_HANDOFF;
        // One second later. Equal timestamps sort by event id, which can put
        // this question above the invite explanation.
        let follow_at = now_secs().saturating_add(1);
        let follow_id = publish_chat_at(
            conn,
            keys,
            channel,
            OutgoingChat {
                content: follow,
                direction: None,
                body: None,
                from: None,
                extra: &[],
                thread: thread
                    .as_ref()
                    .map(|target| (target.root.as_str(), target.parent.as_str())),
                reply_to: thread.as_ref().and(speaker.as_deref()),
            },
            Some(follow_at),
        )
        .await?;
        if let Some(room) = rooms.get_mut(channel) {
            room.push_acted(
                follow_id,
                me.to_string(),
                follow.to_string(),
                follow_at,
                false,
                thread.as_ref().map(|target| target.root.clone()),
                false,
            );
        }
    }
    Ok(())
}

/// A direction we drafted in chat was just confirmed. Pick the walk up in
/// the room and branch where that draft sits, so nobody has to ask what is next.
async fn continue_after_confirm(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    me: &str,
    board: &Board,
    rooms: &mut HashMap<String, Room>,
    listening: &[String],
    confirmed: &ConfirmedDirection,
) -> Result<(), String> {
    for channel in listening {
        let found = rooms.get(channel).and_then(|room| {
            let (thread_root, draft_id) = confirmed_draft_branch(room, me, &confirmed.slug)?;
            let mut lines = model_history(room, me, thread_root.as_deref());
            lines.push((
                String::new(),
                chat_act::confirmed_note(&confirmed.slug, confirmed.version),
                now_secs(),
            ));
            let speaker = room
                .messages
                .iter()
                .rev()
                .find(|line| {
                    !line.author.eq_ignore_ascii_case(me) && in_branch(line, thread_root.as_deref())
                })
                .map(|line| line.author.clone());
            Some((thread_root, draft_id, lines, speaker))
        });
        let Some((thread_root, draft_id, lines, speaker)) = found else {
            continue;
        };
        let answers_every_line = board.shapers_room().is_none_or(|room| room != channel);
        let mut place = conversation_place(board, channel, answers_every_line);
        if thread_root.is_some() {
            place.push_str(" This continues the thread the draft was in.");
        }
        let reply = match complete_reply_retry(&lines, me, &place, &board.overview()).await {
            Ok(reply) => reply,
            Err(error) => {
                eprintln!("org agent: model call failed: {error}");
                continue;
            }
        };
        if reply.say.is_empty() {
            continue;
        }
        let shaped = chat_act::apply_interview(
            board,
            "",
            reply.say.clone(),
            reply.direction.clone(),
            reply.body.clone(),
        );
        // A confirm with no new draft keeps the model's own words. The
        // interview would otherwise replace a human lead-in with the bare
        // question for the next row.
        let drafted = match (shaped.direction.as_deref(), shaped.body.as_deref()) {
            (Some(slug), Some(body)) => chat_act::drop_restated_lines(
                &chat_act::direction_reply_say(&shaped.say, slug),
                slug,
                body,
            ),
            (Some(slug), None) => chat_act::direction_reply_say(&shaped.say, slug),
            (None, _) => reply.say.clone(),
        };
        let mut say = chat_act::acceptance_transition(
            &confirmed.slug,
            &drafted,
            chat_act::next_interview_question(board),
        );
        let project = if shaped.direction.is_none()
            && chat_act::next_interview_question(board).is_none()
        {
            speaker.as_deref().and_then(|who| {
                chat_act::project_draft_for_reply(reply.act.as_ref(), &say, board, who, now_secs())
            })
        } else {
            None
        };
        if project.is_none() && chat_act::claims_project_draft(&say) {
            say = chat_act::without_false_project_draft(&say);
        }
        let project_tags = project
            .as_ref()
            .and_then(|act| speaker.as_deref().map(|who| chat_act::act_tags(act, who)));
        let id = publish_chat(
            conn,
            keys,
            channel,
            OutgoingChat {
                content: &say,
                direction: shaped.direction.as_deref(),
                body: shaped.body.as_deref(),
                from: speaker.as_deref(),
                extra: project_tags.as_deref().unwrap_or(&[]),
                thread: thread_root.as_deref().map(|root| (root, draft_id.as_str())),
                reply_to: None,
            },
        )
        .await?;
        if let Some(room) = rooms.get_mut(channel) {
            room.push_acted(
                id.clone(),
                me.to_string(),
                say,
                now_secs(),
                false,
                thread_root,
                shaped.direction.is_some() || project_tags.is_some(),
            );
            if let Some(slug) = shaped.direction.as_deref() {
                room.note_direction(&id, slug, shaped.body.as_deref());
            }
        }
    }
    Ok(())
}

/// A `39100` head seen for the first time after the org backlog.
struct ConfirmedDirection {
    slug: String,
    version: u64,
}

/// A direction confirm continues the interview. A profile confirm asks the
/// repository question. A codebases confirm asks the landing page, or says
/// the org is ready, when that row was still open.
enum ConfirmedAdvance {
    Direction(ConfirmedDirection),
    Profile,
    Codebases,
}

/// Remembers every direction and profile head id. The backlog and each
/// periodic org re-read deliver the same heads again; only an id never seen
/// is a confirm.
fn newly_confirmed(
    event: &Event,
    seen: &mut HashSet<String>,
    after_backlog: bool,
) -> Option<ConfirmedAdvance> {
    let kind = kind_of(event);
    if !matches!(kind, 39100 | 39105 | 39106) || !seen.insert(event.id.to_hex()) || !after_backlog {
        return None;
    }
    if kind == 39105 {
        return Some(ConfirmedAdvance::Profile);
    }
    if kind == 39106 {
        let slug = tag_values(event, "d").into_iter().next()?;
        if slug != "codebases" {
            return None;
        }
        return Some(ConfirmedAdvance::Codebases);
    }
    let slug = tag_values(event, "d").into_iter().next()?;
    let version = tag_values(event, "version")
        .into_iter()
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    Some(ConfirmedAdvance::Direction(ConfirmedDirection {
        slug,
        version,
    }))
}

/// After a profile is published, ask the next open row. A later edit of the
/// same profile does not ask again.
async fn continue_after_profile(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    me: &str,
    board: &Board,
    rooms: &mut HashMap<String, Room>,
    listening: &[String],
) -> Result<(), String> {
    let Some(question) = chat_act::next_interview_question(board) else {
        return Ok(());
    };
    if !question.contains("code live") && !question.contains("landing page") {
        return Ok(());
    }
    for channel in listening {
        let branch = rooms.get(channel).and_then(|room| {
            if !room_asked_profile(room, me) || room_already_asked(room, me, question) {
                None
            } else {
                Some(profile_draft_branch(room, me))
            }
        });
        let Some(branch) = branch else {
            continue;
        };
        let say = chat_act::acceptance_transition("profile", "", Some(question));
        let id = publish_chat(
            conn,
            keys,
            channel,
            OutgoingChat {
                content: &say,
                direction: None,
                body: None,
                from: None,
                extra: &[],
                thread: branch
                    .as_ref()
                    .map(|(root, draft)| (root.as_str(), draft.as_str())),
                reply_to: None,
            },
        )
        .await?;
        if let Some(room) = rooms.get_mut(channel) {
            room.push_acted(
                id,
                me.to_string(),
                say,
                now_secs(),
                false,
                branch.map(|(root, _)| root),
                false,
            );
        }
    }
    Ok(())
}

/// After the codebases list is published, ask the landing page, or say the
/// org is ready, once. A later edit of the same list does not ask again.
async fn continue_after_codebases(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    me: &str,
    board: &Board,
    rooms: &mut HashMap<String, Room>,
    listening: &[String],
) -> Result<(), String> {
    let question = chat_act::next_interview_question(board);
    let say = match question {
        Some(question) if question.contains("landing page") => {
            chat_act::acceptance_transition("codebases", "", Some(question))
        }
        None => board.interview_line(),
        Some(_) => return Ok(()),
    };
    for channel in listening {
        let branch = rooms.get(channel).and_then(|room| {
            if !room_asked_code(room, me) || room_already_said(room, me, &say) {
                None
            } else {
                Some(())
            }
        });
        if branch.is_none() {
            continue;
        }
        let id = publish_chat(
            conn,
            keys,
            channel,
            OutgoingChat {
                content: &say,
                direction: None,
                body: None,
                from: None,
                extra: &[],
                thread: None,
                reply_to: None,
            },
        )
        .await?;
        if let Some(room) = rooms.get_mut(channel) {
            room.push_acted(
                id,
                me.to_string(),
                say.clone(),
                now_secs(),
                false,
                None,
                false,
            );
        }
    }
    Ok(())
}

fn room_asked_code(room: &Room, me: &str) -> bool {
    room.messages.iter().any(|line| {
        line.author.eq_ignore_ascii_case(me)
            && (line
                .content
                .to_ascii_lowercase()
                .contains("draft of the codebases")
                || line.content.to_ascii_lowercase().contains("where the code"))
    })
}

fn room_already_said(room: &Room, me: &str, say: &str) -> bool {
    let needle = if say.to_ascii_lowercase().contains("landing page") {
        "landing page"
    } else if say.to_ascii_lowercase().contains("org is ready") {
        "org is ready"
    } else {
        return false;
    };
    room.messages.iter().any(|line| {
        line.author.eq_ignore_ascii_case(me) && line.content.to_ascii_lowercase().contains(needle)
    })
}

fn room_asked_profile(room: &Room, me: &str) -> bool {
    room.messages.iter().any(|line| {
        line.author.eq_ignore_ascii_case(me)
            && (line
                .content
                .to_ascii_lowercase()
                .contains("draft of your profile")
                || line
                    .content
                    .to_ascii_lowercase()
                    .contains("who does the work")
                || line
                    .content
                    .to_ascii_lowercase()
                    .contains("skills you actually have"))
    })
}

fn room_already_asked(room: &Room, me: &str, question: &str) -> bool {
    let needle = if question.contains("landing page") {
        "landing page"
    } else if question.contains("code live") {
        "code live"
    } else {
        return false;
    };
    room.messages.iter().any(|line| {
        line.author.eq_ignore_ascii_case(me) && line.content.to_ascii_lowercase().contains(needle)
    })
}

fn profile_draft_branch(room: &Room, me: &str) -> Option<(String, String)> {
    let draft = room.messages.iter().rev().find(|line| {
        line.author.eq_ignore_ascii_case(me)
            && line
                .content
                .to_ascii_lowercase()
                .contains("draft of your profile")
            && !line.id.is_empty()
    })?;
    let root = draft.thread_root.clone()?;
    Some((root, draft.id.clone()))
}

async fn publish_profile(conn: &mut NostrWsConnection, keys: &Keys) -> Result<(), String> {
    let me = keys.public_key().to_hex();
    let existing = current_profile_content(conn, &me).await?;
    let content = profile_document(existing.as_deref()).to_string();
    let event = sign(keys, Permitted::Profile, &content, Vec::new()).map_err(publish_err)?;
    let ok = conn
        .send_event(event)
        .await
        .map_err(|error| format!("profile: {error}"))?;
    if !ok.accepted {
        return Err(format!("profile rejected: {}", ok.message));
    }
    Ok(())
}

/// Kind:0 the agent publishes. Name and about are fixed. An existing
/// `picture` is kept — a reconnect must not wipe the avatar.
fn profile_document(existing: Option<&str>) -> Value {
    let mut profile = serde_json::Map::new();
    profile.insert("name".into(), json!(ORG_AGENT_NAME));
    profile.insert("about".into(), json!("I draft. You decide."));
    if let Some(picture) = existing.and_then(kept_profile_picture) {
        profile.insert("picture".into(), Value::String(picture));
    }
    Value::Object(profile)
}

fn kept_profile_picture(content: &str) -> Option<String> {
    let value: Value = serde_json::from_str(content).ok()?;
    let picture = value.get("picture")?.as_str()?.trim();
    (!picture.is_empty()).then(|| picture.to_string())
}

/// Latest kind:0 content for this key, if the relay has one.
///
/// A failed read is an error. Publishing name and about without knowing
/// whether a picture is already set would replace the avatar with nothing.
async fn current_profile_content(
    conn: &mut NostrWsConnection,
    pubkey: &str,
) -> Result<Option<String>, String> {
    conn.send_raw(&json!([
        "REQ",
        "profile",
        { "kinds": [0], "authors": [pubkey], "limit": 1 }
    ]))
    .await
    .map_err(|error| format!("profile read: {error}"))?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut newest: Option<(u64, String)> = None;
    let content = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break Err("profile read timed out".to_string());
        }
        match conn.next_event(remaining).await {
            Ok(RelayMessage::Event {
                subscription_id,
                event,
            }) if subscription_id == "profile" => {
                let at = event.created_at.as_secs();
                let replace = match &newest {
                    None => true,
                    Some((prev, _)) => at >= *prev,
                };
                if replace {
                    newest = Some((at, event.content.clone()));
                }
            }
            Ok(RelayMessage::Eose { subscription_id }) if subscription_id == "profile" => {
                break Ok(newest.map(|(_, content)| content));
            }
            Ok(RelayMessage::Closed {
                subscription_id,
                message,
            }) if subscription_id == "profile" => {
                break Err(format!("profile read closed: {message}"));
            }
            Ok(_) => {}
            Err(WsClientError::Timeout) => break Err("profile read timed out".to_string()),
            Err(error) => break Err(format!("profile read: {error}")),
        }
    };
    if let Err(error) = content {
        let _ = conn.send_raw(&json!(["CLOSE", "profile"])).await;
        return Err(error);
    }
    // The relay answers CLOSE with CLOSED. Read that ack here so the session
    // loop does not treat it as a dropped subscription and reconnect.
    conn.send_raw(&json!(["CLOSE", "profile"]))
        .await
        .map_err(|error| format!("profile close: {error}"))?;
    let close_deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let remaining = close_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("profile close timed out".to_string());
        }
        match conn.next_event(remaining).await {
            Ok(RelayMessage::Closed {
                subscription_id, ..
            }) if subscription_id == "profile" => {
                break;
            }
            Ok(
                RelayMessage::Event {
                    subscription_id, ..
                }
                | RelayMessage::Eose { subscription_id },
            ) if subscription_id == "profile" => {}
            Err(WsClientError::Timeout) => return Err("profile close timed out".to_string()),
            Err(error) => return Err(format!("profile close: {error}")),
            Ok(_) => {}
        }
    }
    content
}

struct OutgoingChat<'a> {
    content: &'a str,
    direction: Option<&'a str>,
    body: Option<&'a str>,
    from: Option<&'a str>,
    extra: &'a [Vec<String>],
    /// `(root, parent)` when this posts into the thread the member just wrote in.
    thread: Option<(&'a str, &'a str)>,
    /// Author of that message, so the reply names them.
    reply_to: Option<&'a str>,
}

fn chat_tags(channel: &str, chat: &OutgoingChat<'_>) -> Result<Vec<Tag>, String> {
    let mut tags = vec![Tag::parse(["h", channel]).map_err(|error| format!("h tag: {error}"))?];
    if let Some((root, parent)) = chat.thread {
        if let Some(author) = chat.reply_to {
            tags.push(Tag::parse(["p", author]).map_err(|error| format!("p tag: {error}"))?);
        }
        for row in thread_tag_rows(root, parent) {
            tags.push(Tag::parse(row).map_err(|error| format!("thread tag: {error}"))?);
        }
    }
    if let Some(slug) = chat.direction {
        let mut row = vec!["direction".to_string(), slug.to_string()];
        if let Some(body) = chat.body {
            row.push(body.to_string());
        }
        tags.push(Tag::parse(row).map_err(|error| format!("direction tag: {error}"))?);
        if let Some(from) = chat.from {
            tags.push(Tag::parse(["from", from]).map_err(|error| format!("from tag: {error}"))?);
        }
    }
    for row in chat.extra {
        tags.push(Tag::parse(row.clone()).map_err(|error| format!("act tag: {error}"))?);
    }
    Ok(tags)
}

/// Same NIP-10 shape the desktop uses: a direct reply carries one `reply`
/// marker; a nested reply names `root` and `reply`.
fn thread_tag_rows(root: &str, parent: &str) -> Vec<Vec<String>> {
    if root == parent {
        vec![vec!["e".into(), root.into(), String::new(), "reply".into()]]
    } else {
        vec![
            vec!["e".into(), root.into(), String::new(), "root".into()],
            vec!["e".into(), parent.into(), String::new(), "reply".into()],
        ]
    }
}

async fn publish_chat(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    channel: &str,
    chat: OutgoingChat<'_>,
) -> Result<String, String> {
    publish_chat_at(conn, keys, channel, chat, None).await
}

async fn publish_chat_at(
    conn: &mut NostrWsConnection,
    keys: &Keys,
    channel: &str,
    chat: OutgoingChat<'_>,
    created_at: Option<u64>,
) -> Result<String, String> {
    let tags = chat_tags(channel, &chat)?;
    let event =
        sign_at(keys, Permitted::Chat, chat.content, tags, created_at).map_err(publish_err)?;
    let id = event.id.to_hex();
    let ok = conn
        .send_event(event)
        .await
        .map_err(|error| format!("chat: {error}"))?;
    if !ok.accepted {
        return Err(format!("chat rejected: {}", ok.message));
    }
    Ok(id)
}

fn publish_err(error: PublishError) -> String {
    error.to_string()
}

/// What the model said, and which direction artifact the member just stated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelReply {
    pub say: String,
    /// `mission`, `vision`, `situation`, `objectives`, or `strategy` when they just stated it.
    pub direction: Option<String>,
    /// The artifact sentence, never the member's "yes, set it".
    pub body: Option<String>,
    /// A project, ticket, done, or DRI the speaker asked to publish.
    pub act: Option<RawAct>,
    /// `io-setup` row for the desktop. The model's `say` is still the reply.
    pub setup: Option<Vec<String>>,
}

/// Read the model's JSON. Plain text stays a sentence with no direction.
pub fn parse_model_reply(raw: &str) -> ModelReply {
    let (say, direction, body, act) = chat_act::parse_model_reply(raw);
    ModelReply {
        say,
        direction,
        body,
        act,
        setup: chat_act::setup_tag(raw),
    }
}

/// A project draft with fewer than two real steps asks for those steps
/// before the card is published. A failed plan drops the draft.
async fn ensure_project_steps(act: Option<ResolvedAct>, overview: &str) -> Option<ResolvedAct> {
    let act = act?;
    let Some((title, brief)) = chat_act::project_to_plan(&act) else {
        return Some(act);
    };
    match think_project_steps(title, brief, overview).await {
        Ok(steps) => match serde_json::to_value(&steps) {
            Ok(value) => Some(chat_act::project_with_steps(act, value)),
            Err(error) => {
                eprintln!("org agent: no project draft without steps: {error}");
                None
            }
        },
        Err(error) => {
            eprintln!("org agent: no project draft without steps: {error}");
            None
        }
    }
}

async fn ensure_held_plans(org: &mut OrgState, digest: &str) {
    let mut missing = crate::plan::projects_without_steps(org);
    for row in crate::plan::projects_needing_how(org) {
        if !missing.iter().any(|(id, _, _)| id == &row.0) {
            missing.push(row);
        }
    }
    let direction = crate::plan::direction_excerpt(org);
    let context = if digest.trim().is_empty() {
        direction
    } else {
        format!("{direction}\n{digest}")
    };
    for (id, title, brief) in missing.into_iter().take(4) {
        match think_project_steps(&title, &brief, &context).await {
            Ok(steps) => {
                org.plans.insert(id, steps);
            }
            Err(error) => eprintln!("org agent: no steps for {title}: {error}"),
        }
    }
}

async fn think_project_steps(
    title: &str,
    brief: &str,
    context: &str,
) -> Result<Vec<buzz_core::intelligent_org::PlanStep>, String> {
    let prompt = crate::plan::project_steps_prompt(title, brief, context);
    let mut last = String::from("plan was not two to seven steps");
    for _ in 0..2 {
        let text = match steps_completion(&prompt).await {
            Ok(text) => text,
            Err(error) => {
                last = error;
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let value = match json_object(&text) {
            Ok(value) => value,
            Err(error) => {
                last = error;
                continue;
            }
        };
        if let Some(steps) =
            crate::plan::steps_ready_in(title, &value, &crate::plan::digest_paths_in(context))
        {
            return Ok(steps);
        }
        last = "plan was not two to seven steps".into();
    }
    Err(last)
}

async fn steps_completion(prompt: &str) -> Result<String, String> {
    let key = std::env::var("OPENAI_COMPAT_API_KEY")
        .or_else(|_| std::env::var("VENICE_API_KEY"))
        .map_err(|_| "OPENAI_COMPAT_API_KEY is not set".to_string())?;
    let base = std::env::var("OPENAI_COMPAT_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".into());
    let model = std::env::var("OPENAI_COMPAT_MODEL")
        .or_else(|_| std::env::var("IO_MODEL_FAST"))
        .or_else(|_| std::env::var("IO_MODEL_DRAFT"))
        .map_err(|_| "OPENAI_COMPAT_MODEL is not set".to_string())?;
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("http: {error}"))?;
    let response = client
        .post(url)
        .bearer_auth(key)
        .json(&json!({
            "model": model,
            "messages": [{ "role": "user", "content": prompt }],
            "temperature": 0.2
        }))
        .send()
        .await
        .map_err(|error| format!("model request: {error}"))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|error| format!("model body: {error}"))?;
    if !status.is_success() {
        let detail = body
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("model request failed");
        return Err(format!("{status}: {detail}"));
    }
    body.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| "model returned no steps".to_string())
}

fn json_object(text: &str) -> Result<Value, String> {
    let trimmed = text.trim();
    let start = trimmed
        .find('{')
        .or_else(|| trimmed.find('['))
        .ok_or_else(|| "model returned no JSON".to_string())?;
    let end = trimmed
        .rfind('}')
        .or_else(|| trimmed.rfind(']'))
        .ok_or_else(|| "model returned no JSON".to_string())?;
    if end < start {
        return Err("model returned no JSON".into());
    }
    serde_json::from_str(&trimmed[start..=end]).map_err(|error| format!("steps json: {error}"))
}

async fn complete_reply_retry(
    history: &[(String, String, u64)],
    me: &str,
    place: &str,
    overview: &str,
) -> Result<ModelReply, String> {
    let mut delay = Duration::from_secs(2);
    let mut last = String::new();
    for attempt in 0..4 {
        match complete_reply(history, me, place, overview).await {
            Ok(reply) => return Ok(reply),
            Err(error) if error.contains("429") && attempt < 3 => {
                last = error;
                eprintln!("org agent: model busy, retrying");
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(20));
            }
            Err(error) => return Err(error),
        }
    }
    Err(last)
}

async fn complete_reply(
    history: &[(String, String, u64)],
    me: &str,
    place: &str,
    overview: &str,
) -> Result<ModelReply, String> {
    let key = std::env::var("OPENAI_COMPAT_API_KEY")
        .or_else(|_| std::env::var("VENICE_API_KEY"))
        .map_err(|_| "OPENAI_COMPAT_API_KEY is not set".to_string())?;
    let base = std::env::var("OPENAI_COMPAT_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".into());
    let model = std::env::var("OPENAI_COMPAT_MODEL")
        .or_else(|_| std::env::var("IO_MODEL_FAST"))
        .or_else(|_| std::env::var("IO_MODEL_DRAFT"))
        .map_err(|_| "OPENAI_COMPAT_MODEL is not set".to_string())?;
    let mut messages = vec![json!({
        "role": "system",
        "content": chat_act::system_prompt(place, overview)
    })];
    for (author, content, _) in history {
        let role = if author.eq_ignore_ascii_case(me) {
            "assistant"
        } else {
            "user"
        };
        messages.push(json!({ "role": role, "content": content }));
    }
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| format!("http: {error}"))?;
    let response = client
        .post(url)
        .bearer_auth(key)
        .json(&json!({
            "model": model,
            "messages": messages,
            "temperature": 0.4
        }))
        .send()
        .await
        .map_err(|error| format!("model request: {error}"))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|error| format!("model body: {error}"))?;
    if !status.is_success() {
        let detail = body
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("model request failed");
        return Err(format!("{status}: {detail}"));
    }
    let text = body
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok(parse_model_reply(&text))
}

struct Turn {
    /// The member's client signs this. Tickets, done, and a ticket removal.
    sign: Option<ResolvedAct>,
    /// Tags on the message. A draft card; they publish it themselves.
    draft: Option<ResolvedAct>,
    /// No tags yet. A ticket they have not agreed to offer.
    hold: Option<ResolvedAct>,
}

fn is_proposal(act: &ResolvedAct) -> bool {
    matches!(
        act,
        ResolvedAct::Project { .. }
            | ResolvedAct::Dri { .. }
            | ResolvedAct::ReviseDirection { .. }
            | ResolvedAct::ReviseProject { .. }
            | ResolvedAct::RemoveProject { .. }
            | ResolvedAct::Shapers { .. }
            | ResolvedAct::Rules { .. }
            | ResolvedAct::Agent { .. }
    )
}

/// One user line produces one bubble. A proposal is a draft card on this
/// message, whether one Shaper is seated or many. Saying yes does not sign
/// it. A ticket stays held until they agree. Done and a ticket removal sign.
fn is_ticket(act: &ResolvedAct) -> bool {
    matches!(act, ResolvedAct::Ticket { .. })
}

fn choose_turn(
    just_agreed: bool,
    ticket_agreed: bool,
    removal_agreed: bool,
    resolved: Option<ResolvedAct>,
    pending: Option<ResolvedAct>,
) -> Turn {
    let direct_removal = if removal_agreed {
        resolved
            .clone()
            .filter(|act| matches!(act, ResolvedAct::Remove { .. }))
            .or_else(|| {
                pending
                    .clone()
                    .filter(|act| matches!(act, ResolvedAct::Remove { .. }))
            })
    } else {
        None
    };
    let agreed_ticket = if ticket_agreed {
        resolved
            .clone()
            .filter(is_ticket)
            .or_else(|| pending.clone().filter(is_ticket))
    } else {
        None
    };
    let sign = if direct_removal.is_some() {
        direct_removal
    } else if agreed_ticket.is_some() {
        agreed_ticket
    } else if just_agreed {
        resolved
            .clone()
            .filter(|act| !is_proposal(act))
            .or_else(|| pending.clone().filter(|act| !is_proposal(act)))
    } else if resolved
        .as_ref()
        .is_some_and(|act| matches!(act, ResolvedAct::Done { .. } | ResolvedAct::Profile { .. }))
    {
        resolved.clone()
    } else {
        None
    };
    let draft = if sign.is_some() {
        None
    } else {
        resolved.clone().filter(is_proposal)
    };
    let hold = if sign.is_some() || draft.is_some() {
        None
    } else {
        resolved.filter(|act| !is_proposal(act))
    };
    Turn { sign, draft, hold }
}

fn asks_to_confirm(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains('?') || lower.contains("say yes")
}

/// The draft question replaces a stall ("heads up"). A publish keeps the
/// model's outcome sentence, unless that sentence is still asking for a yes.
fn reply_sentence(
    model_say: &str,
    hold: Option<&ResolvedAct>,
    publish: Option<&ResolvedAct>,
) -> String {
    if let Some(act) = hold {
        return chat_act::draft_say(act);
    }
    if let Some(act) = publish {
        let say = model_say.trim();
        if say.is_empty() || asks_to_confirm(say) {
            return chat_act::published_say(act);
        }
        return say.to_string();
    }
    model_say.trim().to_string()
}

struct RunLock {
    path: std::path::PathBuf,
}

impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// One live agent per state directory. A second `run` exits instead of
/// posting another reply to the same message.
fn acquire_run_lock(dir: &std::path::Path) -> Result<RunLock, String> {
    fs::create_dir_all(dir).map_err(|error| format!("state dir: {error}"))?;
    let path = dir.join("run.lock");
    for _ in 0..2 {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let pid = std::process::id();
                writeln!(file, "{pid}").map_err(|error| format!("run lock: {error}"))?;
                let _ = fs::write(dir.join("run.pid"), format!("{pid}\n"));
                return Ok(RunLock { path });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let pid = fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| text.trim().parse::<u32>().ok())
                    .unwrap_or(0);
                if process_alive(pid) {
                    return Err(format!("org agent is already running (pid {pid})"));
                }
                let _ = fs::remove_file(&path);
            }
            Err(error) => return Err(format!("run lock: {error}")),
        }
    }
    Err("org agent is already running".to_string())
}

fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn welcomed_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("dm-welcomed.json")
}

fn load_welcomed(dir: &std::path::Path) -> HashSet<String> {
    let Ok(text) = fs::read_to_string(welcomed_path(dir)) else {
        return HashSet::new();
    };
    serde_json::from_str::<Vec<String>>(&text)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn remember_welcomed(dir: &std::path::Path, channel: &str) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|error| format!("state dir: {error}"))?;
    let mut channels = load_welcomed(dir);
    channels.insert(channel.to_string());
    let list: Vec<&String> = channels.iter().collect();
    let text = serde_json::to_string(&list).map_err(|error| format!("state json: {error}"))?;
    fs::write(welcomed_path(dir), text).map_err(|error| format!("state write: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_dm_is_the_one_other_person() {
        let me = "aa".repeat(32);
        let them = "bb".repeat(32);
        assert!(is_agent_dm(&me, std::slice::from_ref(&them)));
        let both = [me.clone(), them.clone()];
        assert!(is_agent_dm(&me, &both));
        let strangers = [them.clone(), "cc".repeat(32)];
        assert!(!is_agent_dm(&me, &strangers));
        assert!(!is_agent_dm(&me, &[]));
    }

    #[test]
    fn welcome_says_what_the_agent_can_do() {
        assert_eq!(WELCOME_LINES.len(), 1);
        let line = WELCOME_LINES[0];
        assert!(line.starts_with(
            "Hey, I'm glad to connect! I'm a powerful AI agent, that can take your organization to the next level."
        ));
        assert!(line.contains("View me as Elon Musk on steroids at your service."));
        assert!(line.ends_with("Do you have time to set up your organization now?"));
        assert!(LEGACY_WELCOME_LINES.iter().any(|old| {
            old.contains(
                "I draft for this organization — what it is for, the work, who decides, and your profile.",
            ) && old.contains(
                "Questions — Ask about anything the organization has already written down.",
            )
        }));
        assert!(LEGACY_WELCOME_LINES.iter().any(|old| {
            old.contains("Direction — Mission, vision, objectives, and strategy.")
                && old.ends_with("Are you shaping this alone, or with other people?")
        }));
        assert!(LEGACY_WELCOME_LINES.iter().any(|old| old.contains(
            "Direction — Mission, vision, where you stand, objectives, and strategy, with an honest read on each."
        )));
        assert!(!line.contains("Are you shaping this alone"));
        assert!(!line.contains("Congratulations"));
        assert!(!line.contains("Hey. I'm Org. Agent."));
    }

    #[test]
    fn a_model_invite_keeps_its_words_and_the_setup_tag() {
        let raw = r#"{"say":"Four more people. How many of them should agree before a decision passes?","direction":null,"body":null,"act":null,"setup":null}"#;
        let reply = parse_model_reply(raw);
        assert_eq!(
            reply.say,
            "Four more people. How many of them should agree before a decision passes?"
        );
        assert!(reply.setup.is_none());
        let tagged = r#"{"say":"The link is under this message. Send it to the four people.","setup":{"mode":"invite","invitees":4,"quorum":3}}"#;
        let tagged = parse_model_reply(tagged);
        assert!(tagged.say.contains("four people"));
        assert_eq!(
            tagged.setup,
            Some(vec![
                "io-setup".to_string(),
                "invite".to_string(),
                "4".to_string(),
                "3".to_string(),
            ])
        );
    }

    #[test]
    fn a_legacy_opening_still_gets_the_current_first_message() {
        let me = "aa".repeat(32);
        let mut room = fresh_room(true);
        assert!(owes_welcome(&room, &me));
        room.push(
            "",
            me.clone(),
            LEGACY_WELCOME_LINES[0].into(),
            10,
            false,
            None,
        );
        assert!(owes_welcome(&room, &me));
        room.push("", me.clone(), WELCOME_LINES[0].into(), 11, false, None);
        assert!(!owes_welcome(&room, &me));

        let mut talking = fresh_room(true);
        talking.push("", "bb".repeat(32), "Hi".into(), 10, false, None);
        talking.push(
            "",
            me.clone(),
            LEGACY_WELCOME_LINES[0].into(),
            11,
            false,
            None,
        );
        assert!(!owes_welcome(&talking, &me));
    }

    #[test]
    fn a_user_message_still_needs_a_reply_after_the_welcome() {
        let me = "aa".repeat(32);
        let them = "bb".repeat(32);
        let mut room = fresh_room(true);
        room.push("", them.clone(), "Hi".into(), 10, false, None);
        room.push("", me.clone(), WELCOME_LINES[0].into(), 11, false, None);
        room.push(
            "",
            me.clone(),
            LEGACY_WELCOME_LINES[0].into(),
            12,
            false,
            None,
        );
        assert!(needs_reply(&room, &me));
        room.push(
            "",
            me.clone(),
            "Just you, then. What is this org for?".into(),
            13,
            false,
            None,
        );
        assert!(!needs_reply(&room, &me));
        room.push("", them.clone(), "sounds good".into(), 14, false, None);
        mark_quiet(&mut room, &me);
        assert!(!needs_reply(&room, &me));
        room.push("", them, "what's our mission?".into(), 15, false, None);
        assert!(needs_reply(&room, &me));
    }

    #[test]
    fn a_channel_reply_does_not_answer_a_branch() {
        let me = "aa".repeat(32);
        let them = "bb".repeat(32);
        let root = "ab".repeat(32);
        let mut room = fresh_room(true);
        room.push(
            root.clone(),
            me.clone(),
            "No problem.".into(),
            10,
            false,
            None,
        );
        room.push(
            "user-msg",
            them,
            "testing branch".into(),
            11,
            false,
            Some(root.clone()),
        );
        room.push(
            "agent-channel",
            me.clone(),
            "I see a project.".into(),
            12,
            false,
            None,
        );
        assert!(needs_reply(&room, &me));
        room.push(
            "agent-thread",
            me.clone(),
            "In the branch.".into(),
            13,
            false,
            Some(root),
        );
        assert!(!needs_reply(&room, &me));
    }

    #[test]
    fn branch_history_is_that_thread() {
        let me = "aa".repeat(32);
        let them = "bb".repeat(32);
        let root = "ab".repeat(32);
        let mut room = fresh_room(true);
        room.push(
            root.clone(),
            me.clone(),
            "No problem.".into(),
            10,
            false,
            None,
        );
        room.push(
            "channel-line",
            them.clone(),
            "u here?".into(),
            11,
            false,
            None,
        );
        room.push(
            "user-msg",
            them,
            "testing branch".into(),
            12,
            false,
            Some(root.clone()),
        );
        let branch: Vec<_> = stored_history(&room, Some(&root))
            .into_iter()
            .map(|(_, content, _)| content)
            .collect();
        assert_eq!(branch, ["No problem.", "testing branch"]);
        let channel: Vec<_> = stored_history(&room, None)
            .into_iter()
            .map(|(_, content, _)| content)
            .collect();
        assert_eq!(channel, ["No problem.", "u here?"]);
    }

    fn chat_event(keys: &Keys, channel: &str, content: &str, extra: &[&[&str]], at: u64) -> Event {
        let mut tags = vec![Tag::parse(["h", channel]).unwrap()];
        tags.extend(extra.iter().map(|row| Tag::parse(row.to_vec()).unwrap()));
        nostr::EventBuilder::new(nostr::Kind::Custom(9), content)
            .tags(tags)
            .custom_created_at(nostr::Timestamp::from(at))
            .sign_with_keys(keys)
            .unwrap()
    }

    fn edit_event(keys: &Keys, channel: &str, target: &str, content: &str, at: u64) -> Event {
        nostr::EventBuilder::new(nostr::Kind::Custom(40003), content)
            .tags(vec![
                Tag::parse(["h", channel]).unwrap(),
                Tag::parse(["e", target]).unwrap(),
            ])
            .custom_created_at(nostr::Timestamp::from(at))
            .sign_with_keys(keys)
            .unwrap()
    }

    #[test]
    fn an_edited_draft_sentence_is_what_the_interview_reads() {
        let agent = Keys::generate();
        let me = agent.public_key().to_hex();
        let channel = "dm-1";
        let original = chat_event(
            &agent,
            channel,
            "A draft of the strategy. Open it, then publish.",
            &[],
            10,
        );
        let edit = edit_event(
            &agent,
            channel,
            &original.id.to_hex(),
            "A draft of the codebases. Open it, then publish.",
            11,
        );
        let mut rooms = HashMap::new();
        let mut dms = HashSet::new();
        let welcomed = HashSet::new();
        note_event(&original, &me, &mut rooms, &mut dms, &welcomed);
        let room = rooms.get(channel).expect("room");
        assert!(!room_asked_code(room, &me));
        note_event(&edit, &me, &mut rooms, &mut dms, &welcomed);
        let room = rooms.get(channel).expect("room");
        assert!(room_asked_code(room, &me));
        assert_eq!(
            room.messages[0].content,
            "A draft of the codebases. Open it, then publish."
        );
    }

    #[test]
    fn a_new_strategy_line_keeps_the_lines_already_drafted() {
        let agent = Keys::generate();
        let me = agent.public_key().to_hex();
        let vlad = Keys::generate();
        let channel = "shapers-1";
        let first = chat_event(
            &agent,
            channel,
            "Drafted it as the first strategy line.",
            &[&[
                "direction",
                "strategy",
                "We refuse new orgs until November.",
            ]],
            10,
        );
        let second = chat_event(
            &agent,
            channel,
            "Drafted it as the second strategy line.",
            &[&[
                "direction",
                "strategy",
                "We refuse new orgs until November.\nHypha Buzz wins when the two compete.",
            ]],
            12,
        );
        let answer = chat_event(&vlad, channel, "I will be alone on Hypha Buzz", &[], 13);
        let mut rooms = HashMap::new();
        let mut dms = HashSet::new();
        let welcomed = HashSet::new();
        for event in [&first, &second, &answer] {
            note_event(event, &me, &mut rooms, &mut dms, &welcomed);
        }
        let room = rooms.get(channel).expect("room");
        let previous = previous_draft(room, &me, "strategy", None);
        assert_eq!(
            previous,
            Some("We refuse new orgs until November.\nHypha Buzz wins when the two compete.")
        );
        let history = model_history(room, &me, None);
        assert!(history[1]
            .1
            .contains("Hypha Buzz wins when the two compete."));

        let third = "Vlad builds alone at 30 hours a week.";
        assert_eq!(
            carry_draft_lines("strategy", previous, third, "I will be alone on Hypha Buzz"),
            "We refuse new orgs until November.\nHypha Buzz wins when the two compete.\nVlad builds alone at 30 hours a week."
        );
        let whole = "We refuse new orgs until November.\nVlad builds alone at 30 hours a week.";
        assert_eq!(carry_draft_lines("strategy", previous, whole, "ok"), whole);
        assert_eq!(
            carry_draft_lines("strategy", previous, third, "remove the first two"),
            third
        );
        assert_eq!(carry_draft_lines("vision", previous, third, "ok"), third);
    }

    #[test]
    fn a_confirm_picks_up_where_that_draft_was_made() {
        let agent = Keys::generate();
        let me = agent.public_key().to_hex();
        let vlad = Keys::generate();
        let channel = "shapers-1";
        let draft = chat_event(
            &agent,
            channel,
            "A draft of the vision. Open it, then publish.",
            &[&["direction", "vision", "By 2027, a thousand organizations…"]],
            10,
        );
        let liked = chat_event(&vlad, channel, "I like your sharper draft", &[], 11);
        let mut rooms = HashMap::new();
        let mut dms = HashSet::new();
        let welcomed = HashSet::new();
        note_event(&draft, &me, &mut rooms, &mut dms, &welcomed);
        note_event(&liked, &me, &mut rooms, &mut dms, &welcomed);
        let room = rooms.get(channel).expect("room");

        assert_eq!(
            confirmed_draft_branch(room, &me, "vision"),
            Some((None, draft.id.to_hex()))
        );
        assert_eq!(confirmed_draft_branch(room, &me, "mission"), None);

        let next = chat_event(
            &agent,
            channel,
            "A draft of the situation. Open it, then publish.",
            &[&["direction", "situation", "Only an idea so far."]],
            12,
        );
        note_event(&next, &me, &mut rooms, &mut dms, &welcomed);
        let room = rooms.get(channel).expect("room");
        assert_eq!(confirmed_draft_branch(room, &me, "vision"), None);
    }

    #[test]
    fn a_confirm_in_a_thread_continues_that_thread() {
        let agent = Keys::generate();
        let me = agent.public_key().to_hex();
        let root = "ab".repeat(32);
        let channel = "shapers-1";
        let draft = chat_event(
            &agent,
            channel,
            "A draft of the mission. Open it, then publish.",
            &[
                &["e", &root, "", "reply"],
                &["direction", "mission", "We feed the street."],
            ],
            10,
        );
        let mut rooms = HashMap::new();
        note_event(
            &draft,
            &me,
            &mut rooms,
            &mut HashSet::new(),
            &HashSet::new(),
        );
        let room = rooms.get(channel).expect("room");
        assert_eq!(
            confirmed_draft_branch(room, &me, "mission"),
            Some((Some(root), draft.id.to_hex()))
        );
    }

    #[test]
    fn only_a_head_first_seen_after_the_backlog_is_a_confirm() {
        let relay = Keys::generate();
        let head = |slug: &str, version: &str| {
            nostr::EventBuilder::new(nostr::Kind::Custom(39100), "{}")
                .tags(vec![
                    Tag::parse(["d", slug]).unwrap(),
                    Tag::parse(["version", version]).unwrap(),
                ])
                .sign_with_keys(&relay)
                .unwrap()
        };
        let mission = head("mission", "1");
        let vision = head("vision", "2");
        let mut seen = HashSet::new();

        assert!(newly_confirmed(&mission, &mut seen, false).is_none());
        assert!(newly_confirmed(&mission, &mut seen, true).is_none());
        let confirmed = newly_confirmed(&vision, &mut seen, true).expect("live confirm");
        let ConfirmedAdvance::Direction(confirmed) = confirmed else {
            panic!("expected a direction confirm");
        };
        assert_eq!((confirmed.slug.as_str(), confirmed.version), ("vision", 2));
        assert!(newly_confirmed(&vision, &mut seen, true).is_none());
    }

    #[test]
    fn a_branch_message_is_answered_in_that_thread() {
        let me = "aa".repeat(32);
        let them = Keys::generate();
        let root = "ab".repeat(32);
        let parent = "cd".repeat(32);
        let channel = "channel-1";
        let direct = nostr::EventBuilder::new(nostr::Kind::Custom(9), "testing branch")
            .tags(vec![
                Tag::parse(["h", channel]).unwrap(),
                Tag::parse(["e", &root, "", "reply"]).unwrap(),
            ])
            .sign_with_keys(&them)
            .unwrap();
        let nested = nostr::EventBuilder::new(nostr::Kind::Custom(9), "still in the branch")
            .tags(vec![
                Tag::parse(["h", channel]).unwrap(),
                Tag::parse(["e", &root, "", "root"]).unwrap(),
                Tag::parse(["e", &parent, "", "reply"]).unwrap(),
            ])
            .sign_with_keys(&them)
            .unwrap();
        let mut rooms = HashMap::new();
        let mut dms = HashSet::new();
        let welcomed = HashSet::new();
        note_event(&direct, &me, &mut rooms, &mut dms, &welcomed);
        note_event(&nested, &me, &mut rooms, &mut dms, &welcomed);
        let room = rooms.get(channel).expect("room");
        let direct_line = room
            .messages
            .iter()
            .find(|line| line.content == "testing branch")
            .expect("direct");
        let target = reply_target(direct_line).expect("thread");
        assert_eq!(target.root, root);
        assert_eq!(target.parent, direct.id.to_hex());
        let nested_line = room
            .messages
            .iter()
            .find(|line| line.content == "still in the branch")
            .expect("nested");
        let nested_target = reply_target(nested_line).expect("thread");
        assert_eq!(nested_target.root, root);
        assert_eq!(nested_target.parent, nested.id.to_hex());

        let author = "ee".repeat(32);
        let tags = chat_tags(
            channel,
            &OutgoingChat {
                content: "In the branch.",
                direction: None,
                body: None,
                from: None,
                extra: &[],
                thread: Some((target.root.as_str(), target.parent.as_str())),
                reply_to: Some(&author),
            },
        )
        .expect("tags");
        let rows: Vec<Vec<String>> = tags.iter().map(|tag| tag.as_slice().to_vec()).collect();
        assert!(rows.iter().any(|row| row.as_slice() == ["h", channel]));
        assert!(rows
            .iter()
            .any(|row| row.as_slice() == ["p", author.as_str()]));
        assert!(rows
            .iter()
            .any(|row| row.as_slice() == ["e", root.as_str(), "", "root"]));
        assert!(rows
            .iter()
            .any(|row| row.as_slice() == ["e", target.parent.as_str(), "", "reply"]));

        let channel_tags = chat_tags(
            channel,
            &OutgoingChat {
                content: "In the channel.",
                direction: None,
                body: None,
                from: None,
                extra: &[],
                thread: None,
                reply_to: None,
            },
        )
        .expect("tags");
        assert!(channel_tags
            .iter()
            .all(|tag| tag.as_slice().first().map(String::as_str) != Some("e")));
    }

    #[test]
    fn a_yes_in_a_branch_does_not_take_a_channel_draft() {
        let root = "ab".repeat(32);
        let mut room = fresh_room(true);
        room.pending = Some(HeldAct {
            act: sample_project(),
            thread_root: None,
        });
        assert!(pending_for(&room, Some(&root)).is_none());
        assert!(pending_for(&room, None).is_some());
    }

    #[test]
    fn a_channel_line_is_answered_only_when_it_concerns_the_agent() {
        assert!(!should_answer_line(
            false,
            "sounds good, I'll grab coffee",
            false
        ));
        assert!(!should_answer_line(false, "what time is lunch?", false));
        assert!(should_answer_line(false, "what's our mission?", false));
        assert!(should_answer_line(
            false,
            "can you draft a project for the hall",
            false
        ));
        assert!(should_answer_line(
            false,
            "create new ticket to create automated tests",
            false
        ));
        assert!(should_answer_line(false, "hey", true));
        assert!(should_answer_line(true, "sounds good", false));
        let announced = "Opened a objectives proposal: Book the hall by March. Pay the growers the week they sell.";
        assert!(declines_reply(announced, true, true, true));
        assert!(!declines_reply(
            "let's set the objectives",
            false,
            false,
            false
        ));
    }

    #[test]
    fn a_yes_after_a_described_ticket_is_still_unfinished_when_the_opening_had_no_tags() {
        let me = "aa".repeat(32);
        let them = "bb".repeat(32);
        let mut room = fresh_room(true);
        room.push(
            "",
            me.clone(),
            "I can create a ticket called Create Automated Tests under Finish Phase 0 for Hypha Desktop App, offered to you. Want me to publish it?".into(),
            10,
            false,
            None,
        );
        room.push("", them.clone(), "yes".into(), 11, false, None);
        room.push(
            "",
            me.clone(),
            "Opening the ticket Create Automated Tests under Finish Phase 0 for Hypha Desktop App, offered to you.".into(),
            12,
            false,
            None,
        );
        let (speaker, act) = unfinished_ticket(&room, &me).expect("unfinished");
        assert_eq!(speaker, them);
        assert!(matches!(act, chat_act::RawAct::Ticket { .. }));
        room.push_acted(
            "",
            me.clone(),
            "Offering “Create Automated Tests”.".into(),
            13,
            false,
            None,
            true,
        );
        assert!(unfinished_ticket(&room, &me).is_none());
    }

    #[test]
    fn creating_the_ticket_and_naming_yourself_finishes_a_hollow_opening() {
        let me = "aa".repeat(32);
        let them = "bb".repeat(32);
        let mut room = fresh_room(true);
        room.push(
            "",
            me.clone(),
            "Opening this ticket for you under Choose Design Framework for Landing Page. Choose Colouring of the Site. No holder named yet, so I will leave it open for a match.".into(),
            10,
            false,
            None,
        );
        room.push(
            "",
            them.clone(),
            "create the ticket put me as DRI".into(),
            11,
            false,
            None,
        );
        room.push(
            "",
            me.clone(),
            "Opening that ticket for you. Choose Colouring of the Site under Choose Design Framework for Landing Page, offered to you.".into(),
            12,
            false,
            None,
        );
        let (speaker, act) = unfinished_ticket(&room, &me).expect("unfinished");
        assert_eq!(speaker, them);
        match act {
            chat_act::RawAct::Ticket { title, who, .. } => {
                assert_eq!(title, "Choose Colouring of the Site");
                assert_eq!(who, "me");
            }
            other => panic!("{other:?}"),
        }
        assert!(needs_reply(&room, &me));
    }

    #[test]
    fn a_false_opening_does_not_count_as_the_answer() {
        let me = "aa".repeat(32);
        let them = "bb".repeat(32);
        let mut room = fresh_room(true);
        room.push("", them.clone(), "yes".into(), 10, false, None);
        room.push(
            "",
            me.clone(),
            "Opening the ticket Create Automated Tests under Finish Phase 0, offered to you."
                .into(),
            11,
            false,
            None,
        );
        assert!(needs_reply(&room, &me));
        room.push_acted(
            "",
            me.clone(),
            "Offering “Create Automated Tests”.".into(),
            12,
            false,
            None,
            true,
        );
        assert!(!needs_reply(&room, &me));
    }

    #[test]
    fn visible_reply_drops_a_think_fence() {
        let raw = "<think>planning</think>\nJust you for now. What is this org for?";
        assert_eq!(
            visible_reply(raw),
            "Just you for now. What is this org for?"
        );
    }

    #[test]
    fn visible_reply_keeps_a_paragraph() {
        let paragraph = "Based on the overview, here's what I see. ".repeat(12);
        assert!(paragraph.chars().count() > 360);
        let raw = format!("<think>drafting</think>{paragraph}");
        assert_eq!(visible_reply(&raw), paragraph.trim());
    }

    #[test]
    fn a_direction_reply_names_the_slug_and_keeps_the_sentence() {
        let raw = r#"{"say":"Got it. What's the vision?","direction":"mission","body":"We exist so organizations can run themselves."}"#;
        let reply = parse_model_reply(raw);
        assert_eq!(reply.say, "Got it. What's the vision?");
        assert_eq!(reply.direction.as_deref(), Some("mission"));
        assert_eq!(
            reply.body.as_deref(),
            Some("We exist so organizations can run themselves.")
        );
    }

    #[test]
    fn a_confirmation_is_not_the_artifact() {
        let raw = r#"{"say":"Mission set.","direction":"mission","body":"yes thats good, set it"}"#;
        let reply = parse_model_reply(raw);
        assert_eq!(reply.direction, None);
        assert_eq!(reply.body, None);
    }

    #[test]
    fn a_plain_sentence_sets_no_direction() {
        let reply = parse_model_reply("<think>hmm</think>Are you shaping this alone?");
        assert_eq!(reply.say, "Are you shaping this alone?");
        assert_eq!(reply.direction, None);
    }

    #[test]
    fn an_unknown_direction_slug_is_dropped() {
        let raw = r#"{"say":"Tell me more.","direction":"budget"}"#;
        let reply = parse_model_reply(raw);
        assert_eq!(reply.say, "Tell me more.");
        assert_eq!(reply.direction, None);
    }

    fn sample_project() -> ResolvedAct {
        ResolvedAct::Project {
            title: "Testing the App".into(),
            brief: "Test the Hypha desktop app.".into(),
            due_at: 10,
            suggested: None,
            plan: None,
        }
    }

    fn sample_dri() -> ResolvedAct {
        ResolvedAct::Dri {
            item: "item-1".into(),
            title: "Hall roof".into(),
            who: "aa".repeat(32),
        }
    }

    #[test]
    fn one_turn_is_either_the_draft_or_the_action() {
        let draft = choose_turn(false, false, false, Some(sample_project()), None);
        assert!(draft.sign.is_none());
        assert!(draft.draft.is_some());
        assert!(draft.hold.is_none());

        let agreed = choose_turn(true, false, false, None, Some(sample_project()));
        assert!(agreed.sign.is_none());
        assert!(agreed.draft.is_none());

        let asked = choose_turn(false, false, false, Some(sample_project()), None);
        assert!(asked.draft.is_some());
        assert!(asked.sign.is_none());

        let dri = choose_turn(false, false, false, Some(sample_dri()), None);
        assert!(dri.draft.is_some());
        assert!(dri.sign.is_none());

        let removal = ResolvedAct::Remove {
            item: "item-1".into(),
            title: "Hall".into(),
        };
        let go = choose_turn(false, false, true, Some(removal.clone()), None);
        assert!(go.sign.is_some());
        assert!(go.draft.is_none());
        let not_a_project = choose_turn(false, false, true, Some(sample_project()), None);
        assert!(not_a_project.sign.is_none());
        assert!(not_a_project.draft.is_some());
        let pending = choose_turn(false, false, true, None, Some(removal));
        assert!(pending.sign.is_some());
    }

    #[test]
    fn the_action_keeps_the_outcome_sentence() {
        let project = sample_project();
        let draft = reply_sentence(
            "Heads up — there's already a project that sounds similar. Still want a new one?",
            Some(&project),
            None,
        );
        assert!(draft.contains("Open the draft"));
        assert!(!draft.contains("Heads up"));

        let published = reply_sentence(
            "Published! Here's the project proposal — let me know if you want to adjust the brief.",
            None,
            Some(&project),
        );
        assert!(published.starts_with("Published!"));

        let dri = sample_dri();
        let asked = reply_sentence("Name them as the holder.", Some(&dri), None);
        assert_eq!(asked, chat_act::draft_say(&dri));
    }

    #[test]
    fn profile_keeps_an_existing_picture() {
        let bare = profile_document(None);
        assert_eq!(bare["name"], ORG_AGENT_NAME);
        assert_eq!(bare["about"], "I draft. You decide.");
        assert!(bare.get("picture").is_none());

        let kept = profile_document(Some(
            r#"{"name":"Org. Agent","about":"old","picture":"http://localhost:3000/media/face.png"}"#,
        ));
        assert_eq!(kept["name"], ORG_AGENT_NAME);
        assert_eq!(kept["about"], "I draft. You decide.");
        assert_eq!(kept["picture"], "http://localhost:3000/media/face.png");

        let blank = profile_document(Some(r#"{"picture":"  "}"#));
        assert!(blank.get("picture").is_none());
        assert!(kept_profile_picture("not json").is_none());
    }

    #[test]
    fn a_second_process_does_not_take_the_run_lock() {
        let dir = tempfile::tempdir().expect("temp dir");
        let first = acquire_run_lock(dir.path()).expect("first lock");
        let second = acquire_run_lock(dir.path());
        assert!(second.is_err(), "a second run must not also answer");
        drop(first);
        let again = acquire_run_lock(dir.path());
        assert!(again.is_ok());
    }

    /// The production step call (`think_project_steps`) against the operator
    /// model. Run with `--ignored` and `IO_LIVE_PLAN=1`.
    #[tokio::test]
    #[ignore = "calls the operator model"]
    async fn live_planner_publishes_a_full_plan() {
        if std::env::var("IO_LIVE_PLAN").ok().as_deref() != Some("1") {
            return;
        }
        let case = std::env::var("IO_LIVE_CASE").unwrap_or_else(|_| "all".into());
        let river = crate::fixtures::load_events(
            &crate::fixtures::fixtures_dir().join("orgs/river/seed.json"),
        )
        .expect("river seed");
        let org = crate::state::OrgState::from_events(&river).expect("river");
        let direction = crate::plan::direction_excerpt(&org);
        if case == "all" || case == "hall" {
            let hall = think_project_steps(
            "Weekday hall trial",
            "Four weekday sessions, then a decision on whether weekday buyers come. The evening licence is not held yet.",
            &direction,
        )
        .await
        .expect("weekday hall steps");
            assert!(
                hall.iter().any(|step| !step.after.is_empty()),
                "a later step waits: {hall:?}"
            );
            publish_live("Weekday hall trial", &hall, None);
        }

        if case != "all" && case != "code" {
            return;
        }
        let digest: serde_json::Value =
            serde_json::from_str(include_str!("../tests/eval/digests/buzz.json"))
                .expect("buzz digest");
        let files: Vec<String> = digest["files"]
            .as_array()
            .expect("files")
            .iter()
            .filter_map(|path| path.as_str().map(str::to_string))
            .collect();
        let commit = digest["commit"].as_str().unwrap_or("");
        let repo = "hypha-dao/buzz-hypha";
        let block = crate::plan::digest_block(repo, commit, &files);
        let code = think_project_steps(
            "Show the next offered task",
            "The next offered task is visible on My work. Change the work model and the project page. Leave unrelated files alone.",
            &block,
        )
        .await
        .expect("code steps");
        assert!(
            code.iter()
                .any(|step| step.kind == "code" && !step.files.is_empty()),
            "a code step names digest files: {code:?}"
        );
        publish_live(
            "Show the next offered task",
            &code,
            Some((repo, commit, files.as_slice())),
        );
    }

    fn publish_live(
        title: &str,
        steps: &[buzz_core::intelligent_org::PlanStep],
        digest: Option<(&str, &str, &[String])>,
    ) {
        use buzz_core::intelligent_org::{
            DecisionRule, Executed, ProjectHome, Proposal, ProposalKind, ProposalStatus,
        };
        let mut root = crate::test_support::root_held(&"aa".repeat(32));
        root.title = title.into();
        root.due_at = 1_785_488_400;
        if let Some((repo, _, _)) = digest {
            root.home = Some(ProjectHome {
                channel: "room-1".into(),
                repo: Some(repo.into()),
                project: None,
            });
        }
        let mut state = crate::state::OrgState::new();
        state
            .item_generations
            .insert(root.id.clone(), "ab".repeat(32));
        state.items.insert(root.id.clone(), root);
        if let Some((repo, commit, files)) = digest {
            crate::plan::remember_digest(&mut state, repo, commit, files);
        }
        state.proposals.insert(
            "prop-1".into(),
            Proposal {
                id: "prop-1".into(),
                kind: ProposalKind::Project,
                status: ProposalStatus::Passed,
                opened_by: "aa".repeat(32),
                opened_at: 1,
                expires_at: 2,
                draft: None,
                payload: serde_json::json!({
                    "title": title,
                    "brief": "The work this project actually does.",
                    "plan": steps
                }),
                rule: DecisionRule::MAJORITY,
                needed: 1,
                eligible: vec![],
                votes: vec![],
                decided_at: Some(2),
                executed: Some(Executed {
                    kind: "work_item".into(),
                    id: "root-1".into(),
                }),
                settlement: None,
            },
        );
        let keys = nostr::Keys::generate();
        let events = crate::plan::take_due_tickets(&mut state, &keys).expect("publish");
        let tickets = events
            .iter()
            .filter(|event| u32::from(event.kind.as_u16()) == 50100)
            .count();
        let prompts = events
            .iter()
            .filter(|event| u32::from(event.kind.as_u16()) == 50104)
            .count();
        assert_eq!(tickets, steps.len(), "every step is a ticket");
        assert_eq!(prompts, steps.len(), "every ticket has a prompt");
        if let Some((repo, commit, _)) = digest {
            let needle = format!("{repo}@{commit}");
            assert!(
                events.iter().any(|event| {
                    u32::from(event.kind.as_u16()) == 50104 && event.content.contains(&needle)
                }),
                "a code prompt names the digest commit"
            );
        }
        eprintln!(
            "live plan {title}: {tickets} tickets, {prompts} prompts, steps={}",
            serde_json::to_string(steps).unwrap_or_default()
        );
    }
}
