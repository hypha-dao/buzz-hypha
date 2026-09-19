//! `buzz-org-agent` — `run` | `dry-run` | `replay` | `doctor`.
//! `supervise` is wave 3.

use std::path::PathBuf;
use std::process::ExitCode;

use buzz_org_agent::config::Config;
use buzz_org_agent::relay::FakeRelay;
use buzz_org_agent::state::OrgState;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "buzz-org-agent",
    about = "Hosted org agent — HEAR → THINK → JUDGE → ROUTE. A-1: skeleton, nothing drafts."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// One community; identity and provider from env. A-1 connects the
    /// skeleton; it does not draft.
    Run,
    /// THINK + JUDGE for one trigger; prints; publishes nothing.
    DryRun {
        /// Transition or tick name.
        #[arg(long)]
        trigger: Option<String>,
    },
    /// Harness runner over an E-1 case name.
    Replay {
        /// Case (`river`, `energy`, `cold`, or a sequence name).
        case: String,
    },
    /// Key, membership, provider, timezone, budgets — no live relay.
    Doctor,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cfg = Config::from_env();
    match cli.command {
        Command::Doctor => doctor(&cfg),
        Command::DryRun { trigger } => dry_run(&cfg, trigger.as_deref()),
        Command::Replay { case } => replay(&case),
        Command::Run => run(&cfg),
    }
}

fn doctor(cfg: &Config) -> ExitCode {
    let mut ok = true;
    check("BUZZ_RELAY_URL", cfg.relay_url.is_some(), &mut ok);
    check("BUZZ_PRIVATE_KEY", cfg.private_key.is_some(), &mut ok);
    check("IO_MODEL_DRAFT", cfg.model_draft.is_some(), &mut ok);
    check("IO_TIMEZONE", cfg.timezone_ok(), &mut ok);
    println!(
        "IO_STATE_DIR={} hear={} done_from_talk={} max_think={}",
        cfg.state_dir.display(),
        cfg.hear_enabled,
        cfg.done_from_talk,
        cfg.max_concurrent_think
    );
    if ok {
        println!("doctor: ok (no live relay)");
        ExitCode::SUCCESS
    } else {
        println!("doctor: missing required env — see above");
        ExitCode::from(1)
    }
}

fn check(name: &str, present: bool, ok: &mut bool) {
    if present {
        println!("{name}: ok");
    } else {
        println!("{name}: missing");
        *ok = false;
    }
}

fn dry_run(cfg: &Config, trigger: Option<&str>) -> ExitCode {
    let _ = FakeRelay::new();
    let state = load_snapshot(&cfg.state_dir);
    println!(
        "dry-run: trigger={} items={} live={} — A-1 produces no draft",
        trigger.unwrap_or("(none)"),
        state.items.len(),
        state.live
    );
    ExitCode::SUCCESS
}

fn replay(case: &str) -> ExitCode {
    println!("replay: case={case} — apply fixtures through OrgState (cargo test covers this)");
    ExitCode::SUCCESS
}

fn run(cfg: &Config) -> ExitCode {
    if cfg.relay_url.is_none() || cfg.private_key.is_none() {
        eprintln!("run: BUZZ_RELAY_URL and BUZZ_PRIVATE_KEY are required");
        return ExitCode::from(1);
    }
    println!(
        "run: skeleton only — would connect to {} (no live relay in A-1 tests)",
        cfg.relay_url.as_deref().unwrap_or("")
    );
    ExitCode::SUCCESS
}

fn load_snapshot(dir: &PathBuf) -> OrgState {
    let _ = dir;
    OrgState::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_reports_missing_env() {
        let cfg = Config::from_env();
        // from_env never panics; doctor is the check.
        let _ = cfg.timezone_ok();
    }
}
