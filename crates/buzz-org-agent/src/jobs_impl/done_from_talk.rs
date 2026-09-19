//! Protocol §5.5 `io_done` from talk. Feature-gated; the only module that
//! may construct kind `50009`.

use nostr::{Event, Keys, Tag};

use crate::config::Config;
use crate::relay::publish::{sign_done_from_talk, PublishError};

/// Build a `50009` when `IO_DONE_FROM_TALK_ENABLED` is set. The §5.5
/// check-list is this module's test suite (A-2+); A-1 only owns the
/// chokepoint.
pub fn relay(keys: &Keys, cfg: &Config, item: &str, receipt: &str) -> Result<Event, PublishError> {
    if !cfg.done_from_talk {
        return Err(PublishError::DoneNotPermitted);
    }
    let tags = vec![
        Tag::parse(["i", item]).map_err(|e| PublishError::Sign(e.to_string()))?,
        Tag::parse(["e", receipt]).map_err(|e| PublishError::Sign(e.to_string()))?,
    ];
    sign_done_from_talk(keys, "{}", tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    fn cfg(enabled: bool) -> Config {
        Config {
            relay_url: None,
            private_key: None,
            auth_tag: None,
            state_dir: PathBuf::from("."),
            timezone: "UTC".into(),
            language: "en".into(),
            model_draft: None,
            model_fast: None,
            hear_enabled: false,
            done_from_talk: enabled,
            max_concurrent_think: 2,
        }
    }

    #[test]
    fn disabled_flag_refuses_to_sign() {
        let keys = Keys::generate();
        let err = relay(&keys, &cfg(false), "item", "aa".repeat(32).as_str()).expect_err("flag");
        assert!(matches!(err, PublishError::DoneNotPermitted));
    }

    #[test]
    fn enabled_flag_signs_kind_50009() {
        let keys = Keys::generate();
        let event = relay(
            &keys,
            &cfg(true),
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            &"ab".repeat(32),
        )
        .expect("sign");
        assert_eq!(u32::from(event.kind.as_u16()), 50009);
    }
}
