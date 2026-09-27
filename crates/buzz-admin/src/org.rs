//! Operator hosted-org-agent helpers (intelligent-org O-1).
//!
//! Writes `io_hosted_agents` and the agent's `kind:0` profile. NIP-43 add
//! stays on `buzz-admin add-member` (kind:13534 roster — the operator path
//! V4 already named; not a client `kind:9030`).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use anyhow::Result;
use buzz_db::event::EventQuery;
use buzz_db::intelligent_org::HostedAgentSet;
use clap::Subcommand;
use nostr::{EventBuilder, Keys, Kind};

use crate::{connect_db, parse_pubkey_hex, resolve_admin_tenant_for_host};

/// The `kind:0` `name` Design § Where it runs / Org agent § 15.2 require.
const ORG_AGENT_NAME: &str = "Org agent";

const KIND0_CONTENT: &str = r#"{"name":"Org agent"}"#;

#[derive(Subcommand)]
pub enum HostedAgentCommand {
    /// Mint (or reuse) a keypair in the operator store. Prints only the pubkey.
    Mint {
        /// Directory that will hold `secret` (0600) and `pubkey`.
        #[arg(long)]
        store: PathBuf,
    },
    /// Record `community → pubkey` in `io_hosted_agents`. Idempotent for the same key.
    Set {
        #[arg(long)]
        pubkey: String,
        /// Optional budget JSON (Protocol §6.2).
        #[arg(long)]
        budget: Option<String>,
        /// Community host (`communities.host`). Defaults to the RELAY_URL authority.
        #[arg(long)]
        host: Option<String>,
    },
    /// Print the live hosted pubkey hex, or nothing if none.
    Get {
        #[arg(long)]
        host: Option<String>,
    },
    /// Publish `kind:0` `{"name":"Org agent"}` from a secret file. Never prints the secret.
    PublishProfile {
        #[arg(long)]
        secret_file: PathBuf,
        #[arg(long)]
        host: Option<String>,
    },
}

pub async fn run(command: HostedAgentCommand) -> Result<i32> {
    match command {
        HostedAgentCommand::Mint { store } => cmd_mint(&store),
        HostedAgentCommand::Set {
            pubkey,
            budget,
            host,
        } => cmd_set(pubkey, budget, host.as_deref()).await,
        HostedAgentCommand::Get { host } => cmd_get(host.as_deref()).await,
        HostedAgentCommand::PublishProfile { secret_file, host } => {
            cmd_publish_profile(&secret_file, host.as_deref()).await
        }
    }
}

fn cmd_mint(store: &Path) -> Result<i32> {
    match mint_to_store(store) {
        Ok(pubkey) => {
            println!("{pubkey}");
            Ok(0)
        }
        Err(e) => {
            eprintln!("error: {e}");
            Ok(5)
        }
    }
}

/// Write (or reuse) `store/secret` and `store/pubkey`. Returns the pubkey hex.
pub(crate) fn mint_to_store(store: &Path) -> Result<String> {
    fs::create_dir_all(store)?;
    let secret_path = store.join("secret");
    let pubkey_path = store.join("pubkey");
    let keys = if secret_path.exists() {
        load_secret_keys(&secret_path)?
    } else {
        let keys = Keys::generate();
        let secret = keys.secret_key().display_secret().to_string();
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&secret_path)?;
        file.write_all(secret.as_bytes())?;
        file.sync_all()?;
        keys
    };
    let pubkey = keys.public_key().to_hex();
    fs::write(&pubkey_path, format!("{pubkey}\n"))?;
    Ok(pubkey)
}

fn load_secret_keys(path: &Path) -> Result<Keys> {
    let raw = fs::read_to_string(path)?;
    Keys::parse(raw.trim()).map_err(|e| {
        anyhow::anyhow!(
            "invalid secret in {} (file not printed): {e}",
            path.display()
        )
    })
}

async fn cmd_set(pubkey_arg: String, budget: Option<String>, host: Option<&str>) -> Result<i32> {
    let pubkey_hex = match parse_pubkey_hex(&pubkey_arg) {
        Ok(h) => h,
        Err(msg) => {
            eprintln!("error: {msg}");
            return Ok(1);
        }
    };
    let budget = match budget {
        Some(raw) => match serde_json::from_str(&raw) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("error: invalid --budget JSON: {e}");
                return Ok(1);
            }
        },
        None => None,
    };
    let db = connect_db().await?;
    let tenant = resolve_admin_tenant_for_host(&db, host).await?;
    match db
        .set_hosted_agent(tenant.community(), &pubkey_hex, budget)
        .await
    {
        Ok(HostedAgentSet::Inserted) => {
            println!("hosted-agent set {pubkey_hex}");
            Ok(0)
        }
        Ok(HostedAgentSet::AlreadySet) => {
            println!("already set: {pubkey_hex}");
            Ok(0)
        }
        Err(e) => {
            eprintln!("error: {e}");
            Ok(5)
        }
    }
}

async fn cmd_get(host: Option<&str>) -> Result<i32> {
    let db = connect_db().await?;
    let tenant = resolve_admin_tenant_for_host(&db, host).await?;
    if let Some(pubkey) = db.live_hosted_agent_pubkey(tenant.community()).await? {
        println!("{pubkey}");
    }
    Ok(0)
}

async fn cmd_publish_profile(secret_file: &Path, host: Option<&str>) -> Result<i32> {
    let keys = match load_secret_keys(secret_file) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(1);
        }
    };
    let db = connect_db().await?;
    let tenant = resolve_admin_tenant_for_host(&db, host).await?;
    let author = keys.public_key().to_bytes().to_vec();
    let existing = db
        .query_events(&EventQuery {
            kinds: Some(vec![0]),
            authors: Some(vec![author]),
            limit: Some(1),
            global_only: true,
            ..EventQuery::for_community(tenant.community())
        })
        .await?;
    if existing
        .first()
        .is_some_and(|ev| profile_name_is_org_agent(&ev.event.content))
    {
        println!("already published");
        return Ok(0);
    }
    let event = EventBuilder::new(Kind::Metadata, KIND0_CONTENT)
        .sign_with_keys(&keys)
        .map_err(|e| anyhow::anyhow!("sign kind:0: {e}"))?;
    let (_stored, inserted) = db
        .replace_addressable_event(tenant.community(), &event, None)
        .await?;
    if inserted {
        println!("published kind:0 name={ORG_AGENT_NAME}");
    } else {
        println!("already published");
    }
    Ok(0)
}

fn profile_name_is_org_agent(content: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s == ORG_AGENT_NAME)
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind0_name_is_org_agent() {
        assert!(profile_name_is_org_agent(KIND0_CONTENT));
        assert!(profile_name_is_org_agent(
            r#"{"name":"Org agent","about":"x"}"#
        ));
        assert!(!profile_name_is_org_agent(r#"{"name":"Other"}"#));
        assert!(!profile_name_is_org_agent("not json"));
    }

    #[test]
    fn mint_reuses_the_same_key_and_does_not_equal_the_secret() {
        let dir = std::env::temp_dir().join(format!(
            "o1-mint-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp store");
        let first = mint_to_store(&dir).expect("mint");
        let second = mint_to_store(&dir).expect("reuse");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        let secret = fs::read_to_string(dir.join("secret")).expect("secret");
        assert_ne!(first, secret.trim());
        let mode = fs::metadata(dir.join("secret"))
            .expect("meta")
            .permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(mode.mode() & 0o777, 0o600);
        }
        fs::remove_dir_all(&dir).ok();
    }
}
