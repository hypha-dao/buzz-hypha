//! Environment → [`Config`].

use std::env;
use std::path::PathBuf;

/// Runtime configuration. Flags have no switch for the judge or the
/// dedupe key (Org agent § 13).
#[derive(Debug, Clone)]
pub struct Config {
    /// `BUZZ_RELAY_URL`.
    pub relay_url: Option<String>,
    /// `BUZZ_PRIVATE_KEY` (hex).
    pub private_key: Option<String>,
    /// `BUZZ_AUTH_TAG`.
    pub auth_tag: Option<String>,
    /// `IO_STATE_DIR`.
    pub state_dir: PathBuf,
    /// `IO_TIMEZONE` (IANA). Default `UTC`.
    pub timezone: String,
    /// Community language. Default `en`.
    pub language: String,
    /// `IO_MODEL_DRAFT`.
    pub model_draft: Option<String>,
    /// `IO_MODEL_FAST`.
    pub model_fast: Option<String>,
    /// `IO_HEAR_ENABLED`. Phase 0 default is off.
    pub hear_enabled: bool,
    /// `IO_DONE_FROM_TALK_ENABLED`.
    pub done_from_talk: bool,
    /// `IO_MAX_CONCURRENT_THINK`. Default 2.
    pub max_concurrent_think: u32,
}

impl Config {
    /// Read the process environment. Missing values stay `None` / default;
    /// `doctor` reports them rather than refusing here.
    pub fn from_env() -> Self {
        let state_dir = env::var("IO_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".io-state"));
        Self {
            relay_url: env::var("BUZZ_RELAY_URL").ok(),
            private_key: env::var("BUZZ_PRIVATE_KEY").ok(),
            auth_tag: env::var("BUZZ_AUTH_TAG").ok(),
            state_dir,
            timezone: env::var("IO_TIMEZONE").unwrap_or_else(|_| "UTC".into()),
            language: env::var("IO_LANGUAGE").unwrap_or_else(|_| "en".into()),
            model_draft: env::var("IO_MODEL_DRAFT").ok(),
            model_fast: env::var("IO_MODEL_FAST").ok(),
            hear_enabled: truthy("IO_HEAR_ENABLED"),
            done_from_talk: truthy("IO_DONE_FROM_TALK_ENABLED"),
            max_concurrent_think: env::var("IO_MAX_CONCURRENT_THINK")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),
        }
    }

    /// IANA timezone parses.
    pub fn timezone_ok(&self) -> bool {
        self.timezone.parse::<chrono_tz::Tz>().is_ok()
    }
}

fn truthy(key: &str) -> bool {
    matches!(
        env::var(key).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}
