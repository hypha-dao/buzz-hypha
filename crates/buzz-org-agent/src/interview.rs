//! The guided interview. Code picks the next missing row and refuses a
//! direction draft that does not meet that row. The model explains; it does
//! not decide that a vague objective is ready.

use super::{Board, CodebaseLink, RawAct};

/// Said once every row in the minimum table is confirmed. No question.
pub const READY_LINE: &str = "The org is ready. I can draft a project from an objective.";

const MISSION_Q: &str = "What do you do, and for whom?";
const VISION_Q: &str = "What does success look like, and by when?";
const SITUATION_Q: &str =
    "Where does the org stand today, and what is the one thing it must learn next?";
const OBJECTIVES_Q: &str =
    "What outcome should be true by a date, and what is the done when a person could check?";
const STRATEGY_Q: &str = "What is one bet you are making, and what will you refuse to do?";
const PEOPLE_Q: &str =
    "In one sentence, what do you do here? Then name the skills you actually have, after Skills:";
const CODE_Q: &str = "Where does the code live, or is there no repository yet?";
const SITE_Q: &str = "Where is the landing page, or is there no site yet?";

const OBJECTIVE_HOLD: &str = "That objective needs a date and a done when someone could answer yes or no. What would be true, and by when?";

/// The next thing the interview says. `asks` is false only on the ready line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterviewCue {
    Ask {
        row: &'static str,
        question: &'static str,
    },
    Ready,
}

/// A model reply after the interview has kept or dropped its draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterviewReply {
    pub say: String,
    pub direction: Option<String>,
    pub body: Option<String>,
    /// Set when the person just answered the profile row.
    pub profile: Option<RawAct>,
    /// The codebases list to draft. Confirming it does not change strategy.
    pub codebases: Option<Vec<CodebaseLink>>,
}

impl Board {
    /// The next row, from the same heads the overview lists.
    pub fn interview_cue(&self) -> InterviewCue {
        if !prose_ready(self, "mission") {
            return ask("mission", MISSION_Q);
        }
        if !prose_ready(self, "vision") {
            return ask("vision", VISION_Q);
        }
        if !prose_ready(self, "situation") {
            return ask("situation", SITUATION_Q);
        }
        // A confirmed strategy is not rewound to the objective count. The
        // Shapers already published it, so the walk continues.
        if !strategy_ready(self) {
            if !objectives_ready(self) {
                return ask("objectives", OBJECTIVES_Q);
            }
            return ask("strategy", STRATEGY_Q);
        }
        if !people_ready(self) {
            return ask("people", PEOPLE_Q);
        }
        if !code_ready(self) {
            return ask("code", CODE_Q);
        }
        if !site_ready(self) {
            return ask("site", SITE_Q);
        }
        InterviewCue::Ready
    }

    /// The row after a profile was just saved. The board may not show that
    /// profile yet, so this does not ask the profile question again.
    pub fn question_after_people(&self) -> Option<&'static str> {
        if !code_ready(self) {
            return Some(CODE_Q);
        }
        if !site_ready(self) {
            return Some(SITE_Q);
        }
        None
    }

    /// One line for the overview the model reads before it speaks.
    pub fn interview_line(&self) -> String {
        match self.interview_cue() {
            InterviewCue::Ready => READY_LINE.to_string(),
            InterviewCue::Ask { question, .. } => question.to_string(),
        }
    }
}

fn ask(row: &'static str, question: &'static str) -> InterviewCue {
    InterviewCue::Ask { row, question }
}

/// Keep a direction draft only when the body meets that row. An objective
/// with no done-when comes back as a follow-up and no draft.
pub fn apply_interview(
    board: &Board,
    user: &str,
    say: String,
    direction: Option<String>,
    body: Option<String>,
) -> InterviewReply {
    let cue = board.interview_cue();
    if let InterviewCue::Ask { row: "code", .. } = cue {
        return match repository_items(user) {
            Some(items) => codebases_reply(with_role(board, "repository", items)),
            None => ask_only(CODE_Q),
        };
    }
    if let InterviewCue::Ask { row: "site", .. } = cue {
        return match site_items(user) {
            Some(items) => codebases_reply(with_role(board, "site", items)),
            None => ask_only(SITE_Q),
        };
    }
    if let (Some(slug), Some(text)) = (direction.as_deref(), body.as_deref()) {
        return match accept_body(slug, text) {
            Ok(kept) => InterviewReply {
                say,
                direction: Some(slug.to_string()),
                body: Some(kept),
                profile: None,
                codebases: None,
            },
            Err(follow) => InterviewReply {
                say: follow,
                direction: None,
                body: None,
                profile: None,
                codebases: None,
            },
        };
    }

    if let InterviewCue::Ask { row: "people", .. } = cue {
        if let Some(act) = profile_from(user) {
            return InterviewReply {
                say: "A draft of your profile. Open it, then publish.".to_string(),
                direction: None,
                body: None,
                profile: Some(act),
                codebases: None,
            };
        }
    }

    let say = match cue {
        InterviewCue::Ready if asks_a_row(&say) => READY_LINE.to_string(),
        InterviewCue::Ask { question, .. } if declares_direction_finished(&say) => {
            question.to_string()
        }
        InterviewCue::Ask { row, question } if asks_some_other_row(&say, row) => {
            question.to_string()
        }
        _ => say,
    };
    InterviewReply {
        say,
        direction: None,
        body: None,
        profile: None,
        codebases: None,
    }
}

fn ask_only(question: &str) -> InterviewReply {
    InterviewReply {
        say: question.to_string(),
        direction: None,
        body: None,
        profile: None,
        codebases: None,
    }
}

fn codebases_reply(items: Vec<CodebaseLink>) -> InterviewReply {
    InterviewReply {
        say: "A draft of the codebases. Open it, then publish.".to_string(),
        direction: None,
        body: None,
        profile: None,
        codebases: Some(items),
    }
}

fn with_role(board: &Board, role: &str, incoming: Vec<CodebaseLink>) -> Vec<CodebaseLink> {
    let mut kept: Vec<CodebaseLink> = board
        .codebases
        .iter()
        .filter(|item| item.kind != role)
        .cloned()
        .collect();
    kept.extend(incoming);
    kept
}

fn accept_body(slug: &str, body: &str) -> Result<String, String> {
    let cleaned = body.trim();
    if cleaned.is_empty() {
        return Err(OBJECTIVE_HOLD.to_string());
    }
    if slug == "objectives" && !objective_body_is_draftable(cleaned) {
        return Err(OBJECTIVE_HOLD.to_string());
    }
    Ok(cleaned.to_string())
}

/// Every objective line has a date and a checkable done-when.
pub fn objective_body_is_draftable(body: &str) -> bool {
    let lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    !lines.is_empty() && lines.iter().copied().all(objective_line_is_ready)
}

fn objective_line_is_ready(line: &str) -> bool {
    let Some(check) = done_when_text(line) else {
        return false;
    };
    if check.chars().count() < 8 || check.chars().count() > 200 {
        return false;
    }
    has_date(line)
}

fn done_when_text(line: &str) -> Option<&str> {
    let lower = line.to_ascii_lowercase();
    let at = lower.find("done when")?;
    let rest = line[at + "done when".len()..]
        .trim_start_matches(|ch: char| ch == ':' || ch.is_whitespace());
    let end = rest
        .to_ascii_lowercase()
        .find(". by:")
        .or_else(|| rest.to_ascii_lowercase().find("\nby:"))
        .unwrap_or(rest.len());
    let check = rest[..end].trim().trim_end_matches('.');
    if check.is_empty() {
        None
    } else {
        Some(check)
    }
}

fn has_date(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    has_iso_date(&lower) || has_month(&lower)
}

fn has_iso_date(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    bytes.windows(10).any(|window| {
        window[4] == b'-'
            && window[7] == b'-'
            && window
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    })
}

fn has_month(lower: &str) -> bool {
    const MONTHS: &[&str] = &[
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
        "jan",
        "feb",
        "mar",
        "apr",
        "jun",
        "jul",
        "aug",
        "sep",
        "oct",
        "nov",
        "dec",
    ];
    MONTHS.iter().any(|month| contains_word(lower, month))
}

fn contains_word(text: &str, word: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|part| part == word)
}

fn prose_ready(board: &Board, slug: &str) -> bool {
    board
        .direction
        .get(slug)
        .is_some_and(|head| head.body.trim().chars().count() >= 12)
}

fn objectives_ready(board: &Board) -> bool {
    let Some(head) = board.direction.get("objectives") else {
        return false;
    };
    let count = head.lines.len();
    (3..=7).contains(&count)
        && head.lines.iter().all(|line| {
            line.date.is_some()
                && line
                    .done_when
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty() && text.chars().count() <= 200)
        })
}

fn strategy_ready(board: &Board) -> bool {
    let Some(head) = board.direction.get("strategy") else {
        return false;
    };
    let mut bet = false;
    let mut refusal = false;
    for line in &head.lines {
        match line.line_type.as_deref() {
            Some("bet") => bet = true,
            Some("refusal") => refusal = true,
            _ => {}
        }
    }
    bet && refusal
}

fn people_ready(board: &Board) -> bool {
    if board.shapers.is_empty() {
        return false;
    }
    board.shapers.iter().all(|pubkey| {
        board.profiles.get(pubkey).is_some_and(|profile| {
            profile.about.trim().chars().count() >= 12 && !profile.skills.is_empty()
        })
    })
}

fn code_ready(board: &Board) -> bool {
    board.codebases.iter().any(|item| item.kind == "repository")
}

fn site_ready(board: &Board) -> bool {
    board.codebases.iter().any(|item| item.kind == "site")
}

fn repository_items(user: &str) -> Option<Vec<CodebaseLink>> {
    let lower = user.to_ascii_lowercase();
    if lower.contains("no repository yet")
        || lower.contains("no repo yet")
        || lower.contains("no repositories yet")
    {
        return Some(vec![CodebaseLink {
            kind: "repository".to_string(),
            name: String::new(),
            url: String::new(),
            about: "no repository yet".to_string(),
            ..Default::default()
        }]);
    }
    let items: Vec<CodebaseLink> = https_urls(user)
        .into_iter()
        .map(|url| CodebaseLink {
            kind: "repository".to_string(),
            name: name_from_url(url),
            url: url.to_string(),
            about: String::new(),
            ..Default::default()
        })
        .collect();
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

fn site_items(user: &str) -> Option<Vec<CodebaseLink>> {
    if let Some(url) = https_urls(user)
        .into_iter()
        .find(|url| !url.to_ascii_lowercase().contains("github.com"))
    {
        return Some(vec![CodebaseLink {
            kind: "site".to_string(),
            name: name_from_url(url),
            url: url.to_string(),
            about: String::new(),
            ..Default::default()
        }]);
    }
    if declines_site(user) {
        return Some(vec![CodebaseLink {
            kind: "site".to_string(),
            name: String::new(),
            url: String::new(),
            about: "no landing page yet".to_string(),
            ..Default::default()
        }]);
    }
    None
}

/// "No public site yet" and "no site" are the same answer as "no site yet".
fn declines_site(user: &str) -> bool {
    let lower = user.to_ascii_lowercase().replace("public ", "");
    lower.contains("no site")
        || lower.contains("no landing")
        || lower.contains("no website")
        || lower.contains("no web site")
        || lower.contains("isn't one")
        || lower.contains("isnt one")
        || lower.contains("there isn't")
        || lower.contains("there isnt")
        || lower.contains("there is no")
        || lower.contains("don't have")
        || lower.contains("dont have")
        || lower.contains("do not have")
        || lower.contains("none yet")
        || lower.contains("not yet")
}

fn name_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let path = path.trim_end_matches('/');
    let name = path.rsplit('/').next().unwrap_or("repository");
    let name = name.trim_end_matches(".git");
    if name.is_empty() || name.contains(':') {
        "repository".to_string()
    } else {
        name.to_string()
    }
}

fn https_urls(user: &str) -> Vec<&str> {
    let mut urls = Vec::new();
    let mut rest = user;
    while let Some(start) = rest.find("https://").or_else(|| rest.find("http://")) {
        let from = &rest[start..];
        let end = from
            .find(|ch: char| ch.is_whitespace() || ch == ')' || ch == '>')
            .unwrap_or(from.len());
        let url = from[..end].trim_end_matches(['.', ',', ';']);
        if url.len() >= 12 {
            urls.push(url);
        }
        if end == 0 {
            break;
        }
        rest = &from[end..];
    }
    urls
}

/// The model closed the interview and offered a project while a row is open.
fn declares_direction_finished(say: &str) -> bool {
    let lower = say.to_ascii_lowercase();
    lower.contains("direction is set")
        || lower.contains("the org is ready")
        || (lower.contains("project") && lower.contains("in the draft"))
}

fn profile_from(user: &str) -> Option<RawAct> {
    let lower = user.to_ascii_lowercase();
    let at = lower.find("skills:")?;
    let about = user[..at].trim().trim_end_matches(['.', ':']).trim();
    if about.chars().count() < 12 {
        return None;
    }
    let skills: Vec<String> = user[at + "skills:".len()..]
        .split(',')
        .map(str::trim)
        .filter(|skill| !skill.is_empty())
        .map(str::to_string)
        .collect();
    if skills.is_empty() {
        return None;
    }
    Some(RawAct::Profile {
        about: Some(about.to_string()),
        skills: Some(skills),
        socials: None,
    })
}

fn asks_a_row(say: &str) -> bool {
    say.contains('?')
        && [
            "mission",
            "vision",
            "situation",
            "objective",
            "strategy",
            "skill",
            "repository",
            "landing",
        ]
        .iter()
        .any(|word| say.to_ascii_lowercase().contains(word))
}

fn asks_some_other_row(say: &str, next: &str) -> bool {
    if !say.contains('?') {
        return false;
    }
    const ROWS: &[(&str, &str)] = &[
        ("mission", "mission"),
        ("vision", "vision"),
        ("situation", "situation"),
        ("objectives", "objective"),
        ("strategy", "strategy"),
        ("people", "skill"),
        ("code", "repository"),
        ("site", "landing"),
    ];
    let lower = say.to_ascii_lowercase();
    ROWS.iter()
        .any(|(row, word)| *row != next && lower.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ME: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn prose(board: &mut Board, slug: &str, body: &str) {
        board.observe(
            39100,
            "relay",
            20,
            Some(slug),
            &json!({ "version": 1, "body": body }).to_string(),
        );
    }

    fn mission_and_vision() -> Board {
        let mut board = Board::default();
        prose(
            &mut board,
            "mission",
            "We sell growers' produce at a Saturday stall.",
        );
        prose(
            &mut board,
            "vision",
            "A weekday night market is full by June 2028.",
        );
        board
    }

    #[test]
    fn a_board_with_mission_and_vision_asks_situation_not_mission() {
        let board = mission_and_vision();
        let cue = board.interview_cue();
        let InterviewCue::Ask { row, question } = cue else {
            panic!("expected a question, got {cue:?}");
        };
        assert_eq!(row, "situation");
        assert!(question.contains('?'));
        assert!(!question.to_ascii_lowercase().contains("mission"));
        let shaped = apply_interview(
            &board,
            "ok",
            "What should the mission be?".to_string(),
            None,
            None,
        );
        assert!(shaped.say.to_ascii_lowercase().contains("stand today"));
        assert!(shaped.say.contains('?'));
        assert!(!shaped.say.to_ascii_lowercase().contains("mission"));
        assert!(shaped.direction.is_none());
        assert!(board
            .overview()
            .contains("Interview: Where does the org stand today"));
    }

    #[test]
    fn a_board_with_every_row_confirmed_is_ready_and_asks_nothing() {
        let mut board = mission_and_vision();
        board.observe(
            39103,
            "relay",
            10,
            Some("shapers"),
            &json!({ "shapers": [ME], "room": "room-1" }).to_string(),
        );
        prose(
            &mut board,
            "situation",
            "A Saturday stall since March, three growers, no weekday night. The next unknown is whether weekday buyers will come.",
        );
        board.observe(
            39100,
            "relay",
            30,
            Some("objectives"),
            &json!({
                "version": 1,
                "body": "Three outcomes.",
                "lines": [
                    {"n": 1, "id": "a", "text": "Weekday trial", "date": 1_780_000_000, "done_when": "one paid night happened"},
                    {"n": 2, "id": "b", "text": "Three growers stay", "date": 1_790_000_000, "done_when": "all three sold that night"},
                    {"n": 3, "id": "c", "text": "A second date", "date": 1_800_000_000, "done_when": "a second night is booked"}
                ]
            })
            .to_string(),
        );
        board.observe(
            39100,
            "relay",
            31,
            Some("strategy"),
            &json!({
                "version": 1,
                "body": "How we get there.",
                "lines": [
                    {"n": 1, "id": "d", "text": "Try one Thursday before any build", "type": "bet"},
                    {"n": 2, "id": "e", "text": "No brand money", "type": "refusal"}
                ]
            })
            .to_string(),
        );
        board.observe(
            39106,
            "relay",
            31,
            Some("codebases"),
            &json!({
                "slug": "codebases",
                "version": 1,
                "items": [
                    {"kind": "repository", "name": "", "url": "", "about": "no repository yet"},
                    {"kind": "site", "name": "", "url": "", "about": "no landing page yet"}
                ]
            })
            .to_string(),
        );
        board.observe(
            39105,
            "relay",
            32,
            Some(ME),
            &json!({ "about": "I run the Saturday stall.", "skills": ["hosting"] }).to_string(),
        );
        assert_eq!(board.interview_line(), READY_LINE);
        assert!(!board.interview_line().contains('?'));
        let shaped = apply_interview(
            &board,
            "what next",
            "What should the mission be?".to_string(),
            None,
            None,
        );
        assert_eq!(shaped.say, READY_LINE);
        assert!(!shaped.say.contains('?'));
        assert!(shaped.direction.is_none());
    }

    #[test]
    fn an_objective_without_done_when_is_not_a_draft() {
        let board = mission_and_vision();
        let vague = apply_interview(
            &board,
            "be more visible",
            "Drafted.".to_string(),
            Some("objectives".to_string()),
            Some("be more visible".to_string()),
        );
        assert!(vague.direction.is_none());
        assert!(vague.body.is_none());
        assert!(vague.say.to_ascii_lowercase().contains("done when"));
        assert!(vague.say.contains('?'));

        let ready = apply_interview(
            &board,
            "the hall",
            "Drafted.".to_string(),
            Some("objectives".to_string()),
            Some(
                "A weekday hall is open. Done when: the hall has hosted one paid night. By: 2026-06-01"
                    .to_string(),
            ),
        );
        assert_eq!(ready.direction.as_deref(), Some("objectives"));
        assert!(ready.body.unwrap().contains("Done when:"));
    }

    fn strategy_without_code_or_site(board: &mut Board) {
        board.observe(
            39103,
            "relay",
            10,
            Some("shapers"),
            &json!({ "shapers": [ME], "room": "room-1" }).to_string(),
        );
        prose(
            board,
            "situation",
            "A Saturday stall since March, three growers, no weekday night. The next unknown is whether weekday buyers will come.",
        );
        board.observe(
            39100,
            "relay",
            30,
            Some("objectives"),
            &json!({
                "version": 1,
                "body": "Two outcomes.",
                "lines": [
                    {"n": 1, "id": "a", "text": "Weekday trial", "date": 1_780_000_000, "done_when": "one paid night happened"},
                    {"n": 2, "id": "b", "text": "Three growers stay", "date": 1_790_000_000, "done_when": "all three sold that night"}
                ]
            })
            .to_string(),
        );
        board.observe(
            39100,
            "relay",
            31,
            Some("strategy"),
            &json!({
                "version": 1,
                "body": "How we get there.",
                "lines": [
                    {"n": 1, "id": "d", "text": "Try one Thursday before any build", "type": "bet"},
                    {"n": 2, "id": "e", "text": "No brand money", "type": "refusal"}
                ]
            })
            .to_string(),
        );
    }

    #[test]
    fn a_confirmed_strategy_asks_for_the_profile_then_the_code_then_the_site() {
        let mut board = mission_and_vision();
        strategy_without_code_or_site(&mut board);
        let InterviewCue::Ask { row, question } = board.interview_cue() else {
            panic!("expected the profile question");
        };
        assert_eq!(row, "people");
        assert!(question.contains("Skills:"));
        assert_eq!(
            board.question_after_people(),
            Some("Where does the code live, or is there no repository yet?")
        );

        let jumped = apply_interview(
            &board,
            "ok",
            "The strategy is in, so the direction is set. The first project is in the draft below."
                .to_string(),
            None,
            None,
        );
        assert!(jumped.say.contains("Skills:"));
        assert!(jumped.direction.is_none());
        assert!(!jumped.say.to_ascii_lowercase().contains("direction is set"));

        board.observe(
            39105,
            "relay",
            32,
            Some(ME),
            &json!({ "about": "I run the Saturday stall.", "skills": ["hosting"] }).to_string(),
        );
        let InterviewCue::Ask { row, .. } = board.interview_cue() else {
            panic!("expected the code question");
        };
        assert_eq!(row, "code");
        let strategy_version = board.direction.get("strategy").map(|head| head.version);
        let repos = apply_interview(
            &board,
            "https://github.com/hypha/relay and https://github.com/hypha/desktop",
            "A draft of the strategy. Open it, then publish.".to_string(),
            Some("strategy".to_string()),
            Some("code lives at https://github.com/hypha/relay. Type: rule".to_string()),
        );
        assert!(repos.direction.is_none());
        assert!(repos.say.contains("codebases"));
        let listed = repos.codebases.expect("codebases");
        assert!(listed.iter().all(|item| item.kind == "repository"));
        assert!(listed
            .iter()
            .any(|item| item.url == "https://github.com/hypha/relay"));
        assert!(listed
            .iter()
            .any(|item| item.url == "https://github.com/hypha/desktop"));
        assert_eq!(
            board.direction.get("strategy").map(|head| head.version),
            strategy_version
        );

        board.observe(
            39100,
            "relay",
            33,
            Some("strategy"),
            &json!({
                "version": 2,
                "body": "How we get there.",
                "lines": [
                    {"n": 1, "id": "d", "text": "Try one Thursday before any build", "type": "bet"},
                    {"n": 2, "id": "e", "text": "No brand money", "type": "refusal"},
                    {"n": 3, "id": "f", "text": "code lives at https://github.com/hypha/relay", "type": "rule"}
                ]
            })
            .to_string(),
        );
        let InterviewCue::Ask { row, .. } = board.interview_cue() else {
            panic!("a strategy line is not the codebase list");
        };
        assert_eq!(row, "code");
        board.observe(
            39106,
            "relay",
            34,
            Some("codebases"),
            &json!({
                "slug": "codebases",
                "version": 1,
                "items": [
                    {"kind": "repository", "name": "relay", "url": "https://github.com/hypha/relay", "about": "The relay"},
                    {"kind": "repository", "name": "desktop", "url": "https://github.com/hypha/desktop", "about": "The desktop"}
                ]
            })
            .to_string(),
        );
        let InterviewCue::Ask { row, question } = board.interview_cue() else {
            panic!("expected the landing page question");
        };
        assert_eq!(row, "site");
        assert!(question.to_ascii_lowercase().contains("landing page"));
        assert_eq!(
            board.question_after_people(),
            Some("Where is the landing page, or is there no site yet?")
        );
        let site = apply_interview(
            &board,
            "the site is https://hypha.earth and the code is https://github.com/hypha/relay",
            "Where?".to_string(),
            None,
            None,
        );
        assert!(site.direction.is_none());
        let items = site.codebases.expect("codebases");
        assert!(items.iter().any(|item| {
            item.kind == "repository" && item.url == "https://github.com/hypha/relay"
        }));
        assert!(items
            .iter()
            .any(|item| item.kind == "site" && item.url == "https://hypha.earth"));
        let github_only = apply_interview(
            &board,
            "https://github.com/hypha/relay",
            "A draft of the strategy.".to_string(),
            Some("strategy".to_string()),
            Some("code lives at https://github.com/hypha/relay. Type: rule".to_string()),
        );
        assert!(github_only.direction.is_none());
        assert!(github_only.codebases.is_none());
        assert!(github_only
            .say
            .to_ascii_lowercase()
            .contains("landing page"));
        for answer in ["no public site yet", "no site"] {
            let declined = apply_interview(&board, answer, "Where?".to_string(), None, None);
            assert!(declined.direction.is_none(), "{answer}");
            assert!(declined.say.contains("codebases"), "{answer}");
            let items = declined.codebases.expect(answer);
            assert!(
                items.iter().any(|item| item.kind == "repository"),
                "{answer}"
            );
            assert!(
                items.iter().any(|item| {
                    item.kind == "site"
                        && item.url.is_empty()
                        && item.about == "no landing page yet"
                }),
                "{answer}"
            );
        }
    }
}
