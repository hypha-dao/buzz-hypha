//! Durable JSONL outbox (Org agent § 4.4). Review-Proven rule 1.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use buzz_core::Event;
use serde::{Deserialize, Serialize};

use super::publish::PublishError;

/// One signed event waiting for `OK true`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxItem {
    /// The signed event.
    pub event: Event,
    /// Send attempts so far.
    pub attempts: u32,
    /// Unix seconds when it was appended.
    pub queued_at: u64,
}

/// JSONL file in `IO_STATE_DIR`.
#[derive(Debug, Clone)]
pub struct Outbox {
    path: PathBuf,
    items: Vec<OutboxItem>,
}

impl Outbox {
    /// Open (or create) `dir/outbox.jsonl`.
    pub fn open(dir: &Path) -> Result<Self, PublishError> {
        fs::create_dir_all(dir).map_err(|e| PublishError::Outbox(e.to_string()))?;
        let path = dir.join("outbox.jsonl");
        let items = if path.is_file() {
            let file = fs::File::open(&path).map_err(|e| PublishError::Outbox(e.to_string()))?;
            let mut items = Vec::new();
            for line in BufReader::new(file).lines() {
                let line = line.map_err(|e| PublishError::Outbox(e.to_string()))?;
                if line.trim().is_empty() {
                    continue;
                }
                items.push(
                    serde_json::from_str(&line).map_err(|e| PublishError::Outbox(e.to_string()))?,
                );
            }
            items
        } else {
            Vec::new()
        };
        Ok(Self { path, items })
    }

    /// Append a newly signed event (`attempts = 0`) and fsync.
    pub fn append(&mut self, event: Event, now: u64) -> Result<(), PublishError> {
        self.items.push(OutboxItem {
            event,
            attempts: 0,
            queued_at: now,
        });
        self.persist()
    }

    /// Pending items, in queue order.
    pub fn pending(&self) -> &[OutboxItem] {
        &self.items
    }

    /// Record a send attempt.
    pub fn bump_attempt(&mut self, event_id: &str) -> Result<(), PublishError> {
        if let Some(item) = self
            .items
            .iter_mut()
            .find(|i| i.event.id.to_hex() == event_id)
        {
            item.attempts = item.attempts.saturating_add(1);
        }
        self.persist()
    }

    /// Remove on `OK true`.
    pub fn ack(&mut self, event_id: &str) -> Result<(), PublishError> {
        self.items.retain(|i| i.event.id.to_hex() != event_id);
        self.persist()
    }

    /// Drop an expired or finally-rejected event.
    pub fn drop_id(&mut self, event_id: &str) -> Result<(), PublishError> {
        self.ack(event_id)
    }

    fn persist(&self) -> Result<(), PublishError> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| PublishError::Outbox(e.to_string()))?;
        for item in &self.items {
            let line =
                serde_json::to_string(item).map_err(|e| PublishError::Outbox(e.to_string()))?;
            writeln!(file, "{line}").map_err(|e| PublishError::Outbox(e.to_string()))?;
        }
        file.sync_all()
            .map_err(|e| PublishError::Outbox(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::publish::{sign, Permitted};
    use nostr::Keys;

    #[test]
    fn append_survives_reopen() {
        let dir = tempfile::tempdir().expect("tmp");
        let keys = Keys::generate();
        let event = sign(&keys, Permitted::AgentNote, "{}", vec![]).expect("sign");
        let id = event.id.to_hex();
        {
            let mut box_ = Outbox::open(dir.path()).expect("open");
            box_.append(event, 1).expect("append");
            assert_eq!(box_.pending().len(), 1);
        }
        let box_ = Outbox::open(dir.path()).expect("reopen");
        assert_eq!(box_.pending()[0].event.id.to_hex(), id);
    }
}
