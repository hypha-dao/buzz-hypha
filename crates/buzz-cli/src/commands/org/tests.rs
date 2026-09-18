//! Argument → event (or REQ filter) for every `buzz org` verb.
//!
//! Each test parses CLI args through clap, runs [`super::plan`] (the
//! production seam), and asserts the kind, tags, and content. Writes go
//! through `build_io_*`; reads are one filter.

use clap::{Parser, Subcommand};
use nostr::{Event, Keys, Tag};
use serde_json::{json, Value};
use uuid::Uuid;

use super::build::{plan, uuid_tag, work_item_matches_root, OrgPlan, PROGRESS_NOT_IMPLEMENTED};
use super::OrgCmd;
use crate::error::CliError;

const PK: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const EV: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const ID: &str = "7f3a0000-0000-4000-8000-000000000001";
const ID2: &str = "7f3a0000-0000-4000-8000-000000000002";

#[derive(Parser)]
#[command(name = "buzz")]
struct Wrap {
    #[command(subcommand)]
    cmd: Inner,
}

#[derive(Subcommand)]
enum Inner {
    #[command(subcommand)]
    Org(OrgCmd),
}

fn parse_org(args: &[&str]) -> OrgCmd {
    match Wrap::try_parse_from(std::iter::once("buzz").chain(args.iter().copied())) {
        Ok(Wrap {
            cmd: Inner::Org(cmd),
        }) => cmd,
        Err(e) => panic!("parse failed: {e}"),
    }
}

fn keys_and_me() -> (Keys, String) {
    let keys = Keys::generate();
    let me = keys.public_key().to_hex();
    (keys, me)
}

fn sign(plan: OrgPlan, keys: &Keys) -> Event {
    match plan {
        OrgPlan::Write(builder) => builder.sign_with_keys(keys).expect("sign"),
        OrgPlan::Query { .. } => panic!("expected write, got query"),
        OrgPlan::NotImplemented => panic!("expected write, got not-implemented"),
    }
}

fn filter(plan: OrgPlan) -> Value {
    match plan {
        OrgPlan::Query {
            mut filters,
            tree_root,
        } => {
            assert!(tree_root.is_none(), "unexpected tree_root on a plain query");
            assert_eq!(filters.len(), 1, "expected one filter");
            filters.remove(0)
        }
        OrgPlan::Write(_) => panic!("expected query, got write"),
        OrgPlan::NotImplemented => panic!("expected query, got not-implemented"),
    }
}

fn filters(plan: OrgPlan) -> (Vec<Value>, Option<Uuid>) {
    match plan {
        OrgPlan::Query { filters, tree_root } => (filters, tree_root),
        OrgPlan::Write(_) => panic!("expected query, got write"),
        OrgPlan::NotImplemented => panic!("expected query, got not-implemented"),
    }
}

fn tags(event: &Event) -> Vec<Vec<String>> {
    event.tags.iter().map(|t| t.as_slice().to_vec()).collect()
}

fn tag_slices(event: &Event) -> Vec<&[String]> {
    event.tags.iter().map(Tag::as_slice).collect()
}

fn content(event: &Event) -> Value {
    serde_json::from_str(&event.content).expect("content is json")
}

fn id(s: &str) -> Uuid {
    Uuid::parse_str(s).expect("uuid")
}

// ── bootstrap ───────────────────────────────────────────────────────────────

#[test]
fn bootstrap_args_build_owner_self_add() {
    let (keys, me) = keys_and_me();
    let event = sign(plan(&parse_org(&["org", "bootstrap"]), &me).unwrap(), &keys);
    assert_eq!(event.kind.as_u16(), 50001);
    assert_eq!(
        tags(&event),
        vec![
            vec!["op".to_string(), "add".to_string()],
            vec!["p".to_string(), me.clone()],
        ]
    );
    assert_eq!(event.content, "{}");
}

#[test]
fn bootstrap_why_is_content() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&["org", "bootstrap", "--why", "first seat"]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(content(&event), json!({ "why": "first seat" }));
    assert_eq!(tags(&event)[1], vec!["p".to_string(), me]);
}

// ── shapers ─────────────────────────────────────────────────────────────────

#[test]
fn shapers_list_is_39103_filter() {
    let (_, me) = keys_and_me();
    let f = filter(plan(&parse_org(&["org", "shapers", "list"]), &me).unwrap());
    assert_eq!(f["kinds"], json!([39103]));
}

#[test]
fn shapers_add_args_build_50001() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "shapers", "add", PK]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50001);
    assert_eq!(
        tags(&event),
        vec![
            vec!["op".to_string(), "add".to_string()],
            vec!["p".to_string(), PK.to_string()],
            vec!["vote".to_string(), "agree".to_string()],
        ]
    );
}

#[test]
fn shapers_remove_args_build_50001() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "shapers", "remove", PK]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50001);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["op".to_string(), "remove".to_string()]));
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["p".to_string(), PK.to_string()]));
}

#[test]
fn shapers_rules_args_build_50001() {
    let (keys, me) = keys_and_me();
    let json = r#"{"rules":{},"decision_window_secs":86400}"#;
    let event = sign(
        plan(&parse_org(&["org", "shapers", "rules", json]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50001);
    assert_eq!(tags(&event)[0], vec!["op".to_string(), "rules".to_string()]);
    assert_eq!(content(&event)["decision_window_secs"], 86400);
}

#[test]
fn shapers_agent_args_build_50001() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "shapers", "agent", PK]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50001);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["op".to_string(), "agent".to_string()]));
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["p".to_string(), PK.to_string()]));
}

#[test]
fn shapers_agent_hosted_omits_p() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "shapers", "agent"]), &me).unwrap(),
        &keys,
    );
    assert!(tags(&event)
        .iter()
        .all(|t| t.first().map(String::as_str) != Some("p")));
}

#[test]
fn shapers_accept_e_tag_is_proposal_uuid() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "shapers", "accept", ID]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50019);
    assert_eq!(uuid_tag(tag_slices(&event), "e"), Some(id(ID)));
    assert_eq!(event.content, "{}");
}

#[test]
fn shapers_step_down_args_build_50020() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&["org", "shapers", "step-down", "--why", "moving on"]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50020);
    assert!(tags(&event).is_empty());
    assert_eq!(content(&event), json!({ "why": "moving on" }));
}

// ── direction ───────────────────────────────────────────────────────────────

#[test]
fn direction_show_is_39100_filter() {
    let (_, me) = keys_and_me();
    let all = filter(plan(&parse_org(&["org", "direction", "show"]), &me).unwrap());
    assert_eq!(all["kinds"], json!([39100]));
    assert!(all.get("#d").is_none());
    let one = filter(plan(&parse_org(&["org", "direction", "show", "mission"]), &me).unwrap());
    assert_eq!(one["#d"], json!(["mission"]));
}

#[test]
fn direction_history_is_50002_filter() {
    let (_, me) = keys_and_me();
    let f = filter(
        plan(
            &parse_org(&["org", "direction", "history", "objectives"]),
            &me,
        )
        .unwrap(),
    );
    assert_eq!(f["kinds"], json!([50002]));
    assert_eq!(f["#d"], json!(["objectives"]));
}

#[test]
fn direction_propose_args_build_50002() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&[
                "org",
                "direction",
                "propose",
                "mission",
                "--base",
                "3",
                "--body",
                "A river through the city.",
                "--why",
                "heard in #shapers",
            ]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50002);
    assert_eq!(
        tags(&event),
        vec![
            vec!["d".to_string(), "mission".to_string()],
            vec!["base".to_string(), "3".to_string()],
            vec!["vote".to_string(), "agree".to_string()],
        ]
    );
    assert_eq!(content(&event)["body"], "A river through the city.");
}

// ── proposals ───────────────────────────────────────────────────────────────

#[test]
fn proposals_list_is_39102_filter() {
    let (_, me) = keys_and_me();
    let f = filter(
        plan(
            &parse_org(&[
                "org",
                "proposals",
                "list",
                "--kind",
                "shapers",
                "--status",
                "open",
            ]),
            &me,
        )
        .unwrap(),
    );
    assert_eq!(f["kinds"], json!([39102]));
    assert_eq!(f["#t"], json!(["shapers"]));
    assert_eq!(f["#s"], json!(["open"]));
}

#[test]
fn proposals_show_is_39102_d_filter() {
    let (_, me) = keys_and_me();
    let f = filter(plan(&parse_org(&["org", "proposals", "show", ID]), &me).unwrap());
    assert_eq!(f["kinds"], json!([39102]));
    assert_eq!(f["#d"], json!([ID]));
}

#[test]
fn proposals_vote_e_tag_is_proposal_uuid() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&[
                "org",
                "proposals",
                "vote",
                ID,
                "decline",
                "--reason",
                "too vague",
            ]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50003);
    assert_eq!(
        tags(&event),
        vec![
            vec!["e".to_string(), ID.to_string()],
            vec!["vote".to_string(), "decline".to_string()],
        ]
    );
    assert_eq!(content(&event), json!({ "reason": "too vague" }));
}

/// 50003 / 50019 store a proposal UUID in `e`. Read it with `uuid_tag`,
/// never through nostr's EventId helpers (progress-log follow-up).
#[test]
fn vote_and_accept_e_tag_is_read_as_plain_uuid() {
    let (keys, me) = keys_and_me();
    let vote = sign(
        plan(&parse_org(&["org", "proposals", "vote", ID, "agree"]), &me).unwrap(),
        &keys,
    );
    let accept = sign(
        plan(&parse_org(&["org", "shapers", "accept", ID]), &me).unwrap(),
        &keys,
    );
    for event in [&vote, &accept] {
        assert_eq!(uuid_tag(tag_slices(event), "e"), Some(id(ID)));
        assert_eq!(
            event.tags.event_ids().count(),
            0,
            "Tags::event_ids must not treat a proposal UUID as an EventId"
        );
        for tag in event.tags.iter() {
            if tag.as_slice().first().map(String::as_str) == Some("e") {
                assert!(
                    tag.as_standardized().is_none(),
                    "as_standardized must not accept a UUID e-tag"
                );
            }
        }
    }
}

#[test]
fn proposals_propose_project_args_build_50004() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&[
                "org",
                "proposals",
                "propose-project",
                "--title",
                "Weekday hall",
                "--brief",
                "md",
                "--due-at",
                "1785000000",
                "--objective-ref",
                "objectives@3#l_7f3a",
                "--suggested-dri",
                PK,
            ]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50004);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["vote".to_string(), "agree".to_string()]));
    assert_eq!(content(&event)["title"], "Weekday hall");
}

#[test]
fn proposals_propose_dri_args_build_50015() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&["org", "proposals", "propose-dri", ID, PK]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50015);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["i".to_string(), ID.to_string()]));
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["p".to_string(), PK.to_string()]));
}

// ── work ────────────────────────────────────────────────────────────────────

#[test]
fn work_tree_is_39101_filter() {
    let (_, me) = keys_and_me();
    let (fs, root) = filters(plan(&parse_org(&["org", "work", "tree"]), &me).unwrap());
    assert_eq!(fs[0]["kinds"], json!([39101]));
    assert!(root.is_none());
    let (fs, root) =
        filters(plan(&parse_org(&["org", "work", "tree", "--root", ID]), &me).unwrap());
    assert_eq!(fs[0]["kinds"], json!([39101]));
    assert_eq!(root, Some(id(ID)));
}

#[test]
fn work_show_is_39101_d_filter() {
    let (_, me) = keys_and_me();
    let f = filter(plan(&parse_org(&["org", "work", "show", ID]), &me).unwrap());
    assert_eq!(f["kinds"], json!([39101]));
    assert_eq!(f["#d"], json!([ID]));
}

#[test]
fn work_create_args_build_50005() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&[
                "org",
                "work",
                "create",
                ID,
                "--title",
                "Permit",
                "--brief",
                "md",
                "--due-at",
                "1780000000",
                "--offer-to",
                PK,
                "--after",
                ID2,
            ]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50005);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["u".to_string(), ID.to_string()]));
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["p".to_string(), PK.to_string()]));
    assert_eq!(content(&event)["title"], "Permit");
    assert_eq!(content(&event)["after"], json!([ID2]));
}

#[test]
fn work_offer_args_build_50006() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "work", "offer", ID, PK]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50006);
    assert_eq!(
        tags(&event),
        vec![
            vec!["i".to_string(), ID.to_string()],
            vec!["p".to_string(), PK.to_string()]
        ]
    );
}

#[test]
fn work_accept_args_build_50007() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "work", "accept", ID]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50007);
    assert_eq!(tags(&event), vec![vec!["i".to_string(), ID.to_string()]]);
}

#[test]
fn work_decline_args_build_50008() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "work", "decline", ID]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50008);
}

#[test]
fn work_done_args_build_50009() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&["org", "work", "done", ID, "--receipt", EV]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50009);
    assert!(tags(&event)
        .iter()
        .any(|t| t.as_slice() == ["e", EV, "", "receipt"]));
}

#[test]
fn work_release_args_build_50010() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "work", "release", ID]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50010);
}

#[test]
fn work_set_due_args_build_50011() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&["org", "work", "set-due", ID, "--due", "1780000000"]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50011);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["due".to_string(), "1780000000".to_string()]));
}

#[test]
fn work_reopen_args_build_50018() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(&parse_org(&["org", "work", "reopen", ID]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50018);
}

#[test]
fn work_my_work_is_39101_p_filter() {
    let (_, me) = keys_and_me();
    let f = filter(plan(&parse_org(&["org", "work", "my-work"]), &me).unwrap());
    assert_eq!(f["kinds"], json!([39101]));
    assert_eq!(f["#p"], json!([me]));
}

#[test]
fn work_tree_root_fence_reads_d_and_root_as_uuid() {
    let root = json!({
        "tags": [["d", ID], ["s", "open"]],
    });
    let child = json!({
        "tags": [["d", ID2], ["root", ID]],
    });
    let other = json!({
        "tags": [["d", "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"], ["root", "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"]],
    });
    assert!(work_item_matches_root(&root, id(ID)));
    assert!(work_item_matches_root(&child, id(ID)));
    assert!(!work_item_matches_root(&other, id(ID)));
}

// ── drafts ──────────────────────────────────────────────────────────────────

#[test]
fn drafts_list_is_50100_39104_filter() {
    let (_, me) = keys_and_me();
    let f = filter(plan(&parse_org(&["org", "drafts", "list"]), &me).unwrap());
    assert_eq!(f["kinds"], json!([50100, 39104]));
}

#[test]
fn drafts_list_needs_me_filters_n() {
    let (_, me) = keys_and_me();
    let f = filter(plan(&parse_org(&["org", "drafts", "list", "--needs", "me"]), &me).unwrap());
    assert_eq!(f["kinds"], json!([50100]));
    assert_eq!(f["#n"], json!([me]));
    let f = filter(
        plan(
            &parse_org(&["org", "drafts", "list", "--needs", "shaper"]),
            &me,
        )
        .unwrap(),
    );
    assert_eq!(f["#n"], json!(["shaper"]));
}

#[test]
fn drafts_show_is_id_and_39104() {
    let (_, me) = keys_and_me();
    let (fs, _) = filters(plan(&parse_org(&["org", "drafts", "show", EV]), &me).unwrap());
    assert_eq!(fs.len(), 2);
    assert_eq!(fs[0]["ids"], json!([EV]));
    assert_eq!(fs[1]["kinds"], json!([39104]));
    assert_eq!(fs[1]["#d"], json!([EV]));
}

#[test]
fn drafts_decide_args_build_50012() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&[
                "org", "drafts", "decide", EV, "decline", "--reason", "too_big",
            ]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50012);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["e".to_string(), EV.to_string()]));
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["outcome".to_string(), "decline".to_string()]));
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["reason".to_string(), "too_big".to_string()]));
}

#[test]
fn drafts_publish_args_build_50100() {
    let (keys, me) = keys_and_me();
    let raw = format!(
        r#"{{"needs":"shaper","t":"done","move":3,"origin":"gap","gap":"{ID}","receipts":[{{"event":"{EV}"}}],"payload":{{"item":"{ID}","why":"heard on a call"}}}}"#
    );
    let event = sign(
        plan(&parse_org(&["org", "drafts", "publish", &raw]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50100);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["n".to_string(), "shaper".to_string()]));
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["t".to_string(), "done".to_string()]));
}

// ── health ──────────────────────────────────────────────────────────────────

#[test]
fn health_show_is_50101_filter() {
    let (_, me) = keys_and_me();
    let f = filter(plan(&parse_org(&["org", "health", "show", ID]), &me).unwrap());
    assert_eq!(f["kinds"], json!([50101]));
    assert_eq!(f["#i"], json!([ID]));
}

#[test]
fn health_rate_args_build_50017() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&["org", "health", "rate", ID, "2026-W38", "wobbly"]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50017);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["week".to_string(), "2026-W38".to_string()]));
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["band".to_string(), "wobbly".to_string()]));
}

// ── profile ─────────────────────────────────────────────────────────────────

#[test]
fn profile_show_is_39105_filter() {
    let (_, me) = keys_and_me();
    let mine = filter(plan(&parse_org(&["org", "profile", "show"]), &me).unwrap());
    assert_eq!(mine["kinds"], json!([39105]));
    assert_eq!(mine["#d"], json!([me]));
    let theirs = filter(plan(&parse_org(&["org", "profile", "show", PK]), &me).unwrap());
    assert_eq!(theirs["#d"], json!([PK]));
}

#[test]
fn profile_set_args_build_50021() {
    let (keys, me) = keys_and_me();
    let event = sign(
        plan(
            &parse_org(&[
                "org",
                "profile",
                "set",
                "--about",
                "I book halls.",
                "--skill",
                "spanish",
                "--limit",
                "3",
            ]),
            &me,
        )
        .unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50021);
    assert_eq!(content(&event)["about"], "I book halls.");
    assert_eq!(content(&event)["skills"], json!(["spanish"]));
    assert_eq!(content(&event)["open_limit"], 3);
}

#[test]
fn profile_who_can_is_39105_k_filter() {
    let (_, me) = keys_and_me();
    let f = filter(plan(&parse_org(&["org", "profile", "who-can", "spanish"]), &me).unwrap());
    assert_eq!(f["kinds"], json!([39105]));
    assert_eq!(f["#k"], json!(["spanish"]));
}

// ── progress / ledger / tally ───────────────────────────────────────────────

#[test]
fn progress_note_is_not_implemented() {
    let (_, me) = keys_and_me();
    match plan(&parse_org(&["org", "progress", "note"]), &me).unwrap() {
        OrgPlan::NotImplemented => {}
        _ => panic!("progress note must refuse"),
    }
    assert_eq!(PROGRESS_NOT_IMPLEMENTED, "not implemented");
}

#[test]
fn ledger_list_is_command_kinds_filter() {
    let (_, me) = keys_and_me();
    let f = filter(
        plan(
            &parse_org(&["org", "ledger", "list", "--item", ID, "--since", "1"]),
            &me,
        )
        .unwrap(),
    );
    let kinds = f["kinds"].as_array().expect("kinds");
    assert!(kinds.iter().any(|k| k == 50001));
    assert!(kinds.iter().any(|k| k == 50021));
    assert_eq!(f["#i"], json!([ID]));
    assert_eq!(f["since"], 1);
}

#[test]
fn ledger_note_args_build_50103() {
    let (keys, me) = keys_and_me();
    let raw = r#"{"note":"budget_exhausted","budget":"calls_per_hour","until":1,"item":"7f3a0000-0000-4000-8000-000000000001"}"#;
    let event = sign(
        plan(&parse_org(&["org", "ledger", "note", raw]), &me).unwrap(),
        &keys,
    );
    assert_eq!(event.kind.as_u16(), 50103);
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["t".to_string(), "budget_exhausted".to_string()]));
    assert!(tags(&event)
        .iter()
        .any(|t| t == &["i".to_string(), ID.to_string()]));
}

#[test]
fn tally_is_50103_tally_filter() {
    let (_, me) = keys_and_me();
    let f = filter(plan(&parse_org(&["org", "tally"]), &me).unwrap());
    assert_eq!(f["kinds"], json!([50103]));
    assert_eq!(f["#t"], json!(["tally"]));
}

// ── contract ────────────────────────────────────────────────────────────────

#[test]
fn no_verb_emits_relay_only_or_bridge_kinds() {
    let (keys, me) = keys_and_me();
    let writes = [
        parse_org(&["org", "bootstrap"]),
        parse_org(&["org", "shapers", "add", PK]),
        parse_org(&["org", "shapers", "remove", PK]),
        parse_org(&["org", "shapers", "rules", r#"{"rules":{}}"#]),
        parse_org(&["org", "shapers", "agent"]),
        parse_org(&["org", "shapers", "accept", ID]),
        parse_org(&["org", "shapers", "step-down"]),
        parse_org(&[
            "org",
            "direction",
            "propose",
            "vision",
            "--base",
            "0",
            "--body",
            "x",
        ]),
        parse_org(&["org", "proposals", "vote", ID, "agree"]),
        parse_org(&[
            "org",
            "proposals",
            "propose-project",
            "--title",
            "t",
            "--brief",
            "b",
            "--due-at",
            "1",
        ]),
        parse_org(&["org", "proposals", "propose-dri", ID, PK]),
        parse_org(&[
            "org", "work", "create", ID, "--title", "t", "--brief", "b", "--due-at", "1",
        ]),
        parse_org(&["org", "work", "offer", ID, PK]),
        parse_org(&["org", "work", "accept", ID]),
        parse_org(&["org", "work", "decline", ID]),
        parse_org(&["org", "work", "done", ID]),
        parse_org(&["org", "work", "release", ID]),
        parse_org(&["org", "work", "set-due", ID, "--due", "1"]),
        parse_org(&["org", "work", "reopen", ID]),
        parse_org(&["org", "drafts", "decide", EV, "accept"]),
        parse_org(&["org", "health", "rate", ID, "2026-W38", "healthy"]),
        parse_org(&["org", "profile", "set", "--about", "x"]),
        parse_org(&[
            "org",
            "ledger",
            "note",
            r#"{"note":"budget_exhausted","budget":"calls_per_hour","until":1}"#,
        ]),
    ];
    for cmd in writes {
        let event = sign(plan(&cmd, &me).unwrap(), &keys);
        let kind = event.kind.as_u16() as u32;
        assert!(
            !(39100..=39105).contains(&kind),
            "verb emitted relay-only kind {kind}"
        );
        assert_ne!(kind, 50014, "verb emitted bridge-only 50014");
    }
}

#[test]
fn compact_event_format_remains_the_three_key_contract() {
    let normalized = json!([{
        "id": "aa",
        "pubkey": "bb",
        "kind": 39103,
        "content": "compact content",
        "created_at": 1,
        "tags": [],
        "sig": "cc",
    }])
    .to_string();
    let compact: Value = serde_json::from_str(&super::format_events(
        &normalized,
        &crate::OutputFormat::Compact,
    ))
    .unwrap();
    assert_eq!(
        compact,
        json!([{
            "id": "aa",
            "content": "compact content",
            "created_at": 1,
        }])
    );
}

#[test]
fn progress_note_dispatch_error_is_other() {
    // The fixed message is what dispatch returns; plan is the seam.
    let err = CliError::Other(PROGRESS_NOT_IMPLEMENTED.to_string());
    assert_eq!(err.to_string(), "not implemented");
    assert_eq!(crate::error::exit_code(&err), 4);
}
