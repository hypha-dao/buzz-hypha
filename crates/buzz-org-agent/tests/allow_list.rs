//! The `Permitted` kinds chokepoint: kind `50009` is not constructible
//! outside `jobs_impl::done_from_talk`.

use std::fs;
use std::path::Path;

use buzz_org_agent::config::Config;
use buzz_org_agent::jobs_impl::done_from_talk;
use buzz_org_agent::relay::publish::{sign, Permitted, PublishError};
use nostr::Keys;

fn src_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn walk_rs(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).expect("src") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            walk_rs(&path, out);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read");
        let rel = path
            .strip_prefix(src_dir())
            .expect("rel")
            .to_string_lossy()
            .into_owned();
        out.push((rel, text));
    }
}

#[test]
fn permitted_kinds_exclude_50009() {
    assert!(!Permitted::all_kinds().contains(&50009));
    let keys = Keys::generate();
    for kind in [
        Permitted::Draft,
        Permitted::Health,
        Permitted::AgentNote,
        Permitted::Chat,
        Permitted::DmOpen,
        Permitted::Engram,
        Permitted::Auth,
        Permitted::Profile,
    ] {
        let event = sign(&keys, kind, "{}", vec![]).expect("sign");
        assert_ne!(u32::from(event.kind.as_u16()), 50009);
    }
}

#[test]
fn kind_50009_is_refused_outside_done_from_talk() {
    let keys = Keys::generate();
    let cfg = Config {
        relay_url: None,
        private_key: None,
        auth_tag: None,
        state_dir: ".".into(),
        timezone: "UTC".into(),
        language: "en".into(),
        model_draft: None,
        model_fast: None,
        hear_enabled: false,
        done_from_talk: false,
        max_concurrent_think: 2,
    };
    let err = done_from_talk::relay(&keys, &cfg, "item", &"aa".repeat(32)).expect_err("flag");
    assert!(matches!(err, PublishError::DoneNotPermitted));
}

#[test]
fn only_done_from_talk_and_publish_construct_kind_50009() {
    let mut files = Vec::new();
    walk_rs(&src_dir(), &mut files);
    let mut builders = Vec::new();
    for (rel, text) in &files {
        for (i, line) in text.lines().enumerate() {
            let mentions_builder = line.contains("EventBuilder")
                && (line.contains("KIND_IO_DONE") || line.contains("50009"));
            let mentions_sign_done = line.contains("Kind::Custom(KIND_IO_DONE");
            if mentions_builder || mentions_sign_done {
                builders.push(format!("{rel}:{}", i + 1));
            }
        }
    }
    assert_eq!(
        builders.len(),
        1,
        "one EventBuilder for 50009, got {builders:?}"
    );
    assert!(
        builders[0].starts_with("relay/publish.rs:"),
        "50009 EventBuilder must live only in sign_done_from_talk, got {builders:?}"
    );
}
