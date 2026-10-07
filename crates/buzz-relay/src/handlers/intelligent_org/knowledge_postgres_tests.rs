//! `50024` opens a codebases proposal. Passing writes `39106` and leaves
//! direction untouched.

use buzz_core::kind::{KIND_IO_DIRECTION, KIND_IO_KNOWLEDGE, KIND_IO_KNOWLEDGE_SET, KIND_IO_VOTE};
use nostr::Keys;
use serde_json::Value;

use super::postgres_tests::{harness, rejected, signed, tag};

fn set(keys: &Keys, content: &str) -> nostr::Event {
    signed(keys, KIND_IO_KNOWLEDGE_SET, vec![], content)
}

fn reply(message: &str) -> Value {
    serde_json::from_str(message).expect("reply is json")
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_shaper_opens_a_codebases_proposal_and_passing_writes_the_list() {
    let h = harness().await;
    h.bootstrap().await;
    let content = r#"{"slug":"codebases","items":[
        {"kind":"repository","name":"buzz-hypha","url":"https://github.com/hypha-dao/buzz-hypha","about":"The product"},
        {"kind":"site","name":"Hypha","url":"https://hypha.earth","about":"Public site"}
    ]}"#;
    let opened = reply(
        &h.ingest(&h.owner, set(&h.owner, content))
            .await
            .expect("open codebases"),
    );
    assert_eq!(opened["status"], "open");
    let proposal = opened["proposal"].as_str().expect("proposal id");
    assert!(
        h.live_state(KIND_IO_KNOWLEDGE).await.is_empty(),
        "the list waits for the proposal to pass"
    );
    assert!(h.live_state(KIND_IO_DIRECTION).await.is_empty());

    let passed = reply(
        &h.send(
            &h.owner,
            KIND_IO_VOTE,
            vec![tag(["e", proposal]), tag(["vote", "agree"])],
            "{}",
        )
        .await
        .expect("agree"),
    );
    assert_eq!(passed["status"], "passed");
    let live = h.live_state(KIND_IO_KNOWLEDGE).await;
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].1["slug"], "codebases");
    assert_eq!(live[0].1["version"], 1);
    assert_eq!(
        live[0].1["items"][0]["url"],
        "https://github.com/hypha-dao/buzz-hypha"
    );
    assert_eq!(live[0].1["items"][1]["kind"], "site");
    assert!(h.live_state(KIND_IO_DIRECTION).await.is_empty());

    let other = Keys::generate();
    h.member(&other).await;
    assert_eq!(
        rejected(h.ingest(&other, set(&other, content)).await),
        "restricted: not a Shaper"
    );
    assert_eq!(h.live_state(KIND_IO_KNOWLEDGE).await[0].1["version"], 1);

    let replaced = r#"{"slug":"codebases","items":[
        {"kind":"repository","name":"none","url":"","about":"no repository yet"}
    ]}"#;
    let again = reply(
        &h.ingest(&h.owner, set(&h.owner, replaced))
            .await
            .expect("open the replacement"),
    );
    assert_eq!(again["status"], "open");
    assert_eq!(h.live_state(KIND_IO_KNOWLEDGE).await[0].1["version"], 1);
    let next_id = again["proposal"].as_str().expect("second proposal");
    let next = reply(
        &h.send(
            &h.owner,
            KIND_IO_VOTE,
            vec![tag(["e", next_id]), tag(["vote", "agree"])],
            "{}",
        )
        .await
        .expect("pass the replacement"),
    );
    assert_eq!(next["status"], "passed");
    let stored = h.live_state(KIND_IO_KNOWLEDGE).await;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].1["version"], 2);
    assert_eq!(stored[0].1["items"][0]["about"], "no repository yet");
    assert!(h.live_state(KIND_IO_DIRECTION).await.is_empty());

    let bad = r#"{"slug":"strategy","items":[{"kind":"repository","name":"x","url":"https://example.com","about":"no"}]}"#;
    assert!(rejected(h.ingest(&h.owner, set(&h.owner, bad)).await).contains("codebases"));
    assert_eq!(h.live_state(KIND_IO_KNOWLEDGE).await[0].1["version"], 2);
}
