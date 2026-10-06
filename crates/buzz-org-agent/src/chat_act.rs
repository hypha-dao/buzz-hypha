//! Chat marks the member's client turns into commands.
//!
//! The agent never signs `50002` / `50004` / `50005` / `50009` / `50015`.
//! It names the act on its kind 9 reply. The person it was talking to
//! signs, and nothing is real until the relay's rule is met.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

/// What the model is allowed to ask the member's client to sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawAct {
    /// `50004` project proposal.
    Project {
        title: String,
        brief: String,
        due_days: u32,
        who: Option<String>,
    },
    /// `50009` — only if the speaker holds the ticket.
    Done { item: String },
    /// `50005` — speaker holds the parent; `who` is offered the child.
    /// `due_days` is absent when the person did not name a date.
    Ticket {
        parent: String,
        title: String,
        brief: String,
        due_days: Option<u32>,
        who: String,
    },
    /// `50015` — work that has no holder.
    Dri { item: String, who: String },
    /// Take a project or ticket off the live board.
    Remove { item: String },
    /// Replace an open direction or project proposal and start the vote over.
    Revise {
        proposal: String,
        slug: Option<String>,
        body: Option<String>,
        title: Option<String>,
        brief: Option<String>,
        due_days: Option<u32>,
    },
    /// `50001` add or remove. A draft until they publish.
    Shapers {
        op: String,
        who: String,
        why: String,
    },
    /// `50001` rules. Missing kinds stay `majority` until the client merges
    /// them with the rules already in force.
    Rules {
        direction: String,
        project: String,
        dri: String,
        shapers: String,
        money: String,
        join: String,
        decision_days: Option<u32>,
        offer_days: Option<u32>,
    },
    /// `50001` agent. `who` absent means return to the hosted agent.
    Agent { who: Option<String>, why: String },
    /// `50021` — the speaker's own profile. `None` keeps that field.
    Profile {
        about: Option<String>,
        skills: Option<Vec<String>>,
        socials: Option<Vec<(String, String)>>,
    },
}

/// How a ticket's parent was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentChoice {
    /// They named the live title, or a short name that matches one item.
    Named,
    /// They described the work, and one item they hold fits those words.
    Described,
    /// They named no parent. This is the best fit among projects they hold.
    Suggested,
}

/// A mark ready to put on the chat event. Ids are already resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedAct {
    Project {
        title: String,
        brief: String,
        due_at: u64,
        suggested: Option<String>,
    },
    Done {
        item: String,
    },
    Ticket {
        parent: String,
        title: String,
        brief: String,
        due_at: u64,
        /// Absent when nobody was named and no profile fits. The ticket stays open.
        offer_to: Option<String>,
        /// How `parent` was chosen. A suggestion is the best fit they hold
        /// when they never named a project.
        parent_choice: ParentChoice,
    },
    Dri {
        item: String,
        title: String,
        who: String,
    },
    /// Direct `50022`. A sole Shaper's project, or a ticket.
    Remove {
        item: String,
        title: String,
    },
    /// `50023` — a project, and more than one Shaper is seated.
    RemoveProject {
        item: String,
        title: String,
    },
    /// `50002` with `revises` — same proposal, new text, votes cleared.
    ReviseDirection {
        proposal: String,
        slug: String,
        body: String,
        base: u32,
    },
    /// `50004` with `revises`.
    ReviseProject {
        proposal: String,
        title: String,
        brief: String,
        due_at: u64,
    },
    /// `50001` add or remove.
    Shapers {
        op: String,
        who: String,
        why: String,
    },
    /// `50001` rules. `rules_json` is the overlay the model named.
    Rules {
        rules_json: String,
        decision_window_secs: Option<u64>,
        offer_window_secs: Option<u64>,
    },
    /// `50001` agent. `pubkey` absent returns to the hosted agent.
    Agent {
        pubkey: Option<String>,
        why: String,
    },
    /// `50021` — the whole profile after merging what they just said.
    Profile {
        about: String,
        skills: Vec<String>,
        socials: Vec<(String, String)>,
        open_limit: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Item {
    id: String,
    title: String,
    parent: Option<String>,
    state: String,
    dri: Option<String>,
    created_by: Option<String>,
    /// Last member who offered this item. Sticky across later clears.
    offered_by: Option<String>,
    created_from: Option<String>,
    created_at: u64,
    /// Project review date, or a ticket's due date, when the live item has one.
    due_at: Option<u64>,
    /// Markdown brief. Empty when the live item has none.
    brief: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectionHead {
    slug: String,
    version: u32,
    body: String,
    created_at: u64,
}

#[derive(Debug, Clone)]
struct OpenProposal {
    id: String,
    kind: String,
    slug: Option<String>,
    text: String,
    base: Option<u32>,
    created_at: u64,
}

/// One member's org profile, as the agent last saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PersonProfile {
    about: String,
    skills: Vec<String>,
    socials: Vec<(String, String)>,
    open_limit: Option<u32>,
    at: u64,
}

/// Live org the model can answer from, and that act resolution checks.
#[derive(Debug, Clone, Default)]
pub struct Board {
    shapers: Vec<String>,
    room: Option<String>,
    names: HashMap<String, (u64, String)>,
    items: HashMap<String, Item>,
    direction: HashMap<String, DirectionHead>,
    proposals: HashMap<String, OpenProposal>,
    shapers_at: u64,
    /// `50005` event id → author, so a ticket's creator is known before
    /// `created_by` was written on the item.
    command_authors: HashMap<String, String>,
    /// Newest `39105` per member.
    profiles: HashMap<String, PersonProfile>,
}

impl Board {
    /// Fold one relay event into the newest head per coordinate.
    pub fn observe(
        &mut self,
        kind: u32,
        author: &str,
        created_at: u64,
        d: Option<&str>,
        content: &str,
    ) {
        let value: Value = serde_json::from_str(content).unwrap_or(Value::Null);
        match kind {
            0 => {
                // Buzz keeps the visible name in `display_name`. `name` is
                // the older handle, used only when no display name is set.
                let Some(name) =
                    text_field(&value, "display_name").or_else(|| text_field(&value, "name"))
                else {
                    return;
                };
                let key = author.trim().to_ascii_lowercase();
                let replace = self
                    .names
                    .get(&key)
                    .is_none_or(|current| created_at >= current.0);
                if replace {
                    self.names.insert(key, (created_at, name));
                }
            }
            39100 => {
                let Some(slug) = d.map(str::trim).filter(|slug| !slug.is_empty()) else {
                    return;
                };
                let body = text_field(&value, "body").unwrap_or_default();
                let version = value.get("version").and_then(Value::as_u64).unwrap_or(0) as u32;
                let replace = self
                    .direction
                    .get(slug)
                    .is_none_or(|current| created_at >= current.created_at);
                if replace {
                    self.direction.insert(
                        slug.to_string(),
                        DirectionHead {
                            slug: slug.to_string(),
                            version,
                            body,
                            created_at,
                        },
                    );
                }
            }
            39101 => {
                let id = d
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                    .map(str::to_string)
                    .or_else(|| text_field(&value, "id"));
                let Some(id) = id else { return };
                let title = text_field(&value, "title").unwrap_or_else(|| id.clone());
                let parent = text_field(&value, "parent");
                let state = text_field(&value, "state").unwrap_or_else(|| "open".into());
                let dri = text_field(&value, "dri").map(|pubkey| pubkey.to_ascii_lowercase());
                let previous = self.items.get(&id).map(|item| {
                    (
                        item.created_at,
                        item.created_by.clone(),
                        item.offered_by.clone(),
                    )
                });
                let created_from = text_field(&value, "created_from");
                let due_at = value.get("due_at").and_then(Value::as_u64);
                let brief = text_field(&value, "brief").unwrap_or_default();
                let created_by = text_field(&value, "created_by")
                    .map(|pubkey| pubkey.to_ascii_lowercase())
                    .or_else(|| {
                        previous
                            .as_ref()
                            .and_then(|(_, created, _)| created.clone())
                    })
                    .or_else(|| {
                        created_from
                            .as_ref()
                            .and_then(|event_id| self.command_authors.get(event_id).cloned())
                    });
                let fresh_offerer = text_field(&value, "offered_by_member")
                    .or_else(|| text_field(&value, "offered_by"))
                    .filter(|pubkey| is_member_pubkey(pubkey))
                    .map(|pubkey| pubkey.to_ascii_lowercase());
                let offered_by = fresh_offerer.or_else(|| {
                    previous
                        .as_ref()
                        .and_then(|(_, _, offered)| offered.clone())
                });
                let replace = previous.as_ref().is_none_or(|(at, _, _)| created_at >= *at);
                if replace {
                    self.items.insert(
                        id.clone(),
                        Item {
                            id,
                            title,
                            parent,
                            state,
                            dri,
                            created_by,
                            offered_by,
                            created_from,
                            created_at,
                            due_at,
                            brief,
                        },
                    );
                }
            }
            39103 => {
                if created_at < self.shapers_at && self.shapers_at > 0 {
                    return;
                }
                self.shapers_at = created_at;
                self.shapers = value
                    .get("shapers")
                    .and_then(Value::as_array)
                    .map(|rows| {
                        rows.iter()
                            .filter_map(Value::as_str)
                            .map(|pubkey| pubkey.trim().to_ascii_lowercase())
                            .filter(|pubkey| !pubkey.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();
                self.room = text_field(&value, "room");
            }
            39102 => self.observe_proposal(d, created_at, &value),
            39105 => self.observe_profile(d, created_at, &value),
            _ => {}
        }
    }

    fn observe_profile(&mut self, d: Option<&str>, created_at: u64, value: &Value) {
        let Some(pubkey) = d
            .map(str::trim)
            .filter(|pubkey| is_member_pubkey(pubkey))
            .map(|pubkey| pubkey.to_ascii_lowercase())
        else {
            return;
        };
        let replace = self
            .profiles
            .get(&pubkey)
            .is_none_or(|current| created_at >= current.at);
        if !replace {
            return;
        }
        let skills = value
            .get("skills")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| {
                        row.as_str()
                            .map(str::to_string)
                            .or_else(|| text_field(row, "label"))
                            .or_else(|| text_field(row, "slug"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let socials = value
            .get("socials")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| {
                        let network = text_field(row, "network")?;
                        let url = text_field(row, "url")?;
                        Some((network, url))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let open_limit = value
            .get("open_limit")
            .and_then(Value::as_u64)
            .and_then(|limit| u32::try_from(limit).ok())
            .filter(|limit| (1..=50).contains(limit));
        self.profiles.insert(
            pubkey,
            PersonProfile {
                about: text_field(value, "about").unwrap_or_default(),
                skills,
                socials,
                open_limit,
                at: created_at,
            },
        );
    }

    fn observe_proposal(&mut self, d: Option<&str>, created_at: u64, value: &Value) {
        let id = d
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .or_else(|| text_field(value, "id"));
        let Some(id) = id else { return };
        let status = text_field(value, "status").unwrap_or_default();
        let kind = text_field(value, "kind").unwrap_or_default();
        if status != "open" || (kind != "direction" && kind != "project") {
            self.proposals.remove(&id);
            return;
        }
        let payload = value.get("payload").cloned().unwrap_or(Value::Null);
        let text = if kind == "direction" {
            text_field(&payload, "body").unwrap_or_default()
        } else {
            let title = text_field(&payload, "title").unwrap_or_default();
            let brief = text_field(&payload, "brief").unwrap_or_default();
            format!("{title}. {brief}")
        };
        let replace = self
            .proposals
            .get(&id)
            .is_none_or(|current| created_at >= current.created_at);
        if !replace {
            return;
        }
        self.proposals.insert(
            id.clone(),
            OpenProposal {
                id,
                kind,
                slug: text_field(&payload, "slug"),
                text,
                base: payload
                    .get("base")
                    .and_then(Value::as_u64)
                    .map(|n| n as u32),
                created_at,
            },
        );
    }

    /// Remember a ticket-create or offer command so removal can name the
    /// creator and the member who offered it.
    pub fn note_command(&mut self, kind: u32, event_id: &str, author: &str, item: Option<&str>) {
        let author = author.trim().to_ascii_lowercase();
        if author.is_empty() {
            return;
        }
        let event_id = event_id.trim().to_ascii_lowercase();
        if kind == 50005 && !event_id.is_empty() {
            self.command_authors
                .insert(event_id.clone(), author.clone());
            for found in self.items.values_mut() {
                if found.created_by.is_none()
                    && found.created_from.as_deref() == Some(event_id.as_str())
                {
                    found.created_by = Some(author.clone());
                }
            }
        }
        if kind == 50006 {
            if let Some(id) = item.map(str::trim).filter(|id| !id.is_empty()) {
                if let Some(found) = self.items.get_mut(id) {
                    found.offered_by = Some(author);
                }
            }
        }
    }

    /// `#shapers` channel id once bootstrap has written `39103.room`.
    /// The agent hears that room whether one Shaper sits or several.
    pub fn shapers_room(&self) -> Option<&str> {
        if self.shapers.is_empty() {
            return None;
        }
        self.room.as_deref().filter(|id| !id.is_empty())
    }

    /// More than one Shaper is seated, so `#shapers` is a group conversation.
    pub fn several_shapers(&self) -> bool {
        self.shapers.len() > 1
    }

    /// Compact overview for the model. Missing pieces stay "not set".
    pub fn overview(&self) -> String {
        let mut lines = Vec::new();
        if self.shapers.is_empty() {
            lines.push("Shapers: not bootstrapped yet.".to_string());
        } else {
            let names: Vec<String> = self
                .shapers
                .iter()
                .map(|pubkey| self.label(pubkey))
                .collect();
            lines.push(format!(
                "Shapers ({}): {}.",
                self.shapers.len(),
                names.join(", ")
            ));
        }
        lines.push("Direction:".to_string());
        for slug in ["mission", "vision", "situation", "objectives", "strategy"] {
            match self.direction.get(slug) {
                Some(head) if !head.body.trim().is_empty() => {
                    lines.push(format!("- {slug} v{}: {}", head.version, head.body.trim()));
                }
                _ => lines.push(format!("- {slug}: not set")),
            }
        }
        lines.push("Open proposals:".to_string());
        let mut open: Vec<&OpenProposal> = self.proposals.values().collect();
        open.sort_by(|left, right| left.kind.cmp(&right.kind).then(left.id.cmp(&right.id)));
        if open.is_empty() {
            lines.push("- none".to_string());
        }
        for proposal in open.iter().take(12) {
            let text = clip(&proposal.text, 180);
            if proposal.kind == "direction" {
                let slug = proposal.slug.as_deref().unwrap_or("direction");
                lines.push(format!("- direction {slug} id={}: {text}", proposal.id));
            } else {
                lines.push(format!("- project id={}: {text}", proposal.id));
            }
        }
        lines.push("Work:".to_string());
        let mut items: Vec<&Item> = self
            .items
            .values()
            .filter(|item| item.state != "withdrawn")
            .collect();
        items.sort_by(|left, right| left.title.cmp(&right.title));
        if items.is_empty() {
            lines.push("- none yet".to_string());
        }
        for item in items.iter().take(40) {
            let kind = if item.parent.is_none() {
                "project"
            } else {
                "ticket"
            };
            let about = {
                let brief = clip(item.brief.trim(), 80);
                if brief.is_empty() {
                    String::new()
                } else {
                    format!(" — {brief}")
                }
            };
            let holder = item
                .dri
                .as_deref()
                .map(|pubkey| format!(", holder {}", self.label(pubkey)))
                .unwrap_or_else(|| ", no holder".into());
            let parent = item
                .parent
                .as_deref()
                .and_then(|id| self.items.get(id))
                .map(|parent| format!(", under {}", parent.title))
                .unwrap_or_default();
            let mut rights = String::new();
            if item.parent.is_some() {
                if let Some(pubkey) = &item.created_by {
                    rights.push_str(&format!(", created by {}", self.label(pubkey)));
                }
                if let Some(pubkey) = &item.offered_by {
                    rights.push_str(&format!(", offered by {}", self.label(pubkey)));
                }
            }
            lines.push(format!(
                "- {}{about} ({kind}, {}{holder}{parent}{rights}) id={}",
                item.title, item.state, item.id
            ));
        }
        if items.len() > 40 {
            lines.push(format!("- and {} more", items.len() - 40));
        }
        if !self.names.is_empty() {
            let mut people: Vec<String> = self
                .names
                .iter()
                .map(|(pubkey, (_, name))| format!("{name} ({pubkey})"))
                .collect();
            people.sort();
            lines.push(format!("People: {}.", people.join("; ")));
        }
        lines.push("Profiles:".to_string());
        let mut profile_keys: Vec<String> = self.profiles.keys().cloned().collect();
        profile_keys.sort_by(|left, right| self.label(left).cmp(&self.label(right)));
        if profile_keys.is_empty() {
            lines.push("- none yet".to_string());
        }
        for pubkey in profile_keys.into_iter().take(30) {
            let label = self.label(&pubkey);
            let (about, wants, limit) = {
                let profile = &self.profiles[&pubkey];
                let wants = if profile.skills.is_empty() {
                    "none".to_string()
                } else {
                    profile.skills.join(", ")
                };
                let about = clip(profile.about.trim(), 120);
                let about = if about.is_empty() {
                    "no about".to_string()
                } else {
                    about
                };
                let limit = profile
                    .open_limit
                    .map(|cap| format!(", limit {cap}"))
                    .unwrap_or_default();
                (about, wants, limit)
            };
            let held = self.held_count(&pubkey);
            lines.push(format!(
                "- {label} ({pubkey}): {about}. Wants: {wants}. Holding {held}{limit}."
            ));
        }
        lines.join("\n")
    }

    fn label(&self, pubkey: &str) -> String {
        let key = pubkey.trim().to_ascii_lowercase();
        self.stored_name(&key).unwrap_or_else(|| {
            let short: String = key.chars().take(8).collect();
            short
        })
    }

    /// Pubkeys the chat may name, who still have no display name.
    /// Bounded so a names request stays small.
    pub fn unnamed_pubkeys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        let mut push = |pubkey: &str| {
            let key = pubkey.trim().to_ascii_lowercase();
            if key.len() == 64 && self.stored_name(&key).is_none() && !keys.contains(&key) {
                keys.push(key);
            }
        };
        for pubkey in &self.shapers {
            push(pubkey);
        }
        for pubkey in self.profiles.keys() {
            push(pubkey);
        }
        for item in self.items.values() {
            if let Some(pubkey) = &item.dri {
                push(pubkey);
            }
            if let Some(pubkey) = &item.created_by {
                push(pubkey);
            }
            if let Some(pubkey) = &item.offered_by {
                push(pubkey);
            }
        }
        keys.sort();
        keys.truncate(40);
        keys
    }

    /// A display name from a profile. A missing name stays absent so a chat
    /// line does not invent one from the pubkey.
    fn stored_name(&self, pubkey: &str) -> Option<String> {
        let key = pubkey.trim().to_ascii_lowercase();
        self.names
            .get(&key)
            .map(|(_, name)| name.trim().to_string())
            .filter(|name| !name.is_empty())
    }

    fn held_count(&self, pubkey: &str) -> u32 {
        self.items
            .values()
            .filter(|item| {
                item.dri.as_deref() == Some(pubkey)
                    && (item.state == "accepted" || item.state == "in_review")
            })
            .count() as u32
    }

    /// Best fit from profiles and current load. `None` when nobody matches.
    fn suggest_holder(&self, title: &str, brief: &str) -> Option<String> {
        let people: Vec<crate::profile_fit::FitPerson> = self
            .profiles
            .iter()
            .map(|(pubkey, profile)| crate::profile_fit::FitPerson {
                pubkey: pubkey.clone(),
                about: profile.about.clone(),
                skills: profile.skills.clone(),
                open_limit: profile.open_limit,
                held: self.held_count(pubkey),
            })
            .collect();
        crate::profile_fit::suggest_holder(&people, title, brief)
    }

    fn stored_profile(&self, pubkey: &str) -> crate::profile_fit::StoredProfile {
        let found = self.profiles.get(pubkey);
        crate::profile_fit::StoredProfile {
            about: found
                .map(|profile| profile.about.clone())
                .unwrap_or_default(),
            skills: found
                .map(|profile| profile.skills.clone())
                .unwrap_or_default(),
            socials: found
                .map(|profile| {
                    profile
                        .socials
                        .iter()
                        .map(|(network, url)| buzz_core::intelligent_org::ProfileSocial {
                            network: network.clone(),
                            url: url.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            open_limit: found.and_then(|profile| profile.open_limit),
        }
    }
}

/// How the agent judges and helps write direction. Objectives wait for the
/// situation because it decides what the first objective can be.
/// "Sharper draft:" is the cue `recover_direction` accepts from a reply.
const DIRECTION_COACHING: &str = "\
    If they are shaping this alone, walk direction in this order, one piece at a time: \
    mission, vision, situation, objectives, then strategy. \
    The situation is where the org stands today. \
    It is its own confirmed text, the same as the other four. \
    Never say there is no situation draft, and never fold where the org stands into the objectives or the strategy. \
    When they ask to create or draft the situation, even as a question, that is the request. \
    Set direction to situation on that turn when the overview or the conversation already holds the stage and what exists. \
    body is one paragraph. If one fact only they know is still missing, ask that one question and leave direction null. \
    Objectives wait until you know it: the right objective for an untested idea is a test, \
    for a running service it is growth or repair. \
    One piece at a time means a piece is finished only when it is confirmed. \
    A reply that sets direction or offers a Sharper draft ends with that piece: \
    say what to do with the draft, open it and publish it, and never ask about the next piece in the same reply. \
    In #shapers with several Shapers, add that it then waits for their votes. \
    Start the next piece only when the overview shows this one with a version, or when they ask to move on. \
    When a note says a piece was just confirmed, say so in a few words and start the next piece in order, \
    with the one question that decides it most, or a draft when the conversation already holds enough. \
    Strategy is the last piece. When it is confirmed, do not draft any direction: \
    say the direction is set, and offer the first project that moves an objective. \
    A message that starts with Opened a and names a proposal is a member publishing a draft. \
    It asks nothing, so never draft from it, and never redraft the text it restates. \
    Ask one question per reply, never two, and about one thing: do not join two asks with and. \
    Never list or count the questions still to come. \
    Take the most decisive one first, \
    wait for the answer, then ask the next one, building on what they said. \
    When they answer, say in a few words what it settled, then ask the next question. \
    A coaching reply is at most three short sentences before its one question. \
    Do not explain the order of the pieces or why you ask unless they ask. \
    A short example answer may follow the question, so it is easy to reply. \
    Ask only what changes the text, and skip anything they or the overview already answered. \
    If they jump ahead, help with what they asked, then name the earlier piece that is missing and why it changes the answer. \
    Judge every mission, vision, situation, objective, or strategy they state, and any in the overview they ask about. \
    In one or two sentences say whether it is strong, close, or weak, and name exactly what is missing against the bar below. \
    Praise only what earns it. \
    When it is not strong, end with one better version written as the artifact itself, introduced as: Sharper draft: <the text>. \
    Build it only from what they said and what the overview shows. \
    When the missing piece is a fact only they know, such as who it is for, ask for it instead of guessing. \
    Their own words are still the draft on that turn. \
    When they take yours (yes, use yours, take it), set direction again with your version as body. \
    When they ask for help and give no text, ask the one question that decides it most. \
    As soon as their answers are enough, write the text yourself and set direction on that turn. \
    When a statement belongs to another text, say so and draft it there: \
    a who-for is mission, an end state is vision, an activity is a project, a refusal is strategy. \
    Mission bar: one sentence saying what we do, for whom, and where when a place bounds the work. \
    For whom names concrete people, never communities, users, or everyone. It should make clear what we are not. \
    Weak: a slogan, a value, or words like empower, leverage, or innovative that no project could be checked against. \
    Vision bar: a picture of the world once this has worked that a stranger could walk into and confirm, \
    two to four measures in words, and a horizon such as in three years. \
    It should be big enough that the obvious path will not reach it. \
    Weak: a slogan, a status report, or something nearly true already. \
    Situation: before the first objective, learn the org's stage: \
    only an idea, just started, running a service or product, growing, or in trouble. \
    Read the overview first and never ask what it already shows: the situation if one is set, \
    live and closed work, people, profiles, Shapers. \
    Then learn what it does not show, one question per reply, roughly in this order: \
    what exists today, such as the service, its users or customers, and partners; \
    what is proven and what is only assumed; what is working and what is stuck; \
    who does the work and how many hours they have; money in words, never balances; \
    any outside date such as a funding round, a season, or a partner's deadline; \
    and the one thing the org must learn next. Drop any of these an earlier answer already covered. \
    When you know enough, draft it: direction situation, body one plain paragraph of three to six sentences \
    covering the stage, what exists, what is proven and what is assumed, what works and what is stuck, \
    who does the work, money in words, outside dates, and what the org must learn next. \
    Once it is confirmed, propose objectives that fit it. \
    Situation bar: a stranger could tell the stage and the next unknown from it alone. \
    Weak: a plan, a hope, or a count that will be stale next week. \
    When a set situation no longer matches what they say or what the work shows, \
    say what changed and draft the new situation. \
    Objectives bar: three to seven lines. Each is one outcome, not an activity; checkable with yes or no on the date; \
    carries its date in the words, such as by March; \
    names where it starts today when that decides the first step, such as: a second site by December, none chosen yet; \
    gives the outside reason for the date when there is one; and does not overlap another line. \
    Fit the stage: an untested idea first proves someone wants it before anything is built; \
    a running service moves the one or two numbers that matter most; an org in trouble repairs first. \
    Push for fewer, harder lines. If every line matters equally, none does. Each line should move the vision. \
    Objectives and strategy grow a line at a time as you talk. Every draft of them carries the whole list: \
    each line of your latest draft card of it, shown under your earlier message, plus the new or changed line. \
    Leave a line out only when they ask to drop it. \
    The draft card shows the text under your reply, so say never repeats the drafted text or its lines: \
    name what changed in a few words, say to open and publish it, and ask at most one question. \
    Strategy bar: two to six bets on how the objectives will be reached, \
    each with why the org believes it and the assumption it rests on; \
    at least one refusal, what the org will not do; which objective wins when two compete; \
    how the work gets done, volunteers or paid, hours, partners; and the money posture in words. \
    Weak: values with no consequence, or a line that is really an objective. \
    Think from first principles, not from what similar orgs usually do. Ask why until you reach the real constraint. \
    Cut anything that moves no objective. Prefer the fastest cheap test to a long plan. \
    Be direct, specific, and brief. No jargon, no flattery, no pep talk. \
    Never write a live number, such as a balance or a member count, into direction.";

/// Prompt plus the live overview. `place` is the DM or `#shapers`.
pub fn system_prompt(place: &str, overview: &str) -> String {
    format!(
        "\
    You are Org. Agent for this community. You draft, people decide. \
    Nothing you draft is real until the right person agrees. \
    Work is offered, never assigned. \
    On direction you work like the best founder-CEO alive: first principles, candour, \
    ambition sized to the stage, and ruthless focus. \
    This conversation is {place}. \
    Reply as JSON only, no markdown fence: \
    {{\"say\":\"<the full reply, plain sentences>\",\"direction\":null,\"body\":null,\"act\":null}} \
    direction is \"mission\", \"vision\", \"situation\", \"objectives\", or \"strategy\" on the same turn \
    they name that artifact and give its text. body is that text, cleaned into the artifact sentence, \
    never their yes and never a question back to them. \
    The chat shows a draft as soon as they state it. They open it, edit it, and publish it. \
    Do not wait for them to approve the wording, and do not ask whether it captures what they have in mind. \
    Do not say the mission, vision, situation, objective, or strategy is published. Publishing is their tap on the draft. \
    If they say yes about a sentence you already wrote and that turn had no draft, body is that sentence, \
    never their yes. \
    {DIRECTION_COACHING} \
    Draft each one on the turn they state it. \
    objectives and strategy are lists: each stated point is its own line in body, \
    separated by a line break, with no numbers. \
    body holds only the lines being added. The published lines stay on the draft, where they can edit or drop any. \
    mission and vision stay a single sentence. situation is one paragraph with no line breaks. \
    act is null, or exactly one of: \
    {{\"kind\":\"project\",\"title\":\"...\",\"brief\":\"...\",\"due_days\":14,\"who\":null}} \
    {{\"kind\":\"done\",\"item\":\"<exact title from the overview>\"}} \
    {{\"kind\":\"ticket\",\"parent\":\"<title, a description, or empty>\",\"title\":\"...\",\"brief\":\"...\",\"who\":\"me\" or a person's name or null,\"due_days\":14}} \
    {{\"kind\":\"dri\",\"item\":\"<exact title>\",\"who\":\"me\" or a person's name}} \
    {{\"kind\":\"remove\",\"item\":\"<exact title from the overview>\"}} \
    {{\"kind\":\"revise\",\"proposal\":\"<id from Open proposals>\",\"slug\":\"mission\",\"body\":\"<the whole new text>\"}} \
    {{\"kind\":\"revise\",\"proposal\":\"<id from Open proposals>\",\"title\":\"...\",\"brief\":\"...\",\"due_days\":14}} \
    {{\"kind\":\"profile\",\"about\":null,\"skills\":null,\"socials\":null}} \
    When they ask to create, change, or open a proposal — a project, a direction, a DRI, a Shaper add or remove, the decision rules, which org agent to use, or a change to an open proposal — set direction or act on this turn. The chat shows a draft they open, edit, and publish. Do not say it is already up for a vote, and do not ask them to say yes. \
    A direction proposal is direction plus body, and situation is one of those five. When act is revise, leave direction null. \
    When an open proposal is listed and they ask to add or change it, set act revise. slug and body revise a direction proposal. title, brief, and due_days revise a project. The text is the whole proposal, including what they asked to add. \
    When they describe a new project, set the project act on this turn. Never ask if they want it published. When they describe a ticket and have not said who it is for, set the ticket act with who null. \
    When they agree to a ticket, done, or a ticket removal — yes, publish it, assign to me, remove it, delete it — set act again and say that you are opening it. A proposal is not opened by yes; they publish the draft. \
    When they name a DRI, set the dri act. It is a draft they publish. When they mark work done, set act and say that it is done. Do not ask them to confirm done. \
    project opens a project proposal. due_days is the review date the Shapers named, in days from today. who is a suggested holder only when they named a person; otherwise who is null. Do not set who to the speaker unless they said the project is for them. A null who is filled from Profiles when someone's about or the work they want matches, and they are under their limit. \
    done marks a ticket they already hold. \
    ticket creates a child under a project or ticket they hold, offered to them (who=me) or to someone else. who null means the same match: the person whose profile fits and who is holding the least. When nobody fits, leave who null and still set the ticket act. The ticket stays open for a match. When they say create the ticket, or put themselves as DRI, that is the yes: set the ticket act again, and set who to me when they named themselves. \
    parent is the overview title when you know it, copied in full. A shorter name is only safe when one live item starts with it. \
    When they describe the project in their own words, put that description in parent. \
    When they have not named or described a project, set parent to \"\". The project they hold that fits the ticket is filled in. Say which project you suggest. \
    due_days on a ticket is when it should be done. If they named a date, use that many days. \
    If they did not, estimate a suitable due_days from the work and write that number. \
    Do not ask them to pick a date, and do not leave due_days out. \
    dri names a holder for work that has none. \
    remove takes one project or ticket off the board. One removal per reply. \
    A project removal is always a proposal, whether one Shaper is seated or many. Only a Shaper can remove a project. \
    {{\"kind\":\"shapers\",\"op\":\"add\" or \"remove\",\"who\":\"a person's name\",\"why\":\"one line\"}} \
    add or remove opens a Shapers proposal. who is a member's name. \
    {{\"kind\":\"rules\",\"direction\":\"majority\",\"project\":\"majority\",\"dri\":\"majority\",\"shapers\":\"all\",\"money\":\"majority\",\"join\":\"majority\",\"decision_days\":7,\"offer_days\":3}} \
    rules changes how many Shapers must agree. Leave a kind out to keep majority. decision_days and offer_days are optional. \
    {{\"kind\":\"agent\",\"who\":\"hosted\" or a person's name,\"why\":\"one line\"}} \
    agent chooses who drafts for the org. who=hosted returns to the hosted agent. \
    When they tell you about themselves, the work they want, or a link to a social, set act profile on this turn. about is the new about, or null to keep the current one. skills is the full list of work they want, including what they already listed, or null to keep it. socials is the full list of links, or null to keep it. Each social is {{\"network\":\"github\",\"url\":\"https://github.com/name\"}}. Networks: website, github, x, linkedin, nostr, mastodon, telegram, discord, youtube, instagram, bluesky. Only https links. Say what you saved. Do not ask them to confirm. \
    A ticket: only the person who created it or the person who offered it. If they are not that person, leave act null and say so. \
    If several live items share the title, still set remove with that exact title. One copy is removed, the oldest. Say it is one copy. \
    If they ask to remove several, set remove for the first named title. \
    When they say remove, delete, go, or continue about a ticket, that is the yes. Set act again and say you are removing that one copy. A project removal stays a draft until they publish it. \
    If the title is missing, or they are not the person who can do it, leave act null and say why. \
    Answer questions from the overview only, and finish the answer. Do not invent work, people, or direction. \
    When they ask what you can do, say you can help with these and that nothing you draft is real until the right person agrees: \
    a direction proposal (mission, vision, situation, objectives, or strategy); \
    an honest read of any of them, with a sharper version, and help writing each in order; a project proposal; \
    changing an open proposal, which starts the vote over when they publish the draft; \
    marking their ticket done; creating a ticket for them or for someone else; \
    naming a DRI for work that has none; adding or removing a Shaper; changing the decision rules; choosing the org agent; removing a project (always a proposal, whether one Shaper is seated or many) or a ticket they created or offered; \
    saving what they say about themselves, the work they want, and their social links onto their profile; \
    and answering from the overview. \
    say has no headings and no lists. \
    If other people will join as Shapers and they are not settling an artifact or asking for a proposal yet, leave direction and act null.\n\
    Overview:\n{overview}"
    )
}

/// Read the model's JSON. A confirmation is not an artifact. An unknown act is dropped.
pub fn parse_model_reply(raw: &str) -> (String, Option<String>, Option<String>, Option<RawAct>) {
    let stripped = strip_think(raw);
    let Some(value) = json_object(&stripped) else {
        return (cap_say(&stripped), None, None, None);
    };
    let say = value
        .get("say")
        .and_then(Value::as_str)
        .map(cap_say)
        .unwrap_or_default();
    let mut direction = value.get("direction").and_then(direction_slug);
    let mut body = value
        .get("body")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| is_direction_body(text))
        .map(str::to_string);
    if direction.as_deref() == Some("situation") {
        if let Some(text) = body.as_mut() {
            *text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        }
    }
    if body.is_none() {
        direction = None;
    }
    let mut act = value.get("act").and_then(parse_act);
    if matches!(act, Some(RawAct::Revise { .. } | RawAct::Profile { .. })) {
        direction = None;
    } else if direction.is_some() {
        act = None;
    }
    (say, direction, body, act)
}

/// Turn a parsed act into tags, or nothing when the overview does not support it.
pub fn resolve_act(act: &RawAct, board: &Board, speaker: &str, now: u64) -> Option<ResolvedAct> {
    let speaker = speaker.trim().to_ascii_lowercase();
    match act {
        RawAct::Project {
            title,
            brief,
            due_days,
            who,
        } => {
            let title = clean_title(title)?;
            let brief = clean_brief(brief, &title);
            let suggested = match who.as_deref() {
                Some(name) => resolve_who(name, board, &speaker),
                None => board.suggest_holder(&title, &brief),
            };
            Some(ResolvedAct::Project {
                title,
                brief,
                due_at: due_unix(*due_days, now),
                suggested,
            })
        }
        RawAct::Done { item } => {
            let found = find_item(board, item)?;
            if !holds(found, &speaker) {
                return None;
            }
            Some(ResolvedAct::Done {
                item: found.id.clone(),
            })
        }
        RawAct::Ticket {
            parent,
            title,
            brief,
            due_days,
            who,
        } => {
            let title = clean_title(title)?;
            let brief = clean_brief(brief, &title);
            let (parent, parent_choice) =
                place_ticket_parent(board, parent, &title, &brief, &speaker)?;
            let offer_to = if who.trim().is_empty() {
                board.suggest_holder(&title, &brief)
            } else {
                Some(resolve_who(who, board, &speaker)?)
            };
            Some(ResolvedAct::Ticket {
                parent: parent.id.clone(),
                title,
                brief,
                due_at: ticket_due_at(*due_days, parent.due_at, now),
                offer_to,
                parent_choice,
            })
        }
        RawAct::Dri { item, who } => {
            let found = find_item(board, item)?;
            if found.state != "open" || found.dri.is_some() {
                return None;
            }
            let pubkey = resolve_who(who, board, &speaker)?;
            Some(ResolvedAct::Dri {
                item: found.id.clone(),
                title: found.title.clone(),
                who: pubkey,
            })
        }
        RawAct::Remove { item } => {
            let found = find_removable(board, item)?;
            let id = found.id.clone();
            let title = found.title.clone();
            let is_project = found.parent.is_none();
            let created_by = found.created_by.clone();
            let offered_by = found.offered_by.clone();
            if is_project {
                if !board.shapers.iter().any(|seat| seat == &speaker) || board.shapers.is_empty() {
                    return None;
                }
                Some(ResolvedAct::RemoveProject { item: id, title })
            } else if may_remove_ticket(&speaker, created_by.as_deref(), offered_by.as_deref()) {
                Some(ResolvedAct::Remove { item: id, title })
            } else {
                None
            }
        }
        RawAct::Revise { .. } => resolve_revise(board, &speaker, act, now),
        RawAct::Shapers { op, who, why } => {
            if !board.shapers.iter().any(|seat| seat == &speaker) {
                return None;
            }
            let pubkey = resolve_who(who, board, &speaker)?;
            let seated = board.shapers.iter().any(|seat| seat == &pubkey);
            if op == "add" {
                if seated {
                    return None;
                }
            } else if op == "remove" {
                if !seated || board.shapers.len() < 2 {
                    return None;
                }
            } else {
                return None;
            }
            let why = why.trim();
            let why = if why.is_empty() {
                "Asked in chat.".to_string()
            } else {
                why.to_string()
            };
            Some(ResolvedAct::Shapers {
                op: op.clone(),
                who: pubkey,
                why,
            })
        }
        RawAct::Rules {
            direction,
            project,
            dri,
            shapers,
            money,
            join,
            decision_days,
            offer_days,
        } => {
            if !board.shapers.iter().any(|seat| seat == &speaker) {
                return None;
            }
            let rules_json = format!(
                "{{\"direction\":{},\"project\":{},\"dri\":{},\"shapers\":{},\"money\":{},\"join\":{}}}",
                rule_json(direction),
                rule_json(project),
                rule_json(dri),
                rule_json(shapers),
                rule_json(money),
                rule_json(join),
            );
            Some(ResolvedAct::Rules {
                rules_json,
                decision_window_secs: decision_days.map(window_secs),
                offer_window_secs: offer_days.map(window_secs),
            })
        }
        RawAct::Agent { who, why } => {
            if !board.shapers.iter().any(|seat| seat == &speaker) {
                return None;
            }
            let pubkey = match who {
                Some(name) => {
                    let pubkey = resolve_who(name, board, &speaker)?;
                    if board.shapers.iter().any(|seat| seat == &pubkey) {
                        return None;
                    }
                    Some(pubkey)
                }
                None => None,
            };
            let why = why.trim();
            let why = if why.is_empty() {
                "Asked in chat.".to_string()
            } else {
                why.to_string()
            };
            Some(ResolvedAct::Agent { pubkey, why })
        }
        RawAct::Profile {
            about,
            skills,
            socials,
        } => {
            let current = board.stored_profile(&speaker);
            let socials = socials.as_ref().map(|rows| {
                rows.iter()
                    .map(|(network, url)| buzz_core::intelligent_org::ProfileSocial {
                        network: network.clone(),
                        url: url.clone(),
                    })
                    .collect()
            });
            let merged = crate::profile_fit::merge_profile(
                &current,
                &crate::profile_fit::ProfilePatch {
                    about: about.clone(),
                    skills: skills.clone(),
                    socials,
                },
            )?;
            Some(ResolvedAct::Profile {
                about: merged.about,
                skills: merged.skills,
                socials: merged
                    .socials
                    .into_iter()
                    .map(|social| (social.network, social.url))
                    .collect(),
                open_limit: merged.open_limit,
            })
        }
    }
}

fn window_secs(days: u32) -> u64 {
    u64::from(days.clamp(1, 90)) * 86_400
}

fn rule_json(value: &str) -> String {
    if value.chars().all(|ch| ch.is_ascii_digit()) && !value.is_empty() {
        value.to_string()
    } else {
        format!("{value:?}")
    }
}

fn rule_token(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => {
            let text = text.trim().to_ascii_lowercase();
            if text == "all" {
                "all".to_string()
            } else if let Ok(count) = text.parse::<u64>() {
                if count >= 1 {
                    count.to_string()
                } else {
                    "majority".to_string()
                }
            } else {
                "majority".to_string()
            }
        }
        Some(Value::Number(count)) => {
            let count = count.as_u64().unwrap_or(0);
            if count >= 1 {
                count.to_string()
            } else {
                "majority".to_string()
            }
        }
        _ => "majority".to_string(),
    }
}

fn window_days(value: &Value) -> Option<u32> {
    let days = value.as_u64()? as u32;
    if days == 0 {
        None
    } else {
        Some(days.clamp(1, 90))
    }
}

fn resolve_revise(board: &Board, speaker: &str, act: &RawAct, now: u64) -> Option<ResolvedAct> {
    let RawAct::Revise {
        proposal,
        slug,
        body,
        title,
        brief,
        due_days,
    } = act
    else {
        return None;
    };
    if !board.shapers.iter().any(|seat| seat == speaker) {
        return None;
    }
    let found = board.proposals.get(proposal.trim())?;
    if let Some(slug) = slug.as_deref() {
        if found.kind != "direction" || found.slug.as_deref() != Some(slug) {
            return None;
        }
        let body = body
            .as_deref()
            .map(str::trim)
            .filter(|text| is_direction_body(text))?;
        return Some(ResolvedAct::ReviseDirection {
            proposal: found.id.clone(),
            slug: slug.to_string(),
            body: body.to_string(),
            base: found.base.unwrap_or(0),
        });
    }
    if found.kind != "project" {
        return None;
    }
    let title = clean_title(title.as_deref().unwrap_or(""))?;
    let days = (*due_days).unwrap_or(14);
    Some(ResolvedAct::ReviseProject {
        proposal: found.id.clone(),
        title: title.clone(),
        brief: clean_brief(brief.as_deref().unwrap_or(""), &title),
        due_at: due_unix(days, now),
    })
}

/// The latest human message is a yes to the draft already shown.
pub fn agrees_to_publish(text: &str) -> bool {
    let normalized = normalize_reply(text);
    matches!(
        normalized.as_str(),
        "yes"
            | "yeah"
            | "yep"
            | "yup"
            | "ok"
            | "okay"
            | "sure"
            | "publish"
            | "publish it"
            | "yes publish"
            | "yes publish it"
            | "yes publish this"
            | "yes please"
            | "go ahead"
            | "do it"
            | "ship it"
            | "assign to me"
            | "assign it to me"
            | "assign this to me"
            | "remove it"
            | "delete it"
            | "yes remove it"
            | "yes delete it"
            | "remove them"
            | "delete them"
    )
}

/// A short go-ahead for a removal already named. Not a yes to any other draft.
pub fn agrees_to_remove(text: &str) -> bool {
    let normalized = normalize_reply(text);
    matches!(
        normalized.as_str(),
        "remove"
            | "delete"
            | "go"
            | "continue"
            | "next"
            | "remove it"
            | "delete it"
            | "remove them"
            | "delete them"
            | "yes remove"
            | "yes delete"
            | "yes remove it"
            | "yes delete it"
    )
}

/// The person asked for a new project in this message.
pub fn asked_for_new_project(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("ticket") && !lower.contains("project") {
        return false;
    }
    lower.contains("new project")
        || lower.contains("create a project")
        || lower.contains("create project")
        || lower.contains("open a project")
        || (lower.contains("project") && lower.contains("create"))
}

/// A direction draft the model described in prose instead of `direction` and `body`.
///
/// "I want to set a new mission, it should be …" is the request. The card
/// still goes on this reply when the model only talks about the wording.
/// Coaching about what a good text needs, or a question back, is not a draft.
///
/// The common noun "situation" is not a request. "Create situation draft" is.
pub fn recover_direction(user: &str, say: &str) -> Option<(String, String)> {
    if is_proposal_announcement(user) {
        return None;
    }
    let slug = direction_request_slug(user)?;
    let body = if slug == "situation" {
        said_situation(say).or_else(|| stated_situation(user))
    } else {
        said_artifact(say, user).or_else(|| stated_artifact(user))
    }?;
    Some((slug.to_string(), body))
}

/// The artifact in the agent's reply: one it announces as a draft, or their
/// own words cleaned up.
fn said_artifact(say: &str, user: &str) -> Option<String> {
    announced_draft(say).or_else(|| stated_artifact(say).filter(|text| restates(text, user)))
}

/// "Here's a draft of the mission: …" or "Sharper draft: …".
fn announced_draft(say: &str) -> Option<String> {
    let lower = say.to_ascii_lowercase();
    lower.match_indices(':').rev().find_map(|(at, _)| {
        let clause_start = lower[..at]
            .rfind(['.', '!', '?', '\n'])
            .map_or(0, |end| end + 1);
        if !lower[clause_start..at].contains("draft") {
            return None;
        }
        first_direction_sentence(&say[at + 1..])
    })
}

/// Most of the sentence's words are theirs.
fn restates(sentence: &str, user: &str) -> bool {
    let heard: HashSet<String> = content_words(user).collect();
    let words: Vec<String> = content_words(sentence).collect();
    if words.is_empty() {
        return false;
    }
    let kept = words.iter().filter(|word| heard.contains(*word)).count();
    kept * 5 >= words.len() * 3
}

fn content_words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| word.chars().count() >= 3)
        .map(str::to_lowercase)
}

fn direction_request_slug(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    if situation_draft_request(&lower) {
        return Some("situation");
    }
    let slug = ["mission", "vision", "objectives", "strategy"]
        .into_iter()
        .find(|slug| mentions_slug(&lower, slug))?;
    const ASKS: &[&str] = &[
        "set ",
        "new ",
        "create ",
        "change ",
        "update ",
        "draft ",
        "write ",
        "make ",
        "add ",
        "should be",
        "replace ",
    ];
    if ASKS.iter().any(|word| lower.contains(word)) {
        return Some(slug);
    }
    None
}

/// Names the situation artifact. The bare word is a common noun
/// ("a tough situation") and must not open a direction draft.
fn situation_draft_request(lower: &str) -> bool {
    const PHRASES: &[&str] = &[
        "situation draft",
        "situation proposal",
        "draft the situation",
        "draft a situation",
        "draft situation",
        "create the situation",
        "create a situation",
        "create situation",
        "set the situation",
        "set a situation",
        "set situation",
        "write the situation",
        "write a situation",
        "write situation",
        "propose the situation",
        "propose a situation",
        "propose situation",
        "make the situation",
        "make a situation",
        "update the situation",
        "change the situation",
        "the situation should",
    ];
    if PHRASES.iter().any(|phrase| contains_phrase(lower, phrase)) {
        return true;
    }
    // "create draft for current situation" names the artifact without the
    // exact phrase "situation draft". A project request that merely mentions
    // a tough situation does not.
    word_has(lower, "situation")
        && !lower.contains("project")
        && ["draft", "create", "write", "propose"]
            .iter()
            .any(|verb| word_has(lower, verb))
}

fn contains_phrase(lower: &str, phrase: &str) -> bool {
    lower.match_indices(phrase).any(|(at, _)| {
        let before_ok = lower[..at]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric());
        let after_ok = lower[at + phrase.len()..]
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric());
        before_ok && after_ok
    })
}

/// The situation paragraph the agent announced, or the one it stated while
/// refusing to call it a draft.
fn said_situation(say: &str) -> Option<String> {
    announced_situation(say).or_else(|| situation_from_reply(say))
}

fn announced_situation(say: &str) -> Option<String> {
    let lower = say.to_ascii_lowercase();
    lower.match_indices(':').rev().find_map(|(at, _)| {
        let clause_start = lower[..at]
            .rfind(['.', '!', '?', '\n'])
            .map_or(0, |end| end + 1);
        let clause = &lower[clause_start..at];
        if !clause.contains("draft") && !clause.contains("situation") {
            return None;
        }
        situation_paragraph(&say[at + 1..])
    })
}

/// Sentences that state where the org stands, skipping a denial and stopping
/// when the reply turns to a project.
fn situation_from_reply(say: &str) -> Option<String> {
    let sentences = situation_sentences(say);
    let start = sentences
        .iter()
        .rposition(|sentence| handoff_sentence(sentence))
        .map_or(0, |index| index + 1);
    let mut kept = Vec::new();
    for sentence in sentences.iter().skip(start) {
        if pivot_sentence(sentence) || meta_situation_sentence(sentence) {
            break;
        }
        if denial_sentence(sentence) {
            continue;
        }
        kept.push(sentence.clone());
        if kept.len() == 6 {
            break;
        }
    }
    let paragraph = kept.join(" ");
    is_direction_body(&paragraph).then_some(paragraph)
}

fn situation_sentences(text: &str) -> Vec<String> {
    let mut rest = text.trim();
    let mut sentences = Vec::new();
    while !rest.is_empty() {
        let Some(end) = rest.find(['.', '!', '?']) else {
            let tail = clean_situation_sentence(rest);
            if !tail.is_empty() {
                sentences.push(tail);
            }
            break;
        };
        if rest[end..].starts_with('?') {
            rest = rest[end + 1..].trim_start();
            continue;
        }
        let sentence = clean_situation_sentence(&rest[..=end]);
        if !sentence.is_empty() {
            sentences.push(sentence);
        }
        rest = rest[end + 1..].trim_start();
    }
    sentences
}

fn handoff_sentence(sentence: &str) -> bool {
    let lower = sentence.to_ascii_lowercase();
    lower.contains("say it back")
        || lower.contains("here it is")
        || lower.contains("here is where you stand")
        || (lower.contains("instead") && lower.contains("draft"))
}

fn denial_sentence(sentence: &str) -> bool {
    let lower = sentence.to_ascii_lowercase();
    lower.contains("no separate")
        || lower.contains("holds only")
        || lower.contains("only mission")
        || lower.contains("not a separate")
}

fn pivot_sentence(sentence: &str) -> bool {
    let lower = sentence.to_ascii_lowercase();
    lower.contains("nothing in direction")
        || lower.contains("as a project")
        || lower.contains("first project")
        || lower.contains("missing is work")
        || lower.contains("board is empty")
        || lower.starts_with("should ")
}

fn stated_situation(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    for marker in ["it should be", "the situation should be", "should be"] {
        if let Some(at) = lower.rfind(marker) {
            if let Some(paragraph) = situation_paragraph(&text[at + marker.len()..]) {
                return Some(paragraph);
            }
        }
    }
    None
}

/// Up to six sentences, one paragraph, stopping before a question or a publish cue.
fn situation_paragraph(text: &str) -> Option<String> {
    let mut rest = text
        .trim()
        .trim_start_matches(|ch: char| matches!(ch, ':' | ',' | '-' | ' '));
    let mut sentences = Vec::new();
    while !rest.is_empty() && sentences.len() < 6 {
        let Some(end) = rest.find(['.', '!', '?']) else {
            let tail = clean_situation_sentence(rest);
            if !tail.is_empty() && !meta_situation_sentence(&tail) {
                sentences.push(tail);
            }
            break;
        };
        if rest[end..].starts_with('?') {
            break;
        }
        let sentence = clean_situation_sentence(&rest[..=end]);
        if sentence.is_empty() || meta_situation_sentence(&sentence) {
            break;
        }
        sentences.push(sentence);
        rest = rest[end + 1..].trim_start();
    }
    let paragraph = sentences.join(" ");
    is_direction_body(&paragraph).then_some(paragraph)
}

fn clean_situation_sentence(text: &str) -> String {
    text.trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '“' | '”'))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn meta_situation_sentence(sentence: &str) -> bool {
    let lower = sentence.to_ascii_lowercase();
    lower.starts_with("open it")
        || lower.starts_with("publish")
        || lower.starts_with("want ")
        || lower.starts_with("does this")
        || lower.starts_with("shall i")
        || lower.starts_with("you can ")
}

fn slug_needles(slug: &str) -> &'static [&'static str] {
    match slug {
        "objectives" => &["objectives", "objective"],
        "strategy" => &["strategies", "strategy"],
        "mission" => &["mission"],
        "vision" => &["vision"],
        _ => &[],
    }
}

fn mentions_slug(lower: &str, slug: &str) -> bool {
    slug_needles(slug)
        .iter()
        .any(|needle| word_has(lower, needle))
}

fn word_has(lower: &str, needle: &str) -> bool {
    lower.match_indices(needle).any(|(at, _)| {
        let before_ok = lower[..at]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric());
        let after_ok = lower[at + needle.len()..]
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric());
        before_ok && after_ok
    })
}

/// One line of a room, in order. `acted` means that message already carried a draft tag.
pub struct ChatTurn<'a> {
    pub from_agent: bool,
    pub content: &'a str,
    pub acted: bool,
}

/// A short yes to wording the agent already wrote. Not a new artifact.
pub fn approves_wording(text: &str) -> bool {
    let normalized = normalize_reply(text);
    matches!(
        normalized.as_str(),
        "yes"
            | "yeah"
            | "yep"
            | "yup"
            | "ok"
            | "okay"
            | "sure"
            | "good"
            | "it is good"
            | "its good"
            | "it's good"
            | "that is good"
            | "thats good"
            | "that's good"
            | "looks good"
            | "sounds good"
            | "perfect"
            | "do it"
            | "go ahead"
    ) || agrees_to_publish(text)
}

/// They approved a direction the agent already stated, and that turn had no draft.
pub fn recover_confirmed_direction(
    lines: &[ChatTurn<'_>],
    approval: &str,
) -> Option<(String, String)> {
    if !approves_wording(approval) {
        return None;
    }
    let mut last_user = "";
    let mut last_agent = "";
    for line in lines {
        if line.from_agent {
            last_agent = line.content;
            continue;
        }
        if normalize_reply(line.content) == normalize_reply(approval) {
            break;
        }
        last_user = line.content;
    }
    if last_user.is_empty() || last_agent.is_empty() {
        return None;
    }
    recover_direction(last_user, last_agent)
}

/// The newest direction they stated that never received a draft tag.
pub fn missing_direction_draft(lines: &[ChatTurn<'_>]) -> Option<(String, String)> {
    let mut last_user = "";
    let mut pending: Option<(String, String)> = None;
    for line in lines {
        if !line.from_agent {
            last_user = line.content;
            continue;
        }
        if line.acted {
            pending = None;
            continue;
        }
        if let Some(found) = recover_direction(last_user, line.content) {
            pending = Some(found);
        }
    }
    pending
}

fn stated_artifact(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if let Some(at) = lower.find("it should be") {
        if let Some(sentence) = first_direction_sentence(&text[at + "it should be".len()..]) {
            return Some(sentence);
        }
    }
    if let Some(at) = lower.find("should be") {
        if let Some(sentence) = first_direction_sentence(&text[at + "should be".len()..]) {
            return Some(sentence);
        }
    }
    if let Some(at) = text.rfind(':') {
        if let Some(sentence) = first_direction_sentence(&text[at + 1..]) {
            return Some(sentence);
        }
    }
    None
}

fn first_direction_sentence(text: &str) -> Option<String> {
    let rest = text
        .trim()
        .trim_start_matches(|ch: char| matches!(ch, ':' | ',' | '-' | ' '));
    let end = rest.find(['.', '!', '?']).unwrap_or(rest.len());
    if rest[end..].starts_with('?') {
        return None;
    }
    let sentence = rest[..end]
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '“' | '”'));
    if is_direction_body(sentence) {
        Some(sentence.to_string())
    } else {
        None
    }
}

/// The line above a direction draft. Publishing stays their tap.
pub fn direction_draft_say(slug: &str) -> String {
    let label = match slug {
        "mission" | "vision" | "situation" | "objectives" | "strategy" => slug,
        _ => "direction",
    };
    format!("A draft of the {label}. Open it, then publish.")
}

/// The turn the model sees when a direction it drafted here was confirmed.
/// The prompt's coaching names this note as the cue to start the next piece.
pub fn confirmed_note(slug: &str, version: u64) -> String {
    format!(
        "(Note from the relay, not a member: the {slug} was just confirmed as version {version}. \
         Nobody has written since.)"
    )
}

/// The line the desktop posts when someone publishes a draft. It restates
/// the proposal, so treating it as a request drafts the same text again,
/// often under the wrong slug when the text names another piece.
pub fn is_proposal_announcement(content: &str) -> bool {
    let lower = content.trim_start().to_ascii_lowercase();
    lower.starts_with("opened a ") && lower.contains(" proposal:")
}

/// A direction tag is a draft. A reply that says it is already published
/// is rewritten so the card and the sentence agree.
pub fn direction_reply_say(say: &str, slug: &str) -> String {
    let lower = say.to_ascii_lowercase();
    if lower.contains("published") {
        return direction_draft_say(slug);
    }
    say.trim().to_string()
}

/// The draft card already shows `body` under the reply. Sentences of `say`
/// that restate one of its lines are dropped. When nothing else is left, the
/// reply is the plain draft line.
pub fn drop_restated_lines(say: &str, slug: &str, body: &str) -> String {
    let lines: Vec<Vec<String>> = body
        .lines()
        .map(longer_words)
        .filter(|words| !words.is_empty())
        .collect();
    if lines.is_empty() {
        return say.trim().to_string();
    }
    let kept: Vec<&str> = say_sentences(say)
        .into_iter()
        .filter(|sentence| {
            let words = longer_words(sentence);
            if sentence.ends_with('?') || words.len() < 6 {
                return true;
            }
            !lines.iter().any(|line| {
                let shared = words.iter().filter(|word| line.contains(word)).count();
                shared * 10 >= words.len() * 6 || shared * 10 >= line.len() * 7
            })
        })
        .collect();
    let text = kept.join(" ").trim().to_string();
    if text.is_empty() {
        direction_draft_say(slug)
    } else {
        text
    }
}

fn longer_words(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| word.chars().count() >= 3)
        .map(str::to_lowercase)
        .collect()
}

fn say_sentences(say: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = say.char_indices().peekable();
    while let Some((at, ch)) = chars.next() {
        let ends = matches!(ch, '.' | '!' | '?')
            && chars.peek().is_none_or(|(_, next)| next.is_whitespace());
        if ends {
            let end = at + ch.len_utf8();
            let sentence = say[start..end].trim();
            if !sentence.is_empty() {
                out.push(sentence);
            }
            start = end;
        }
    }
    let rest = say[start..].trim();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

/// A project the model described in prose instead of an act.
///
/// The chat history still contains "Want me to publish this proposal?", so
/// the model copies that and leaves `act` null. The draft still goes on this
/// reply.
pub fn recover_project(user: &str, say: &str) -> Option<RawAct> {
    let lower = say.to_ascii_lowercase();
    let offered = lower.contains("want me to publish") || lower.contains("up for a vote");
    if !offered && !asked_for_new_project(user) {
        return None;
    }
    if offered {
        if let Some(act) = project_from_sentences(say) {
            return Some(act);
        }
    }
    if asked_for_new_project(user) {
        return project_from_request(user);
    }
    None
}

fn project_from_sentences(say: &str) -> Option<RawAct> {
    let cut = ["want me to publish", "open the draft", "up for a vote"]
        .iter()
        .filter_map(|marker| {
            say.to_ascii_lowercase()
                .find(marker)
                .map(|at| say[..at].to_string())
        })
        .min_by_key(String::len);
    let head = cut.unwrap_or_else(|| say.to_string());
    let head = head
        .trim()
        .trim_end_matches(|ch: char| matches!(ch, '.' | '!' | '?'));
    let (title, brief) = head.split_once(". ")?;
    let title = clean_title(title)?;
    let brief = brief
        .trim()
        .trim_end_matches(|ch: char| matches!(ch, '.' | '!' | '?'));
    if brief.chars().count() < 8 {
        return None;
    }
    let brief = clean_brief(brief, &title);
    Some(RawAct::Project {
        title,
        brief,
        due_days: 14,
        who: None,
    })
}

fn project_from_request(user: &str) -> Option<RawAct> {
    let lower = user.to_ascii_lowercase();
    let (marker, at) = ["new project", "a project", "create project"]
        .iter()
        .filter_map(|marker| lower.find(marker).map(|at| (*marker, at)))
        .min_by_key(|(_, at)| *at)?;
    let rest = user[at + marker.len()..].trim();
    let rest = rest.trim_start_matches(|ch: char| matches!(ch, ',' | ':' | '-' | ' '));
    let rest = rest
        .strip_prefix("about ")
        .or_else(|| rest.strip_prefix("to "))
        .or_else(|| rest.strip_prefix("called "))
        .unwrap_or(rest)
        .trim();
    let title = clean_title(rest)?;
    let brief = clean_brief(rest, &title);
    Some(RawAct::Project {
        title,
        brief,
        due_days: 14,
        who: None,
    })
}

/// A DRI the model described in prose instead of an act.
///
/// "Make a proposal to set me as DRI for Build Best Site" is the request.
/// The draft still goes on this reply when `act` comes back null.
pub fn recover_dri(user: &str, say: &str) -> Option<RawAct> {
    if let Some(act) = dri_from_text(user) {
        return Some(act);
    }
    let lower = say.to_ascii_lowercase();
    let claimed = lower.contains("dri proposal") || lower.contains("drafting a dri");
    if claimed {
        return dri_from_text(say);
    }
    None
}

fn dri_from_text(text: &str) -> Option<RawAct> {
    if !asks_to_name_dri(text) {
        return None;
    }
    let (who, item) = split_dri_request(text)?;
    Some(RawAct::Dri { item, who })
}

fn asks_to_name_dri(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if !lower.contains("dri") && !lower.contains("holder") {
        return false;
    }
    ["set ", "name ", "make ", "assign ", "proposal", "draft"]
        .iter()
        .any(|word| lower.contains(word))
}

fn split_dri_request(text: &str) -> Option<(String, String)> {
    let lower = text.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        " as the dri ",
        " as dri ",
        " as the holder ",
        " as holder ",
        " the dri for ",
        " the dri of ",
        " dri for ",
        " dri of ",
        " holder of ",
        " holder for ",
    ];
    let mut found = None;
    for marker in MARKERS {
        let Some(at) = lower.find(marker) else {
            continue;
        };
        let Some(who) = who_word(&text[..at]) else {
            continue;
        };
        let Some(item) = item_words(&text[at + marker.len()..]) else {
            continue;
        };
        let item_lower = item.to_ascii_lowercase();
        if item_lower.contains("dri")
            || item_lower.contains("holder")
            || item_lower.contains("proposal")
        {
            continue;
        }
        found = Some((who, item));
    }
    found
}

fn who_word(before: &str) -> Option<String> {
    let last = before.split_whitespace().last()?;
    let last =
        last.trim_matches(|ch: char| matches!(ch, ',' | '.' | '!' | '?' | ':' | '"' | '“' | '”'));
    const SKIP: &[&str] = &[
        "as", "the", "a", "an", "to", "for", "of", "set", "name", "make", "proposal", "draft",
        "and",
    ];
    if last.is_empty() || SKIP.iter().any(|word| last.eq_ignore_ascii_case(word)) {
        return None;
    }
    Some(last.to_string())
}

fn item_words(after: &str) -> Option<String> {
    let rest = after.trim();
    let rest = rest
        .strip_prefix("for ")
        .or_else(|| rest.strip_prefix("For "))
        .or_else(|| rest.strip_prefix("of "))
        .or_else(|| rest.strip_prefix("Of "))
        .or_else(|| rest.strip_prefix("on "))
        .or_else(|| rest.strip_prefix("On "))
        .unwrap_or(rest)
        .trim();
    let rest =
        rest.trim_matches(|ch: char| matches!(ch, '.' | '!' | '?' | '"' | '“' | '”' | ',' | ':'));
    let rest = rest.split(['.', '!', '?']).next().unwrap_or(rest).trim();
    if rest.chars().count() < 3 {
        return None;
    }
    Some(rest.to_string())
}

/// They asked to open a proposal, so a project act publishes on this turn.
pub fn asks_for_proposal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if !lower.contains("proposal") {
        return false;
    }
    [
        "create", "open", "make", "draft", "publish", "start", "put up",
    ]
    .iter()
    .any(|verb| lower.contains(verb))
}

fn normalize_reply(text: &str) -> String {
    text.trim()
        .trim_matches(|ch: char| matches!(ch, '.' | '!' | '?'))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// The display name of the person a draft names, when their profile has one.
/// A pubkey prefix is not a name.
pub fn named_holder(act: &ResolvedAct, board: &Board) -> Option<String> {
    let pubkey = holder_pubkey(act)?;
    board.stored_name(pubkey)
}

fn holder_pubkey(act: &ResolvedAct) -> Option<&str> {
    match act {
        ResolvedAct::Project { suggested, .. } => suggested.as_deref(),
        ResolvedAct::Ticket { offer_to, .. } => offer_to.as_deref(),
        ResolvedAct::Dri { who, .. } => Some(who.as_str()),
        ResolvedAct::Shapers { who, .. } => Some(who.as_str()),
        ResolvedAct::Agent { pubkey, .. } => pubkey.as_deref(),
        _ => None,
    }
}

/// What the chat shows while the act is still only a draft.
pub fn draft_say(act: &ResolvedAct) -> String {
    match act {
        ResolvedAct::Project { title, brief, .. } => {
            format!("{title}. {brief} Open the draft to change it, then publish.")
        }
        ResolvedAct::Ticket {
            title,
            brief,
            offer_to,
            ..
        } => {
            let ask = if offer_to.is_some() {
                "Want me to offer this ticket?"
            } else {
                "Want me to open this ticket?"
            };
            format!("{title}. {brief} {ask}")
        }
        ResolvedAct::Done { .. } => "Mark that done? Say yes and I will.".to_string(),
        ResolvedAct::Dri { title, .. } => {
            format!("A draft to name a holder for “{title}”. Open it, then publish.")
        }
        ResolvedAct::Remove { title, .. } => {
            format!("Remove “{title}”? Say yes and I will.")
        }
        ResolvedAct::RemoveProject { title, .. } => {
            format!("A draft to remove “{title}”. Open it, then publish.")
        }
        ResolvedAct::ReviseDirection { .. } | ResolvedAct::ReviseProject { .. } => {
            "Updated the draft. Open it, then publish. The vote starts over when you do."
                .to_string()
        }
        ResolvedAct::Shapers { op, .. } => {
            if op == "add" {
                "A draft to add a Shaper. Open it, then publish.".to_string()
            } else {
                "A draft to remove a Shaper. Open it, then publish.".to_string()
            }
        }
        ResolvedAct::Rules { .. } => {
            "A draft to change the decision rules. Open it, then publish.".to_string()
        }
        ResolvedAct::Agent { .. } => {
            "A draft to choose the org agent. Open it, then publish.".to_string()
        }
        ResolvedAct::Profile { .. } => "Saving that on your profile.".to_string(),
    }
}

/// What the chat shows once they have agreed and the client may sign.
pub fn published_say(act: &ResolvedAct) -> String {
    match act {
        ResolvedAct::Project { title, .. } => {
            format!("Publishing “{title}”. It is real once the proposal passes.")
        }
        ResolvedAct::Ticket {
            title, offer_to, ..
        } => {
            if offer_to.is_some() {
                format!("Offering “{title}”.")
            } else {
                format!("Opening “{title}”.")
            }
        }
        ResolvedAct::Done { .. } => "Marking that done.".to_string(),
        ResolvedAct::Dri { title, .. } => {
            format!("Opening the proposal to name a holder for “{title}”.")
        }
        ResolvedAct::Remove { title, .. } => format!("Removing “{title}”."),
        ResolvedAct::RemoveProject { title, .. } => {
            format!("Opening a proposal to remove “{title}”. It is gone once the Shapers agree.")
        }
        ResolvedAct::ReviseDirection { .. } | ResolvedAct::ReviseProject { .. } => {
            "Updated that proposal. The vote starts over.".to_string()
        }
        ResolvedAct::Shapers { op, .. } => {
            if op == "add" {
                "Opening the proposal to add that Shaper.".to_string()
            } else {
                "Opening the proposal to remove that Shaper.".to_string()
            }
        }
        ResolvedAct::Rules { .. } => "Opening the proposal to change the rules.".to_string(),
        ResolvedAct::Agent { .. } => "Opening the proposal to choose the org agent.".to_string(),
        ResolvedAct::Profile { .. } => "Saved that on your profile.".to_string(),
    }
}

/// Tag rows for a resolved act. `from` is the person whose client may sign.
pub fn act_tags(act: &ResolvedAct, from: &str) -> Vec<Vec<String>> {
    let mut tags = vec![vec!["from".to_string(), from.to_string()]];
    match act {
        ResolvedAct::Project {
            title,
            brief,
            due_at,
            suggested,
        } => {
            tags.push(vec!["project".into(), title.clone(), brief.clone()]);
            tags.push(vec!["due".into(), due_at.to_string()]);
            if let Some(pubkey) = suggested {
                tags.push(vec![
                    "p".into(),
                    pubkey.clone(),
                    "".into(),
                    "suggested".into(),
                ]);
            }
        }
        ResolvedAct::Done { item } => {
            tags.push(vec!["done".into(), item.clone()]);
        }
        ResolvedAct::Ticket {
            parent,
            title,
            brief,
            due_at,
            offer_to,
            ..
        } => {
            tags.push(vec![
                "ticket".into(),
                parent.clone(),
                title.clone(),
                brief.clone(),
            ]);
            tags.push(vec!["due".into(), due_at.to_string()]);
            if let Some(pubkey) = offer_to {
                tags.push(vec!["p".into(), pubkey.clone()]);
            }
        }
        ResolvedAct::Dri { item, who, .. } => {
            tags.push(vec!["dri".into(), item.clone(), who.clone()]);
        }
        ResolvedAct::Remove { item, .. } => {
            tags.push(vec!["remove".into(), item.clone()]);
        }
        ResolvedAct::RemoveProject { item, .. } => {
            tags.push(vec!["remove".into(), item.clone(), "proposal".into()]);
        }
        ResolvedAct::ReviseDirection {
            proposal,
            slug,
            body,
            base,
        } => {
            tags.push(vec![
                "revise".into(),
                proposal.clone(),
                "direction".into(),
                slug.clone(),
                body.clone(),
            ]);
            tags.push(vec!["base".into(), base.to_string()]);
        }
        ResolvedAct::ReviseProject {
            proposal,
            title,
            brief,
            due_at,
        } => {
            tags.push(vec![
                "revise".into(),
                proposal.clone(),
                "project".into(),
                title.clone(),
                brief.clone(),
            ]);
            tags.push(vec!["due".into(), due_at.to_string()]);
        }
        ResolvedAct::Shapers { op, who, why } => {
            tags.push(vec!["shapers".into(), op.clone(), who.clone(), why.clone()]);
        }
        ResolvedAct::Rules {
            rules_json,
            decision_window_secs,
            offer_window_secs,
        } => {
            tags.push(vec![
                "shapers".into(),
                "rules".into(),
                rules_json.clone(),
                decision_window_secs
                    .map(|secs| secs.to_string())
                    .unwrap_or_default(),
                offer_window_secs
                    .map(|secs| secs.to_string())
                    .unwrap_or_default(),
            ]);
        }
        ResolvedAct::Agent { pubkey, why } => {
            tags.push(vec![
                "shapers".into(),
                "agent".into(),
                pubkey.clone().unwrap_or_default(),
                why.clone(),
            ]);
        }
        ResolvedAct::Profile {
            about,
            skills,
            socials,
            open_limit,
        } => {
            tags.push(vec!["profile".into(), about.clone()]);
            let mut skill_tag = vec!["skills".into()];
            skill_tag.extend(skills.iter().cloned());
            tags.push(skill_tag);
            let socials = socials
                .iter()
                .map(|(network, url)| serde_json::json!({ "network": network, "url": url }))
                .collect::<Vec<_>>();
            tags.push(vec![
                "socials".into(),
                serde_json::Value::Array(socials).to_string(),
            ]);
            tags.push(vec![
                "limit".into(),
                open_limit
                    .map(|limit| limit.to_string())
                    .unwrap_or_default(),
            ]);
        }
    }
    tags
}

fn parse_act(value: &Value) -> Option<RawAct> {
    let kind = value.get("kind").and_then(Value::as_str)?;
    match kind {
        "project" => Some(RawAct::Project {
            title: value.get("title").and_then(Value::as_str)?.to_string(),
            brief: value
                .get("brief")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            due_days: due_days(value.get("due_days")),
            who: optional_who(value.get("who")),
        }),
        "done" => Some(RawAct::Done {
            item: value.get("item").and_then(Value::as_str)?.to_string(),
        }),
        "ticket" => Some(RawAct::Ticket {
            parent: value
                .get("parent")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            title: value.get("title").and_then(Value::as_str)?.to_string(),
            brief: value
                .get("brief")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            due_days: optional_due_days(value.get("due_days")),
            who: optional_who(value.get("who")).unwrap_or_default(),
        }),
        "dri" => Some(RawAct::Dri {
            item: value.get("item").and_then(Value::as_str)?.to_string(),
            who: value.get("who").and_then(Value::as_str)?.to_string(),
        }),
        "remove" => Some(RawAct::Remove {
            item: value.get("item").and_then(Value::as_str)?.to_string(),
        }),
        "revise" => {
            let proposal = value
                .get("proposal")
                .and_then(Value::as_str)?
                .trim()
                .to_string();
            if proposal.is_empty() {
                return None;
            }
            Some(RawAct::Revise {
                proposal,
                slug: value.get("slug").and_then(direction_slug),
                body: value
                    .get("body")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(str::to_string),
                title: value
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                brief: value
                    .get("brief")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                due_days: value.get("due_days").map(|days| due_days(Some(days))),
            })
        }
        "shapers" => {
            let op = value.get("op").and_then(Value::as_str)?.trim();
            if op != "add" && op != "remove" {
                return None;
            }
            let who = value.get("who").and_then(Value::as_str)?.trim();
            if who.is_empty() {
                return None;
            }
            Some(RawAct::Shapers {
                op: op.to_string(),
                who: who.to_string(),
                why: value
                    .get("why")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            })
        }
        "rules" => Some(RawAct::Rules {
            direction: rule_token(value.get("direction")),
            project: rule_token(value.get("project")),
            dri: rule_token(value.get("dri")),
            shapers: rule_token(value.get("shapers")),
            money: rule_token(value.get("money")),
            join: rule_token(value.get("join")),
            decision_days: value.get("decision_days").and_then(window_days),
            offer_days: value.get("offer_days").and_then(window_days),
        }),
        "agent" => {
            let who = value
                .get("who")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| {
                    !name.is_empty()
                        && !name.eq_ignore_ascii_case("hosted")
                        && !name.eq_ignore_ascii_case("null")
                })
                .map(str::to_string);
            Some(RawAct::Agent {
                who,
                why: value
                    .get("why")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            })
        }
        "profile" => Some(RawAct::Profile {
            about: optional_profile_text(value.get("about")),
            skills: optional_string_list(value.get("skills")),
            socials: optional_socials(value.get("socials")),
        }),
        _ => None,
    }
}

fn optional_who(value: Option<&Value>) -> Option<String> {
    let text = value?.as_str()?.trim();
    if text.is_empty()
        || text.eq_ignore_ascii_case("null")
        || text.eq_ignore_ascii_case("nobody")
        || text.eq_ignore_ascii_case("none")
        || text.eq_ignore_ascii_case("no one")
    {
        None
    } else {
        Some(text.to_string())
    }
}

/// `null` and a missing field keep the current profile text. A string replaces it.
fn optional_profile_text(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if value.is_null() {
        return None;
    }
    value.as_str().map(str::trim).map(str::to_string)
}

fn optional_string_list(value: Option<&Value>) -> Option<Vec<String>> {
    let value = value?;
    if value.is_null() {
        return None;
    }
    value.as_array().map(|rows| {
        rows.iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(str::to_string)
            .collect()
    })
}

fn optional_socials(value: Option<&Value>) -> Option<Vec<(String, String)>> {
    let value = value?;
    if value.is_null() {
        return None;
    }
    value.as_array().map(|rows| {
        rows.iter()
            .filter_map(|row| {
                let network = text_field(row, "network").unwrap_or_default();
                let url = text_field(row, "url")?;
                Some((network, url))
            })
            .collect()
    })
}

fn due_days(value: Option<&Value>) -> u32 {
    optional_due_days(value).unwrap_or(14)
}

fn optional_due_days(value: Option<&Value>) -> Option<u32> {
    let days = value.and_then(Value::as_u64)? as u32;
    if days == 0 {
        return None;
    }
    Some(days.clamp(1, 180))
}

fn due_unix(days: u32, now: u64) -> u64 {
    now.saturating_add(u64::from(days.clamp(1, 180)) * 86_400)
}

/// A named `due_days` wins. When the person left the date out, pick a
/// completion before the parent's review date, or two weeks when that
/// review date is missing.
fn ticket_due_at(specified: Option<u32>, parent_review: Option<u64>, now: u64) -> u64 {
    if let Some(days) = specified {
        return due_unix(days, now);
    }
    let Some(review) = parent_review.filter(|at| *at > now) else {
        return due_unix(14, now);
    };
    let remaining_days = ((review - now) / 86_400).clamp(1, 180) as u32;
    let days = (remaining_days / 2).clamp(1, 21);
    let at = due_unix(days, now);
    if at < review {
        at
    } else {
        let earlier = review.saturating_sub(86_400);
        if earlier > now {
            earlier
        } else {
            now.saturating_add(3_600)
        }
    }
}

fn clean_title(title: &str) -> Option<String> {
    let text = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() < 3 || text.chars().count() > 80 {
        return None;
    }
    Some(text)
}

/// A project or ticket brief. Long enough for a few paragraphs. The old
/// 400-character cut stopped real drafts mid-sentence.
pub(crate) const MAX_BRIEF_CHARS: usize = 4_000;

fn clean_brief(brief: &str, title: &str) -> String {
    let text = brief.trim();
    if text.chars().count() >= 3 {
        text.chars().take(MAX_BRIEF_CHARS).collect()
    } else {
        title.to_string()
    }
}

fn is_member_pubkey(value: &str) -> bool {
    let key = value.trim();
    key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Creator, the member who offered it, or a legacy ticket whose record
/// names neither (the relay still checks the creating command).
fn may_remove_ticket(speaker: &str, created_by: Option<&str>, offered_by: Option<&str>) -> bool {
    if created_by.is_some_and(|pubkey| pubkey.eq_ignore_ascii_case(speaker)) {
        return true;
    }
    if offered_by
        .is_some_and(|pubkey| is_member_pubkey(pubkey) && pubkey.eq_ignore_ascii_case(speaker))
    {
        return true;
    }
    created_by.is_none() && offered_by.is_none()
}

fn holds(item: &Item, speaker: &str) -> bool {
    matches!(item.state.as_str(), "accepted" | "in_review")
        && item
            .dri
            .as_deref()
            .is_some_and(|dri| dri.eq_ignore_ascii_case(speaker))
}

/// The parent a ticket can use. An exact title stays exact. A description
/// matches one project they hold. No description suggests the best fit.
fn place_ticket_parent<'a>(
    board: &'a Board,
    query: &str,
    title: &str,
    brief: &str,
    speaker: &str,
) -> Option<(&'a Item, ParentChoice)> {
    if !crate::project_fit::parent_unspecified(query) {
        if let Some(item) = find_item(board, query) {
            if !holds(item, speaker) {
                return None;
            }
            let exact = norm(&item.title) == norm(query) || item.id == norm(query);
            let choice = if exact {
                ParentChoice::Named
            } else {
                ParentChoice::Described
            };
            return Some((item, choice));
        }
        let held = held_work(board, speaker);
        return crate::project_fit::match_described_project(&held, query)
            .and_then(|id| board.items.get(&id))
            .map(|item| (item, ParentChoice::Described));
    }
    let projects = held_projects(board, speaker);
    crate::project_fit::suggest_project(&projects, title, brief)
        .and_then(|id| board.items.get(&id))
        .map(|item| (item, ParentChoice::Suggested))
}

/// Projects and tickets they hold. A child can sit under either.
fn held_work(board: &Board, speaker: &str) -> Vec<crate::project_fit::FitProject> {
    let mut items: Vec<_> = board
        .items
        .values()
        .filter(|item| holds(item, speaker))
        .map(|item| crate::project_fit::FitProject {
            id: item.id.clone(),
            title: item.title.clone(),
            brief: item.brief.clone(),
        })
        .collect();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items
}

fn held_projects(board: &Board, speaker: &str) -> Vec<crate::project_fit::FitProject> {
    let mut projects: Vec<_> = board
        .items
        .values()
        .filter(|item| item.parent.is_none() && holds(item, speaker))
        .map(|item| crate::project_fit::FitProject {
            id: item.id.clone(),
            title: item.title.clone(),
            brief: item.brief.clone(),
        })
        .collect();
    projects.sort_by(|left, right| left.id.cmp(&right.id));
    projects
}

/// Name the project on a ticket reply, then the suggested holder.
pub fn annotate_act_say(say: &str, act: &ResolvedAct, board: &Board) -> String {
    if let ResolvedAct::Dri { title, who, .. } = act {
        let name = board.stored_name(who);
        let say = match name.as_deref() {
            Some(name) => replace_holder_code(say.trim(), who, name),
            None => say.trim().to_string(),
        };
        return dri_holder_sentence(&say, title, name.as_deref());
    }
    let mut say = say.trim().to_string();
    if let Some(clause) = project_clause(act, board) {
        let title = ticket_parent_title(act, board).unwrap_or_default();
        let already = !title.is_empty()
            && say
                .to_ascii_lowercase()
                .contains(&title.to_ascii_lowercase());
        if !already {
            const ASKS: [&str; 2] = [
                "Want me to offer this ticket?",
                "Want me to open this ticket?",
            ];
            if let Some((ask, head)) = ASKS
                .iter()
                .find_map(|ask| say.strip_suffix(ask).map(|head| (*ask, head.trim_end())))
            {
                say = format!("{head} {clause} {ask}");
            } else {
                say = format!("{say} {clause}");
            }
        }
    }
    if let Some(pubkey) = holder_pubkey(act) {
        if let Some(name) = named_holder(act, board) {
            say = replace_holder_code(&say, pubkey, &name);
            let names_holder = matches!(
                act,
                ResolvedAct::Project { .. } | ResolvedAct::Ticket { .. }
            );
            // A ticket that is already being offered names the person in
            // the offer. "Suggested holder" is only for the draft question.
            if names_holder && !ticket_is_offered(&say) && !mentions_name(&say, &name) {
                say = format!("{say} Suggested holder: {name}.");
            }
        }
    }
    if matches!(act, ResolvedAct::Ticket { .. }) && ticket_is_offered(&say) {
        say = strip_suggested_holder(&say);
    }
    say
}

/// The reply is opening the ticket, not asking whether to offer it.
fn ticket_is_offered(say: &str) -> bool {
    let lower = say.to_ascii_lowercase();
    if lower.contains("want me to offer") || lower.contains("want me to open") {
        return false;
    }
    lower.contains("offered to ")
        || lower.starts_with("opening the ticket")
        || lower.starts_with("opening that ticket")
        || lower.starts_with("offering ")
        || lower.starts_with("offering “")
        || lower.starts_with("offering \"")
}

fn strip_suggested_holder(say: &str) -> String {
    let lower = say.to_ascii_lowercase();
    let Some(at) = lower.find("suggested holder:") else {
        return say.to_string();
    };
    let mut end = at + "suggested holder:".len();
    let rest = say[end..].chars();
    for ch in rest {
        end += ch.len_utf8();
        if ch == '.' {
            break;
        }
    }
    let head = say[..at].trim_end();
    let tail = say[end..].trim_start();
    if tail.is_empty() {
        return head.to_string();
    }
    if head.is_empty() {
        return tail.to_string();
    }
    format!("{head} {tail}")
}

/// Swap a pubkey, or the 8-character prefix written when no name was known,
/// for the person's display name.
fn replace_holder_code(say: &str, pubkey: &str, name: &str) -> String {
    let key = pubkey.trim().to_ascii_lowercase();
    if key.len() < 8 || name.is_empty() {
        return say.to_string();
    }
    let prefix: String = key.chars().take(8).collect();
    if name.eq_ignore_ascii_case(&key) || name.eq_ignore_ascii_case(&prefix) {
        return say.to_string();
    }
    let with_key = replace_hex_token(say, &key, name);
    replace_hex_token(&with_key, &prefix, name)
}

fn replace_hex_token(say: &str, token: &str, name: &str) -> String {
    let lower = say.to_ascii_lowercase();
    let token = token.to_ascii_lowercase();
    if token.is_empty() {
        return say.to_string();
    }
    let bytes = lower.as_bytes();
    let mut out = String::new();
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find(&token) {
        let at = cursor + rel;
        let end = at + token.len();
        let before_hex = at > 0 && bytes[at - 1].is_ascii_hexdigit();
        let after_hex = end < bytes.len() && bytes[end].is_ascii_hexdigit();
        if before_hex || after_hex {
            out.push_str(&say[cursor..end]);
            cursor = end;
            continue;
        }
        out.push_str(&say[cursor..at]);
        out.push_str(name);
        cursor = end;
    }
    out.push_str(&say[cursor..]);
    out
}

fn mentions_name(say: &str, name: &str) -> bool {
    say.to_ascii_lowercase()
        .contains(&name.to_ascii_lowercase())
}

/// A DRI draft names the project, and the person when their profile has a name.
/// A bare pubkey prefix is not a name, so it stays out of the sentence.
fn dri_holder_sentence(say: &str, title: &str, name: Option<&str>) -> String {
    let drafting = say.contains("Open it, then publish.");
    let opening = say.starts_with("Opening the proposal");
    if drafting {
        return match name {
            Some(name) => {
                format!("A draft to name {name} as the holder of “{title}”. Open it, then publish.")
            }
            None => format!("A draft to name a holder for “{title}”. Open it, then publish."),
        };
    }
    if opening {
        return match name {
            Some(name) => {
                format!("Opening the proposal to name {name} as the holder of “{title}”.")
            }
            None => format!("Opening the proposal to name a holder for “{title}”."),
        };
    }
    match name {
        Some(name) if !mentions_name(say, name) => format!("{say} Suggested holder: {name}."),
        _ => say.to_string(),
    }
}

fn project_clause(act: &ResolvedAct, board: &Board) -> Option<String> {
    let ResolvedAct::Ticket { parent_choice, .. } = act else {
        return None;
    };
    let title = ticket_parent_title(act, board)?;
    match parent_choice {
        ParentChoice::Suggested => Some(format!("I suggest this under {title}.")),
        ParentChoice::Described => Some(format!("Under {title}.")),
        ParentChoice::Named => None,
    }
}

fn ticket_parent_title<'a>(act: &'a ResolvedAct, board: &'a Board) -> Option<&'a str> {
    let ResolvedAct::Ticket { parent, .. } = act else {
        return None;
    };
    board.items.get(parent).map(|item| item.title.as_str())
}

/// One live item with this title. Several copies are not a reason to refuse:
/// the oldest live copy is the one this removal takes.
fn find_removable<'a>(board: &'a Board, query: &str) -> Option<&'a Item> {
    let wanted = norm(query);
    if wanted.is_empty() {
        return None;
    }
    if let Some(item) = board.items.values().find(|item| item.id == wanted) {
        return (item.state != "withdrawn").then_some(item);
    }
    let mut matches: Vec<&Item> = board
        .items
        .values()
        .filter(|item| item.state != "withdrawn" && norm(&item.title) == wanted)
        .collect();
    matches.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    matches.first().copied()
}

fn find_item<'a>(board: &'a Board, query: &str) -> Option<&'a Item> {
    let wanted = norm(query);
    if wanted.is_empty() {
        return None;
    }
    if let Some(item) = board.items.values().find(|item| item.id == wanted) {
        return Some(item);
    }
    let live: Vec<&Item> = board
        .items
        .values()
        .filter(|item| item.state != "withdrawn")
        .collect();
    let exact: Vec<&Item> = live
        .iter()
        .copied()
        .filter(|item| norm(&item.title) == wanted)
        .collect();
    if exact.len() == 1 {
        return exact.first().copied();
    }
    if !exact.is_empty() || wanted.chars().count() < 4 {
        return None;
    }
    // "finish phase 0" is the one project "Finish Phase 0 for Hypha Desktop App".
    let prefixed: Vec<&Item> = live
        .iter()
        .copied()
        .filter(|item| {
            let title = norm(&item.title);
            title.starts_with(&wanted) || wanted.starts_with(&title)
        })
        .collect();
    if prefixed.len() == 1 {
        return prefixed.first().copied();
    }
    None
}

fn resolve_who(name: &str, board: &Board, speaker: &str) -> Option<String> {
    let wanted = name.trim();
    if wanted.eq_ignore_ascii_case("me")
        || wanted.eq_ignore_ascii_case("myself")
        || wanted.eq_ignore_ascii_case("i")
        || wanted.eq_ignore_ascii_case("you")
        || wanted.eq_ignore_ascii_case("yourself")
    {
        return Some(speaker.to_string());
    }
    let key = wanted.to_ascii_lowercase();
    if key.len() == 64 && key.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Some(key);
    }
    let matches: Vec<String> = board
        .names
        .iter()
        .filter(|(_, (_, label))| label.eq_ignore_ascii_case(wanted))
        .map(|(pubkey, _)| pubkey.clone())
        .collect();
    if matches.len() == 1 {
        return matches.into_iter().next();
    }
    None
}

/// A draft sentence that already names the ticket, used when they say yes
/// and the model describes the offer without a resolvable act.
pub fn ticket_described(text: &str) -> Option<RawAct> {
    let lower = text.to_ascii_lowercase();
    let marker = "ticket called ";
    let start = lower.find(marker)?;
    let rest_at = start + marker.len();
    let rest = &text[rest_at..];
    let rest_lower = &lower[rest_at..];
    let under_at = rest_lower.find(" under ")?;
    let title = rest[..under_at].trim().trim_matches(['"', '“', '”']);
    let after_at = under_at + " under ".len();
    let after = &rest[after_at..];
    let after_lower = &rest_lower[after_at..];
    let end = [", offered", ". offered", " offered to"]
        .iter()
        .filter_map(|needle| after_lower.find(needle))
        .min()?;
    let parent = after[..end].trim().trim_matches(['"', '“', '”']);
    let title = clean_title(title)?;
    if parent.chars().count() < 3 {
        return None;
    }
    Some(RawAct::Ticket {
        parent: parent.to_string(),
        title: title.clone(),
        brief: title,
        due_days: None,
        who: "me".into(),
    })
}

/// A ticket named in the model's prose, including an "opening" sentence
/// that never carried tags.
pub fn ticket_from_say(text: &str) -> Option<RawAct> {
    ticket_described(text)
        .or_else(|| ticket_titled_under(text))
        .or_else(|| ticket_under_then_title(text))
}

/// They told the agent to create the ticket now, not merely to describe it.
pub fn asks_to_open_ticket(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("create the ticket")
        || lower.contains("create that ticket")
        || lower.contains("create this ticket")
        || lower.contains("open the ticket")
        || lower.contains("open that ticket")
        || lower.contains("make the ticket")
        || (names_self_as_holder(text) && lower.contains("ticket"))
}

/// "Put me as DRI" names the speaker as the person the ticket is offered to.
pub fn names_self_as_holder(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("put me as")
        || lower.contains("make me the dri")
        || lower.contains("make me dri")
        || lower.contains("set me as")
        || lower.contains("assign to me")
        || lower.contains("assign it to me")
        || lower.contains("offer it to me")
        || lower.contains("offered to me")
}

/// When they name themselves on the yes, the ticket is offered to them.
pub fn offer_named_self(act: ResolvedAct, speaker: &str, text: &str) -> ResolvedAct {
    if speaker.is_empty() || !names_self_as_holder(text) {
        return act;
    }
    match act {
        ResolvedAct::Ticket {
            parent,
            title,
            brief,
            due_at,
            parent_choice,
            ..
        } => ResolvedAct::Ticket {
            parent,
            title,
            brief,
            due_at,
            offer_to: Some(speaker.to_string()),
            parent_choice,
        },
        other => other,
    }
}

/// The model described a ticket in prose, or they asked to create one.
pub fn recover_ticket(user: &str, say: &str) -> Option<RawAct> {
    if let Some(act) = ticket_from_say(say) {
        return Some(claim_raw_self(act, user));
    }
    if asked_to_create_ticket(user) {
        return ticket_from_request(user);
    }
    None
}

fn asked_to_create_ticket(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("create ticket")
        || lower.contains("create a ticket")
        || lower.contains("new ticket")
        || lower.contains("open a ticket")
        || lower.contains("add a ticket")
        || lower.contains("add ticket")
}

fn claim_raw_self(act: RawAct, text: &str) -> RawAct {
    if !names_self_as_holder(text) {
        return act;
    }
    match act {
        RawAct::Ticket {
            parent,
            title,
            brief,
            due_days,
            ..
        } => RawAct::Ticket {
            parent,
            title,
            brief,
            due_days,
            who: "me".into(),
        },
        other => other,
    }
}

/// "Choose Colouring of the Site under Choose Design Framework, offered to you."
fn ticket_titled_under(text: &str) -> Option<RawAct> {
    let lower = text.to_ascii_lowercase();
    let under = lower.find(" under ")?;
    let before = text[..under].trim();
    let title_src = before
        .rsplit_once(". ")
        .map(|(_, title)| title)
        .unwrap_or(before);
    let title = plausible_ticket_title(title_src.trim().trim_matches(['"', '“', '”']))?;
    let after = &text[under + " under ".len()..];
    let after_lower = &lower[under + " under ".len()..];
    let end = [", offered", ". offered", " offered to"]
        .iter()
        .filter_map(|needle| after_lower.find(needle))
        .min()?;
    let parent = after[..end]
        .trim()
        .trim_matches(['"', '“', '”'])
        .trim_end_matches(|ch: char| matches!(ch, '.' | ','));
    if parent.chars().count() < 3 || parent.eq_ignore_ascii_case(&title) {
        return None;
    }
    Some(raw_ticket(parent, title, who_from_opening(&lower)))
}

/// "Opening this ticket for you under Parent. Title. No holder named yet."
fn ticket_under_then_title(text: &str) -> Option<RawAct> {
    let lower = text.to_ascii_lowercase();
    let under = lower.find(" under ")?;
    let after = &text[under + " under ".len()..];
    let after_lower = &lower[under + " under ".len()..];
    let end = after_lower.find(". ")?;
    let parent = after[..end].trim().trim_matches(['"', '“', '”']);
    if parent.chars().count() < 3 {
        return None;
    }
    let rest = after[end + 2..].trim();
    let title_end = rest.find(['.', '!', '?']).unwrap_or(rest.len());
    let title = plausible_ticket_title(rest[..title_end].trim())?;
    Some(raw_ticket(parent, title, who_from_opening(&lower)))
}

fn raw_ticket(parent: &str, title: String, who: String) -> RawAct {
    RawAct::Ticket {
        parent: parent.to_string(),
        title: title.clone(),
        brief: title,
        due_days: None,
        who,
    }
}

fn who_from_opening(lower: &str) -> String {
    if lower.contains("no holder")
        || lower.contains("leave it open")
        || lower.contains("open for a match")
    {
        return String::new();
    }
    if lower.contains("offered to you") || lower.contains("offered to me") {
        return "me".into();
    }
    String::new()
}

fn plausible_ticket_title(title: &str) -> Option<String> {
    let title = clean_title(title)?;
    let lower = title.to_ascii_lowercase();
    if lower.starts_with("opening ")
        || lower.starts_with("i can ")
        || lower.starts_with("i didn't")
        || lower.starts_with("want me ")
    {
        return None;
    }
    Some(title)
}

fn ticket_from_request(user: &str) -> Option<RawAct> {
    let lower = user.to_ascii_lowercase();
    let (marker, at) = if let Some(at) = lower.find("under the ") {
        ("under the ", at)
    } else if let Some(at) = lower.find("under ") {
        ("under ", at)
    } else {
        return None;
    };
    let rest = user[at + marker.len()..].trim();
    let (parent, title) = rest.split_once(',')?;
    let parent = parent
        .trim()
        .trim_end_matches(|ch: char| matches!(ch, '.' | '!' | '?'));
    let title = title
        .trim()
        .trim_start_matches("called ")
        .trim_matches(|ch: char| matches!(ch, '.' | '!' | '?'));
    let title = plausible_ticket_title(title)?;
    if parent.chars().count() < 3 {
        return None;
    }
    let who = if names_self_as_holder(user) {
        "me".into()
    } else {
        String::new()
    };
    Some(raw_ticket(parent, title, who))
}

fn norm(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn clip(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    if count <= max {
        return trimmed.to_string();
    }
    let end = trimmed
        .char_indices()
        .nth(max)
        .map(|(index, _)| index)
        .unwrap_or(trimmed.len());
    format!("{}…", trimmed[..end].trim_end())
}

fn text_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn direction_slug(value: &Value) -> Option<String> {
    match value.as_str()? {
        slug @ ("mission" | "vision" | "situation" | "objectives" | "strategy") => {
            Some(slug.to_string())
        }
        _ => None,
    }
}

fn is_direction_body(body: &str) -> bool {
    let text = body.trim();
    if text.chars().count() < 12 {
        return false;
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 3 {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    let confirmation = ["yes", "yeah", "yep", "ok", "okay", "sure"]
        .iter()
        .any(|word| lower.starts_with(word));
    if confirmation && words.len() <= 8 {
        return false;
    }
    true
}

/// A normal chat answer fits. Past this, a runaway model reply is cut on a
/// word. The relay accepts far more (256 KB).
pub(crate) const MAX_SAY_CHARS: usize = 8_000;

/// Keep `raw` when it fits. Otherwise stop on whitespace and mark the cut.
pub(crate) fn cap_say(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.chars().count() <= MAX_SAY_CHARS {
        return trimmed.to_string();
    }
    let window: String = trimmed.chars().take(MAX_SAY_CHARS).collect();
    let cut_at = window
        .rfind(char::is_whitespace)
        .filter(|index| *index >= window.len() / 2)
        .unwrap_or(window.len());
    let mut out = window[..cut_at].trim_end().to_string();
    out.push('…');
    out
}

fn strip_think(raw: &str) -> String {
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
    text
}

fn json_object(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    let fenced = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|rest| rest.trim().strip_suffix("```").unwrap_or(rest).trim())
        .unwrap_or(trimmed);
    if let Ok(value) = serde_json::from_str::<Value>(fenced) {
        if value.is_object() {
            return Some(value);
        }
    }
    let start = fenced.find('{')?;
    let end = fenced.rfind('}')?;
    if end <= start {
        return None;
    }
    let value: Value = serde_json::from_str(&fenced[start..=end]).ok()?;
    value.is_object().then_some(value)
}

/// Used by tests that want a JSON act without hand-writing the object.
#[cfg(test)]
fn act_json(act: Value) -> String {
    serde_json::json!({ "say": "Drafted.", "direction": null, "body": null, "act": act })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ME: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ADA: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const NOW: u64 = 1_700_000_000;

    fn board() -> Board {
        let mut board = Board::default();
        board.observe(
            39103,
            "relay",
            10,
            Some("shapers"),
            &json!({
                "shapers": [ME, ADA],
                "room": "room-1"
            })
            .to_string(),
        );
        board.observe(0, ME, 11, None, r#"{"name":"You"}"#);
        board.observe(0, ADA, 11, None, r#"{"name":"Ada"}"#);
        board.observe(
            39100,
            "relay",
            12,
            Some("mission"),
            r#"{"version":1,"body":"We exist so organizations can run themselves."}"#,
        );
        board.observe(
            39101,
            "relay",
            13,
            Some("item-hall"),
            &json!({
                "title": "Hall roof",
                "state": "accepted",
                "dri": ME
            })
            .to_string(),
        );
        board.observe(
            39101,
            "relay",
            14,
            Some("item-open"),
            r#"{"title":"Paint the door","state":"open"}"#,
        );
        board
    }

    #[test]
    fn the_shapers_room_is_known_from_bootstrap() {
        let board = board();
        assert_eq!(board.shapers_room(), Some("room-1"));
        assert!(board.several_shapers());
        let mut alone = board;
        alone.observe(
            39103,
            "relay",
            20,
            Some("shapers"),
            &json!({"shapers":[ME],"room":"room-1"}).to_string(),
        );
        assert_eq!(alone.shapers_room(), Some("room-1"));
        assert!(!alone.several_shapers());
        assert_eq!(Board::default().shapers_room(), None);
    }

    #[test]
    fn overview_names_direction_and_work_without_inventing() {
        let text = board().overview();
        assert!(text.contains("mission v1: We exist so organizations can run themselves."));
        assert!(text.contains("vision: not set"));
        assert!(text.contains("Hall roof"));
        assert!(text.contains("Paint the door"));
        assert!(!text.contains("invented"));
    }

    #[test]
    fn a_project_act_becomes_a_proposal_tag() {
        let raw = act_json(json!({
            "kind": "project",
            "title": "Fix the hall",
            "brief": "The roof leaks when it rains.",
            "due_days": 21,
            "who": "Ada"
        }));
        let (_, direction, _, act) = parse_model_reply(&raw);
        assert!(direction.is_none());
        let resolved =
            resolve_act(act.as_ref().expect("act"), &board(), ME, NOW).expect("resolved");
        match resolved {
            ResolvedAct::Project {
                title,
                due_at,
                suggested,
                ..
            } => {
                assert_eq!(title, "Fix the hall");
                assert_eq!(due_at, NOW + 21 * 86_400);
                assert_eq!(suggested.as_deref(), Some(ADA));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn a_project_brief_keeps_more_than_a_short_paragraph() {
        let brief = "Daily suggestions for the desktop app. ".repeat(20);
        assert!(brief.chars().count() > 400);
        assert!(brief.chars().count() < MAX_BRIEF_CHARS);
        let raw = act_json(json!({
            "kind": "project",
            "title": "Daily task suggestions",
            "brief": brief,
            "due_days": 14,
            "who": null
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved =
            resolve_act(act.as_ref().expect("act"), &board(), ME, NOW).expect("resolved");
        let ResolvedAct::Project { brief: kept, .. } = resolved else {
            panic!("unexpected {resolved:?}");
        };
        assert_eq!(kept, brief.trim());
    }

    #[test]
    fn a_runaway_brief_stops_at_the_cap() {
        let brief = "word ".repeat(2_000);
        assert!(brief.chars().count() > MAX_BRIEF_CHARS);
        let raw = act_json(json!({
            "kind": "project",
            "title": "Daily task suggestions",
            "brief": brief,
            "due_days": 14,
            "who": null
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved =
            resolve_act(act.as_ref().expect("act"), &board(), ME, NOW).expect("resolved");
        let ResolvedAct::Project { brief: kept, .. } = resolved else {
            panic!("unexpected {resolved:?}");
        };
        assert_eq!(kept.chars().count(), MAX_BRIEF_CHARS);
        assert!(brief.starts_with(&kept));
    }

    #[test]
    fn a_project_without_a_named_holder_has_nobody() {
        for who in [json!(null), json!(""), json!("nobody"), json!("none")] {
            let raw = act_json(json!({
                "kind": "project",
                "title": "Fix the hall",
                "brief": "The roof leaks when it rains.",
                "due_days": 14,
                "who": who
            }));
            let (_, _, _, act) = parse_model_reply(&raw);
            let resolved =
                resolve_act(act.as_ref().expect("act"), &board(), ME, NOW).expect("resolved");
            let ResolvedAct::Project { suggested, .. } = &resolved else {
                panic!("unexpected {resolved:?}");
            };
            assert!(suggested.is_none());
            assert!(!act_tags(&resolved, ME)
                .iter()
                .any(|tag| tag.first().is_some_and(|name| name == "p")));
        }
    }

    #[test]
    fn done_is_only_for_the_holder() {
        let raw = act_json(json!({"kind":"done","item":"Hall roof"}));
        let (_, _, _, act) = parse_model_reply(&raw);
        let act = act.expect("act");
        assert!(resolve_act(&act, &board(), ADA, NOW).is_none());
        let resolved = resolve_act(&act, &board(), ME, NOW).expect("holder");
        assert_eq!(
            resolved,
            ResolvedAct::Done {
                item: "item-hall".into()
            }
        );
    }

    #[test]
    fn a_ticket_is_offered_to_the_named_person() {
        let raw = act_json(json!({
            "kind": "ticket",
            "parent": "Hall roof",
            "title": "Order tiles",
            "brief": "Buy the replacement tiles.",
            "who": "me",
            "due_days": 7
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board(), ME, NOW).expect("ticket");
        assert_eq!(
            resolved,
            ResolvedAct::Ticket {
                parent: "item-hall".into(),
                title: "Order tiles".into(),
                brief: "Buy the replacement tiles.".into(),
                due_at: NOW + 7 * 86_400,
                offer_to: Some(ME.into()),
                parent_choice: ParentChoice::Named,
            }
        );
        assert!(resolve_act(act.as_ref().unwrap(), &board(), ADA, NOW).is_none());
    }

    #[test]
    fn a_short_project_name_matches_the_one_live_item() {
        let mut board = board();
        board.observe(
            39101,
            "relay",
            16,
            Some("item-phase"),
            &json!({
                "title": "Finish Phase 0 for Hypha Desktop App",
                "state": "accepted",
                "dri": ME
            })
            .to_string(),
        );
        let raw = act_json(json!({
            "kind": "ticket",
            "parent": "Finish Phase 0",
            "title": "Create Automated Tests",
            "brief": "Create the automated tests.",
            "who": "you"
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board, ME, NOW).expect("prefix");
        assert_eq!(
            resolved,
            ResolvedAct::Ticket {
                parent: "item-phase".into(),
                title: "Create Automated Tests".into(),
                brief: "Create the automated tests.".into(),
                due_at: NOW + 14 * 86_400,
                offer_to: Some(ME.into()),
                parent_choice: ParentChoice::Described,
            }
        );

        board.observe(
            39101,
            "relay",
            17,
            Some("item-phase-b"),
            &json!({
                "title": "Finish Phase 0 for Mobile",
                "state": "accepted",
                "dri": ME
            })
            .to_string(),
        );
        assert!(
            resolve_act(act.as_ref().unwrap(), &board, ME, NOW).is_none(),
            "two projects that start the same way are not a guess"
        );
    }

    fn hold_project(board: &mut Board, id: &str, title: &str, brief: &str, at: u64) {
        board.observe(
            39101,
            "relay",
            at,
            Some(id),
            &json!({
                "title": title,
                "brief": brief,
                "state": "accepted",
                "dri": ME
            })
            .to_string(),
        );
    }

    #[test]
    fn a_described_project_is_the_one_they_hold() {
        let mut board = board();
        hold_project(
            &mut board,
            "item-land",
            "Public site",
            "Rebuild the marketing landing page",
            30,
        );
        assert!(board.overview().contains("landing page"));
        let raw = act_json(json!({
            "kind": "ticket",
            "parent": "it is the project that has to do with landing page",
            "title": "Update the hero",
            "brief": "Refresh the homepage hero.",
            "who": "me"
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board, ME, NOW).expect("described");
        assert_eq!(
            resolved,
            ResolvedAct::Ticket {
                parent: "item-land".into(),
                title: "Update the hero".into(),
                brief: "Refresh the homepage hero.".into(),
                due_at: NOW + 14 * 86_400,
                offer_to: Some(ME.into()),
                parent_choice: ParentChoice::Described,
            }
        );
        let say = annotate_act_say(&draft_say(&resolved), &resolved, &board);
        assert!(say.contains("Under Public site."));
        assert!(say.contains("Want me to offer this ticket?"));
        assert!(resolve_act(act.as_ref().unwrap(), &board, ADA, NOW).is_none());
    }

    #[test]
    fn a_missing_project_suggests_the_one_the_ticket_fits() {
        let mut board = board();
        hold_project(
            &mut board,
            "item-land",
            "Public site",
            "Rebuild the marketing landing page",
            30,
        );
        let raw = act_json(json!({
            "kind": "ticket",
            "parent": "",
            "title": "Hero on the landing page",
            "brief": "Update the homepage hero.",
            "who": "me"
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board, ME, NOW).expect("suggested");
        match &resolved {
            ResolvedAct::Ticket {
                parent,
                parent_choice,
                ..
            } => {
                assert_eq!(parent, "item-land");
                assert_eq!(*parent_choice, ParentChoice::Suggested);
            }
            other => panic!("{other:?}"),
        }
        let say = annotate_act_say(&draft_say(&resolved), &resolved, &board);
        let suggest_at = say
            .find("I suggest this under Public site.")
            .expect("suggestion");
        let ask_at = say.find("Want me to offer this ticket?").expect("ask");
        assert!(suggest_at < ask_at);
    }

    #[test]
    fn the_only_project_they_hold_is_suggested_when_they_name_none() {
        let raw = act_json(json!({
            "kind": "ticket",
            "parent": null,
            "title": "Order tiles",
            "brief": "Buy the replacement tiles.",
            "who": "me"
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board(), ME, NOW).expect("only");
        match resolved {
            ResolvedAct::Ticket {
                parent,
                parent_choice,
                ..
            } => {
                assert_eq!(parent, "item-hall");
                assert_eq!(parent_choice, ParentChoice::Suggested);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_project_name_that_fits_nothing_is_not_swapped_for_a_better_fit() {
        let mut board = board();
        hold_project(
            &mut board,
            "item-land",
            "Public site",
            "Rebuild the marketing landing page",
            30,
        );
        let raw = act_json(json!({
            "kind": "ticket",
            "parent": "Aquarium lights",
            "title": "Hero on the landing page",
            "brief": "Update the homepage hero.",
            "who": "me"
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        assert!(resolve_act(act.as_ref().unwrap(), &board, ME, NOW).is_none());
    }

    #[test]
    fn a_described_ticket_is_the_offer_they_already_heard() {
        let sentence = "I can create a ticket called Create Automated Tests under Finish Phase 0 for Hypha Desktop App, offered to you. Want me to publish it?";
        let act = ticket_described(sentence).expect("described");
        assert_eq!(
            act,
            RawAct::Ticket {
                parent: "Finish Phase 0 for Hypha Desktop App".into(),
                title: "Create Automated Tests".into(),
                brief: "Create Automated Tests".into(),
                due_days: None,
                who: "me".into(),
            }
        );
        assert!(ticket_described("Opening the ticket Create Automated Tests.").is_none());
    }

    #[test]
    fn an_opening_sentence_still_names_the_ticket() {
        let open = "Opening this ticket for you under Choose Design Framework for Landing Page. Choose Colouring of the Site. No holder named yet, so I will leave it open for a match.";
        let act = ticket_from_say(open).expect("open");
        assert_eq!(
            act,
            RawAct::Ticket {
                parent: "Choose Design Framework for Landing Page".into(),
                title: "Choose Colouring of the Site".into(),
                brief: "Choose Colouring of the Site".into(),
                due_days: None,
                who: String::new(),
            }
        );
        let offered = "Opening that ticket for you. Choose Colouring of the Site under Choose Design Framework for Landing Page, offered to you.";
        let act = ticket_from_say(offered).expect("offered");
        match act {
            RawAct::Ticket {
                parent, title, who, ..
            } => {
                assert_eq!(parent, "Choose Design Framework for Landing Page");
                assert_eq!(title, "Choose Colouring of the Site");
                assert_eq!(who, "me");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_ticket_with_nobody_to_offer_stays_open() {
        let raw = act_json(json!({
            "kind": "ticket",
            "parent": "Hall roof",
            "title": "Order tiles",
            "brief": "Buy the replacement tiles.",
            "who": null
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board(), ME, NOW).expect("open");
        match &resolved {
            ResolvedAct::Ticket { offer_to, .. } => assert!(offer_to.is_none()),
            other => panic!("{other:?}"),
        }
        assert!(!act_tags(&resolved, ME)
            .iter()
            .any(|tag| tag.first().is_some_and(|name| name == "p")));
        assert!(draft_say(&resolved).contains("Want me to open this ticket?"));
    }

    #[test]
    fn a_described_parent_can_be_a_ticket_they_hold() {
        let mut board = board();
        board.observe(
            39101,
            "relay",
            18,
            Some("item-framework"),
            &json!({
                "title": "Choose Design Framework for Landing Page",
                "brief": "Evaluate and select a design framework.",
                "state": "in_review",
                "dri": ME,
                "parent": "item-hall"
            })
            .to_string(),
        );
        let user = "create ticket under the find design framework, choose colouring of the site";
        let act = recover_ticket(user, "Opening this ticket.").expect("request");
        let resolved = resolve_act(&act, &board, ME, NOW).expect("under the ticket");
        match resolved {
            ResolvedAct::Ticket {
                parent,
                title,
                offer_to,
                parent_choice,
                ..
            } => {
                assert_eq!(parent, "item-framework");
                assert_eq!(title, "choose colouring of the site");
                assert!(offer_to.is_none());
                assert_eq!(parent_choice, ParentChoice::Described);
            }
            other => panic!("{other:?}"),
        }
        assert!(asks_to_open_ticket("create the ticket put me as DRI"));
        assert!(!asks_to_open_ticket(user));
    }

    #[test]
    fn a_ticket_without_a_named_date_is_estimated() {
        let raw = act_json(json!({
            "kind": "ticket",
            "parent": "Hall roof",
            "title": "Order tiles",
            "brief": "Buy the replacement tiles.",
            "who": "me"
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board(), ME, NOW).expect("ticket");
        assert_eq!(
            resolved,
            ResolvedAct::Ticket {
                parent: "item-hall".into(),
                title: "Order tiles".into(),
                brief: "Buy the replacement tiles.".into(),
                due_at: NOW + 14 * 86_400,
                offer_to: Some(ME.into()),
                parent_choice: ParentChoice::Named,
            }
        );

        let mut reviewed = board();
        reviewed.observe(
            39101,
            "relay",
            40,
            Some("item-hall"),
            &json!({
                "title": "Hall roof",
                "state": "accepted",
                "dri": ME,
                "due_at": NOW + 30 * 86_400
            })
            .to_string(),
        );
        let before_review =
            resolve_act(act.as_ref().unwrap(), &reviewed, ME, NOW).expect("estimated");
        assert_eq!(
            before_review,
            ResolvedAct::Ticket {
                parent: "item-hall".into(),
                title: "Order tiles".into(),
                brief: "Buy the replacement tiles.".into(),
                due_at: NOW + 15 * 86_400,
                offer_to: Some(ME.into()),
                parent_choice: ParentChoice::Named,
            }
        );
    }

    #[test]
    fn a_dri_names_someone_only_when_nobody_holds_it() {
        let raw = act_json(json!({"kind":"dri","item":"Paint the door","who":"Ada"}));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board(), ME, NOW).expect("dri");
        let tags = act_tags(&resolved, ME);
        assert!(tags.iter().any(|tag| tag[0] == "dri" && tag[2] == ADA));
        let say = annotate_act_say(&draft_say(&resolved), &resolved, &board());
        assert_eq!(
            say,
            "A draft to name Ada as the holder of “Paint the door”. Open it, then publish."
        );
        let unknown = ResolvedAct::Dri {
            item: "item-open".into(),
            title: "Paint the door".into(),
            who: "ab".repeat(32),
        };
        let unnamed = annotate_act_say(&draft_say(&unknown), &unknown, &board());
        assert_eq!(
            unnamed,
            "A draft to name a holder for “Paint the door”. Open it, then publish."
        );
        assert!(!unnamed.contains("abababab"));
        assert!(recover_dri("which projects dont have a DRI yet?", "").is_none());
        let prose = recover_dri(
            "make a proposal to set me as DRI for paint the door",
            "Drafting a DRI proposal to set you as holder of Paint the door.",
        )
        .expect("request");
        let from_prose = resolve_act(&prose, &board(), ME, NOW).expect("resolved");
        assert_eq!(
            from_prose,
            ResolvedAct::Dri {
                item: "item-open".into(),
                title: "Paint the door".into(),
                who: ME.into(),
            }
        );
        let from_say = recover_dri(
            "ok",
            "Drafting a DRI proposal to set you as holder of Paint the door. Publish it when you are ready.",
        )
        .expect("say");
        assert_eq!(
            resolve_act(&from_say, &board(), ME, NOW).expect("from say"),
            from_prose
        );
        let held = act_json(json!({"kind":"dri","item":"Hall roof","who":"Ada"}));
        let (_, _, _, held_act) = parse_model_reply(&held);
        assert!(resolve_act(held_act.as_ref().unwrap(), &board(), ME, NOW).is_none());
    }

    #[test]
    fn a_stated_mission_is_a_draft_on_that_turn() {
        let stated = "okay I want to set a new mission, it should be Provide social organizations a operating system that would maximize their productivity.";
        let stall = "Here's a clean draft of the mission: Provide social organizations an operating system that maximizes their productivity. Want to refine the wording, or does this capture what you have in mind?";
        let (slug, body) = recover_direction(stated, stall).expect("mission draft");
        assert_eq!(slug, "mission");
        assert_eq!(
            body,
            "Provide social organizations an operating system that maximizes their productivity"
        );
        let own_words =
            recover_direction(stated, "What should the mission say?").expect("own words");
        assert_eq!(
            own_words.1,
            "Provide social organizations a operating system that would maximize their productivity"
        );
        assert!(recover_direction("what is our mission?", stall).is_none());
        assert!(recover_direction("I want to set a new mission", "What should it say?").is_none());
        assert_eq!(
            direction_reply_say(
                "The mission is published: Provide social organizations an operating system that maximizes their productivity.",
                "mission",
            ),
            direction_draft_say("mission")
        );
        assert_eq!(
            direction_reply_say("Got it. What's the vision?", "mission"),
            "Got it. What's the vision?"
        );
    }

    #[test]
    fn a_singular_objective_is_still_the_objectives_draft() {
        let user =
            "now create as a first objective start using this desktop app internally in Hypha";
        let say = "Here's a draft of the first objective: Start using this desktop app internally in Hypha. Does this capture what you mean?";
        let (slug, body) = recover_direction(user, say).expect("objective draft");
        assert_eq!(slug, "objectives");
        assert_eq!(body, "Start using this desktop app internally in Hypha");
        let lines = [
            ChatTurn {
                from_agent: false,
                content: user,
                acted: false,
            },
            ChatTurn {
                from_agent: true,
                content: say,
                acted: false,
            },
            ChatTurn {
                from_agent: false,
                content: "it is good",
                acted: false,
            },
            ChatTurn {
                from_agent: true,
                content: "Objectives so far: Start using this desktop app internally in Hypha. What's the next objective you'd like to add?",
                acted: false,
            },
        ];
        assert_eq!(
            missing_direction_draft(&lines).expect("still missing"),
            (
                "objectives".to_string(),
                "Start using this desktop app internally in Hypha".to_string()
            )
        );
        assert_eq!(
            recover_confirmed_direction(&lines, "it is good").expect("yes"),
            (
                "objectives".to_string(),
                "Start using this desktop app internally in Hypha".to_string()
            )
        );
        let mut tagged = lines;
        tagged[1].acted = true;
        assert!(missing_direction_draft(&tagged).is_none());
    }

    #[test]
    fn coaching_on_a_direction_is_not_a_draft() {
        let ask = "I want to write our mission";
        assert!(recover_direction(
            ask,
            "Good. A strong mission should be specific: it names the people you serve and what you refuse to become."
        )
        .is_none());
        assert!(recover_direction(
            ask,
            "Before I draft it: who exactly are the people you serve today?"
        )
        .is_none());
        let weak = "our mission should be to empower communities through technology";
        let verdict = "Weak: it names no one and rules nothing out. Sharper draft: Give neighbourhood food co-ops in Lisbon one place to run their weekly orders.";
        assert_eq!(
            recover_direction(weak, verdict),
            Some((
                "mission".to_string(),
                "Give neighbourhood food co-ops in Lisbon one place to run their weekly orders"
                    .to_string()
            ))
        );
        let lines = [
            ChatTurn {
                from_agent: false,
                content: weak,
                acted: false,
            },
            ChatTurn {
                from_agent: true,
                content: verdict,
                acted: true,
            },
            ChatTurn {
                from_agent: false,
                content: "yes",
                acted: false,
            },
        ];
        assert_eq!(
            recover_confirmed_direction(&lines, "yes").map(|(_, body)| body),
            Some(
                "Give neighbourhood food co-ops in Lisbon one place to run their weekly orders"
                    .to_string()
            )
        );
    }

    #[test]
    fn a_situation_is_a_direction_draft_and_shows_in_order() {
        let raw = json!({
            "say": "Here is where you stand.",
            "direction": "situation",
            "body": "A working prototype with no outside users yet. Two developers, about twenty hours a week each.",
            "act": null
        })
        .to_string();
        let (_, direction, body, act) = parse_model_reply(&raw);
        assert_eq!(direction.as_deref(), Some("situation"));
        assert!(body.expect("body").starts_with("A working prototype"));
        assert!(act.is_none());

        let mut board = Board::default();
        board.observe(
            39100,
            ME,
            10,
            Some("situation"),
            r#"{"body":"A working prototype, no outside users.","version":1}"#,
        );
        let overview = board.overview();
        let vision = overview.find("- vision").expect("vision");
        let situation = overview
            .find("- situation v1: A working prototype")
            .expect("situation");
        let objectives = overview.find("- objectives").expect("objectives");
        assert!(vision < situation && situation < objectives);

        assert!(recover_direction(
            "we are in a tough situation, set up a project: weekly stall",
            "Sure, which project?"
        )
        .is_none());

        let asked = "should we create situation draft?";
        let drafted = "Here's a draft of the situation: Hypha Buzz runs in the desktop for one builder at thirty hours a week. Direction setting works today. The other members have not set direction yet. The next unknown is whether they will take tasks from the agent. Open it, then publish.";
        let (slug, body) = recover_direction(asked, drafted).expect("situation draft");
        assert_eq!(slug, "situation");
        assert!(body.contains("thirty hours a week"));
        assert!(body.contains("next unknown"));
        assert!(!body.contains("Open it"));
        assert!(!body.contains('\n'));
        assert!(recover_direction(asked, "What stage are you in?").is_none());

        let current = "can you create draft for current situation?";
        let refused = "There's no separate draft type for where the org stands. Direction holds only mission, vision, objectives, and strategy, so I'll say it back here instead. Hypha Buzz is a desktop app that has just started: direction setting works, but the daily task suggestions your first objective depends on do not, and nobody, including you, runs their day in it yet. You build alone while one founder mostly funds the work and approaches investors he knows with no fixed date, so the one thing you must learn next is whether people actually follow the AI's suggested next task. Your published objectives and strategy already fit that stage, so nothing in direction needs to change. What's missing is work: the board is empty. The natural first project is getting daily task suggestions working well enough that you take your own next task from them by mid-October. Should I draft that as a project, and do you want a review date other than about two weeks?";
        let (slug, body) = recover_direction(current, refused).expect("refused situation");
        assert_eq!(slug, "situation");
        assert!(body.starts_with("Hypha Buzz is a desktop app"));
        assert!(body.contains("must learn next"));
        assert!(!body.contains("no separate"));
        assert!(!body.contains("first project"));
        assert!(!body.contains("board is empty"));
    }

    #[test]
    fn the_prompt_coaches_direction_in_order() {
        let prompt = system_prompt("a private DM with one member", "Overview:\nnone");
        assert!(prompt.contains("mission, vision, situation, objectives, then strategy"));
        assert!(prompt.contains("Objectives wait until you know it"));
        assert!(prompt.contains("\"situation\""));
        assert!(prompt.contains("situation is one paragraph"));
        assert!(prompt.contains("Never say there is no situation draft"));
        assert!(prompt.contains("situation is one of those five"));
        assert!(prompt.contains("strong, close, or weak"));
        assert!(prompt.contains("Sharper draft: <the text>"));
        assert!(prompt.contains("never ask what it already shows"));
        assert!(prompt.contains("Fit the stage"));
        assert!(prompt.contains("first principles"));
        assert!(!prompt.contains("walk mission, vision, objectives, then strategy"));
    }

    #[test]
    fn the_walk_waits_for_each_piece_to_be_confirmed() {
        let prompt = system_prompt("#shapers", "Overview:\nnone");
        assert!(prompt.contains("never ask about the next piece in the same reply"));
        assert!(prompt.contains("only when the overview shows this one with a version"));
        assert!(prompt.contains("Once it is confirmed, propose objectives"));
        assert!(!prompt.contains("Then propose objectives that fit it"));
        assert!(prompt.contains("When a note says a piece was just confirmed"));
        assert!(prompt.contains("Strategy is the last piece"));
        assert!(prompt.contains("never draft from it"));
        assert!(confirmed_note("vision", 1).contains("the vision was just confirmed as version 1"));
    }

    #[test]
    fn the_walk_asks_one_question_at_a_time() {
        let prompt = system_prompt("#shapers", "Overview:\nnone");
        assert!(prompt.contains("Ask one question per reply, never two"));
        assert!(prompt.contains("one question per reply, roughly in this order"));
        assert!(!prompt.contains("two or three questions"));
        assert!(!prompt.contains("with the questions that decide it"));
        assert!(prompt.contains("Never list or count the questions still to come"));
        assert!(prompt.contains("at most three short sentences before its one question"));
        assert!(prompt.contains("Every draft of them carries the whole list"));
        assert!(prompt.contains("say never repeats the drafted text or its lines"));
    }

    #[test]
    fn a_publish_announcement_is_never_recovered_as_a_draft() {
        let announced = "Opened a strategy proposal: Until the end of October 2026, we add no organizations beyond Hypha Buzz and Hypha.\n\
                         When the two objectives compete, Hypha Buzz wins: the Hypha team test waits until Vlad's own daily use is solid.";
        assert_eq!(recover_direction(announced, "Strategy confirmed."), None);
        let turns = [
            ChatTurn {
                from_agent: false,
                content: announced,
                acted: false,
            },
            ChatTurn {
                from_agent: true,
                content: "The strategy is set. Want the first project for the objectives?",
                acted: false,
            },
        ];
        assert_eq!(missing_direction_draft(&turns), None);
    }

    #[test]
    fn a_reply_does_not_repeat_the_lines_its_card_shows() {
        let body = "Until the end of October 2026, we add no organizations beyond Hypha Buzz and Hypha.\n\
                    Vlad builds alone through October; a second developer joins only if the team takes its tasks from the AI.";
        let say = "Here is the strategy draft with all four lines. \
                   First: until the end of October 2026, we add no organizations beyond Hypha Buzz and Hypha, because every hour counts. \
                   Third: Vlad builds alone through October, and a second developer joins only if the team takes its tasks from the AI. \
                   Open the draft to edit it, then publish. \
                   Would the second developer be paid?";
        assert_eq!(
            drop_restated_lines(say, "strategy", body),
            "Here is the strategy draft with all four lines. Open the draft to edit it, then publish. Would the second developer be paid?"
        );
        assert_eq!(
            drop_restated_lines(
                "Until the end of October 2026, we add no organizations beyond Hypha Buzz and Hypha.",
                "strategy",
                body
            ),
            "A draft of the strategy. Open it, then publish."
        );
    }

    #[test]
    fn a_suggested_holder_is_the_display_name() {
        let mut named = board();
        named.observe(
            0,
            ADA,
            12,
            None,
            r#"{"display_name":"Ada Lovelace","name":"ada"}"#,
        );
        let prefix: String = ADA.chars().take(8).collect();
        assert_eq!(named.stored_name(ADA).as_deref(), Some("Ada Lovelace"));
        assert!(!named.unnamed_pubkeys().iter().any(|key| key == ADA));

        let ticket = ResolvedAct::Ticket {
            parent: "item-hall".into(),
            title: "Order tiles".into(),
            brief: "Buy the replacement tiles.".into(),
            due_at: NOW,
            offer_to: Some(ADA.into()),
            parent_choice: ParentChoice::Named,
        };
        let say = annotate_act_say("Want me to offer this ticket?", &ticket, &named);
        assert!(say.contains("Suggested holder: Ada Lovelace."));
        assert!(!say.contains(&prefix));

        let copied = annotate_act_say(
            &format!("Offering that ticket. Suggested holder: {prefix}."),
            &ticket,
            &named,
        );
        assert_eq!(copied, "Offering that ticket.");
        assert!(!copied.contains(&prefix));
        let opening = annotate_act_say(
            "Opening the ticket Order tiles under Hall roof, offered to you, due in 7 days. Suggested holder: 977a1ce1.",
            &ticket,
            &named,
        );
        assert_eq!(
            opening,
            "Opening the ticket Order tiles under Hall roof, offered to you, due in 7 days."
        );
        assert!(!opening.to_ascii_lowercase().contains("suggested holder"));

        let project = ResolvedAct::Project {
            title: "Hall roof".into(),
            brief: "Fix the roof.".into(),
            due_at: NOW,
            suggested: Some(ADA.into()),
        };
        let project_say = annotate_act_say("A draft for the hall.", &project, &named);
        assert!(project_say.contains("Suggested holder: Ada Lovelace."));
        assert!(!project_say.contains(&prefix));

        let add = ResolvedAct::Shapers {
            op: "add".into(),
            who: ADA.into(),
            why: "They shape.".into(),
        };
        let shaper_say = annotate_act_say(
            &format!("A draft to add {prefix} as a Shaper. Open it, then publish."),
            &add,
            &named,
        );
        assert!(shaper_say.contains("Ada Lovelace"));
        assert!(!shaper_say.contains(&prefix));

        let bare = Board::default();
        let unnamed = annotate_act_say("Want me to offer this ticket?", &ticket, &bare);
        assert!(!unnamed.contains("Suggested holder"));
        assert!(!unnamed.contains(&prefix));

        let mut display_only = Board::default();
        display_only.observe(0, ADA, 1, None, r#"{"display_name":"Ada Lovelace"}"#);
        assert_eq!(
            display_only.stored_name(ADA).as_deref(),
            Some("Ada Lovelace")
        );
    }

    #[test]
    fn removing_a_project_is_a_proposal_when_several_shapers_sit() {
        let raw = act_json(json!({"kind":"remove","item":"Hall roof"}));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board(), ME, NOW).expect("proposal");
        assert_eq!(
            resolved,
            ResolvedAct::RemoveProject {
                item: "item-hall".into(),
                title: "Hall roof".into(),
            }
        );
        let tags = act_tags(&resolved, ME);
        assert!(tags.iter().any(|tag| tag
            == &vec![
                "remove".to_string(),
                "item-hall".to_string(),
                "proposal".to_string()
            ]));
        assert!(resolve_act(
            act.as_ref().unwrap(),
            &board(),
            "cc".repeat(32).as_str(),
            NOW
        )
        .is_none());

        let mut alone = board();
        alone.observe(
            39103,
            "relay",
            20,
            Some("shapers"),
            &json!({"shapers":[ME],"room":"room-1"}).to_string(),
        );
        let direct = resolve_act(act.as_ref().unwrap(), &alone, ME, NOW).expect("proposal");
        assert_eq!(
            direct,
            ResolvedAct::RemoveProject {
                item: "item-hall".into(),
                title: "Hall roof".into(),
            }
        );
        assert!(act_tags(&direct, ME).iter().any(|tag| tag
            == &vec![
                "remove".to_string(),
                "item-hall".to_string(),
                "proposal".to_string()
            ]));
    }

    #[test]
    fn removing_a_ticket_is_only_the_creator_or_the_offerer() {
        let mut board = board();
        board.observe(
            39101,
            "relay",
            15,
            Some("item-tiles"),
            &json!({
                "title": "Order tiles",
                "parent": "item-hall",
                "state": "offered",
                "created_by": ME,
                "offered_by_member": ADA
            })
            .to_string(),
        );
        let raw = act_json(json!({"kind":"remove","item":"Order tiles"}));
        let (_, _, _, act) = parse_model_reply(&raw);
        let act = act.as_ref().unwrap();
        assert!(matches!(
            resolve_act(act, &board, ME, NOW),
            Some(ResolvedAct::Remove { .. })
        ));
        assert!(matches!(
            resolve_act(act, &board, ADA, NOW),
            Some(ResolvedAct::Remove { .. })
        ));
        assert!(resolve_act(act, &board, "cc".repeat(32).as_str(), NOW).is_none());
    }

    #[test]
    fn removing_a_duplicated_title_takes_the_oldest_copy() {
        let mut board = board();
        board.observe(
            39101,
            "relay",
            40,
            Some("item-older"),
            r#"{"title":"Dogfooding","state":"open"}"#,
        );
        board.observe(
            39101,
            "relay",
            50,
            Some("item-newer"),
            r#"{"title":"Dogfooding","state":"open"}"#,
        );
        let raw = act_json(json!({"kind":"remove","item":"Dogfooding"}));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().unwrap(), &board, ME, NOW).expect("one copy");
        assert_eq!(
            resolved,
            ResolvedAct::RemoveProject {
                item: "item-older".into(),
                title: "Dogfooding".into(),
            }
        );
        assert!(agrees_to_remove("remove"));
        assert!(agrees_to_remove("go"));
        assert!(!agrees_to_publish("remove"));
        assert!(!agrees_to_publish("go"));
    }

    #[test]
    fn two_items_with_the_same_title_do_not_resolve() {
        let mut board = board();
        board.observe(
            39101,
            "relay",
            30,
            Some("item-other"),
            r#"{"title":"Hall roof","state":"open"}"#,
        );
        let raw = act_json(json!({"kind":"done","item":"Hall roof"}));
        let (_, _, _, act) = parse_model_reply(&raw);
        assert!(resolve_act(act.as_ref().unwrap(), &board, ME, NOW).is_none());
    }

    #[test]
    fn a_description_is_not_permission_to_publish() {
        assert!(!agrees_to_publish(
            "Project is to use this app internally in Hypha, think of a description"
        ));
        assert!(agrees_to_publish("yes"));
        assert!(agrees_to_publish("publish it"));
        assert!(agrees_to_publish("yes publish it"));
        assert!(agrees_to_publish("assign to me"));
        assert!(!agrees_to_publish("I want to be DRI of that project"));
    }

    #[test]
    fn a_publish_question_still_opens_the_draft() {
        let say = "Build the Best Website. Design and build the best website for the organization. Want me to publish this proposal?";
        let act =
            recover_project("create a new project build the best website", say).expect("project");
        let resolved = resolve_act(&act, &board(), ME, NOW).expect("resolved");
        let tags = act_tags(&resolved, ME);
        assert!(tags.iter().any(|tag| {
            tag.len() >= 2 && tag[0] == "project" && tag[1] == "Build the Best Website"
        }));
        assert!(draft_say(&resolved).contains("Open the draft"));
        assert!(recover_project("what is a project?", "A project is a piece of work.").is_none());
    }

    #[test]
    fn a_held_project_shows_its_title_and_brief() {
        let say = draft_say(&ResolvedAct::Project {
            title: "Internal Dogfooding at Hypha".into(),
            brief: "Use the app inside Hypha first.".into(),
            due_at: 1,
            suggested: None,
        });
        assert!(say.contains("Internal Dogfooding at Hypha"));
        assert!(say.contains("Use the app inside Hypha first."));
        assert!(say.contains("Open the draft"));
    }

    #[test]
    fn an_open_proposal_can_be_revised_from_chat() {
        let mut board = board();
        let id = "33333333-3333-4333-8333-333333333333";
        board.observe(
            39102,
            "relay",
            20,
            Some(id),
            &json!({
                "id": id,
                "kind": "direction",
                "status": "open",
                "payload": { "slug": "mission", "base": 1, "body": "We exist so organizations can run themselves." }
            })
            .to_string(),
        );
        assert!(board
            .overview()
            .contains(&format!("direction mission id={id}")));
        let raw = act_json(json!({
            "kind": "revise",
            "proposal": id,
            "slug": "mission",
            "body": "We exist so organizations can run themselves, and keep a garden."
        }));
        let (_, direction, _, act) = parse_model_reply(&raw);
        assert!(direction.is_none());
        let resolved = resolve_act(act.as_ref().unwrap(), &board, ME, NOW).expect("revise");
        let tags = act_tags(&resolved, ME);
        assert_eq!(
            tags.iter()
                .find(|tag| tag[0] == "revise")
                .map(|tag| tag[1].as_str()),
            Some(id)
        );
        assert!(tags.iter().any(|tag| tag[0] == "base" && tag[1] == "1"));
        assert!(asks_for_proposal("create a proposal for the mission"));
        assert!(!asks_for_proposal("add a garden"));
    }

    #[test]
    fn rules_and_the_org_agent_are_drafts() {
        let rules = act_json(json!({"kind":"rules","shapers":"all","decision_days":7}));
        let (_, _, _, act) = parse_model_reply(&rules);
        let resolved = resolve_act(act.as_ref().unwrap(), &board(), ME, NOW).expect("rules");
        let ResolvedAct::Rules {
            rules_json,
            decision_window_secs,
            ..
        } = &resolved
        else {
            panic!("rules");
        };
        assert!(rules_json.contains("\"shapers\":\"all\""));
        assert!(rules_json.contains("\"direction\":\"majority\""));
        assert_eq!(*decision_window_secs, Some(7 * 86_400));
        let tags = act_tags(&resolved, ME);
        let rules_tag = tags
            .iter()
            .find(|tag| tag.first().is_some_and(|name| name == "shapers"));
        let rules_tag = rules_tag.expect("rules tag");
        assert_eq!(rules_tag[1], "rules");
        assert!(rules_tag[2].contains("\"shapers\":\"all\""));
        assert!(draft_say(&resolved).contains("Open it"));

        let hosted = act_json(json!({"kind":"agent","who":"hosted","why":"Back to Hypha."}));
        let (_, _, _, hosted_act) = parse_model_reply(&hosted);
        let hosted_resolved =
            resolve_act(hosted_act.as_ref().unwrap(), &board(), ME, NOW).expect("hosted");
        assert_eq!(
            hosted_resolved,
            ResolvedAct::Agent {
                pubkey: None,
                why: "Back to Hypha.".into(),
            }
        );

        let mut named_board = board();
        const BO: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
        named_board.observe(0, BO, 11, None, r#"{"name":"Bo"}"#);
        let named = act_json(json!({"kind":"agent","who":"Bo","why":"Bo runs it."}));
        let (_, _, _, named_act) = parse_model_reply(&named);
        let named_resolved =
            resolve_act(named_act.as_ref().unwrap(), &named_board, ME, NOW).expect("named");
        assert_eq!(
            named_resolved,
            ResolvedAct::Agent {
                pubkey: Some(BO.into()),
                why: "Bo runs it.".into(),
            }
        );
        assert!(resolve_act(named_act.as_ref().unwrap(), &board(), ADA, NOW).is_none());
        let seated = act_json(json!({"kind":"agent","who":"Ada","why":"Ada runs it."}));
        let (_, _, _, seated_act) = parse_model_reply(&seated);
        assert!(resolve_act(seated_act.as_ref().unwrap(), &board(), ME, NOW).is_none());
    }

    #[test]
    fn the_prompt_lists_the_chat_acts() {
        let prompt = system_prompt("a private DM with one member", "Overview:\nnone");
        assert!(prompt.contains("project proposal"));
        assert!(prompt.contains("marking their ticket done"));
        assert!(prompt.contains("naming a DRI"));
        assert!(prompt.contains("removing a project"));
        assert!(prompt.contains("social links"));
        assert!(prompt.contains("nothing you draft is real"));
        assert!(prompt.contains("finish the answer"));
        assert!(prompt.contains("as soon as they state it"));
        assert!(prompt.contains("Draft each one on the turn they state it"));
        assert!(!prompt.contains("only when they have just settled"));
        assert!(prompt.contains("describe the project in their own words"));
        assert!(prompt.contains("Say which project you suggest"));
        assert!(!prompt.contains("one or two short"));
    }

    #[test]
    fn a_chat_reply_past_a_few_sentences_is_kept() {
        let say = "Based on the overview, here's what I see. ".repeat(20);
        assert!(say.chars().count() > 360);
        assert!(say.chars().count() < MAX_SAY_CHARS);
        let raw = format!(
            r#"{{"say":{say_json},"direction":null,"act":null}}"#,
            say_json = serde_json::to_string(&say).expect("json")
        );
        let (parsed, direction, _, act) = parse_model_reply(&raw);
        assert_eq!(parsed, say.trim());
        assert_eq!(direction, None);
        assert!(act.is_none());
    }

    #[test]
    fn a_runaway_reply_stops_on_a_word() {
        let huge = "overview ".repeat(2_000);
        assert!(huge.chars().count() > MAX_SAY_CHARS);
        let (parsed, _, _, _) = parse_model_reply(&huge);
        assert!(parsed.ends_with('…'));
        let body = parsed.trim_end_matches('…').trim_end();
        assert!(body.ends_with("overview"));
        assert!(body.chars().count() <= MAX_SAY_CHARS);
        assert!(!body.contains("overvie…") && !parsed.contains("overvie…"));
    }

    #[test]
    fn a_project_with_no_named_holder_suggests_the_member_who_fits() {
        let mut live = board();
        const LEA: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
        live.observe(0, LEA, 4, None, r#"{"name":"Lea"}"#);
        live.observe(
            39105,
            "relay",
            5,
            Some(LEA),
            r#"{"about":"I write grants.","skills":[{"slug":"grant-writing","label":"grant writing"}],"open_limit":3}"#,
        );
        live.observe(
            39105,
            "relay",
            5,
            Some(ADA),
            r#"{"about":"I cook.","skills":[{"slug":"cooking","label":"cooking"}],"open_limit":1}"#,
        );
        let raw = act_json(json!({
            "kind": "project",
            "title": "Write the grant",
            "brief": "The community needs a grant.",
            "due_days": 14,
            "who": null
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().expect("act"), &live, ME, NOW).expect("resolved");
        match resolved {
            ResolvedAct::Project { suggested, .. } => assert_eq!(suggested.as_deref(), Some(LEA)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_profile_act_keeps_fields_they_did_not_change() {
        let mut live = board();
        live.observe(
            39105,
            "relay",
            5,
            Some(ME),
            r#"{"about":"I wire halls.","skills":[{"label":"electrics"}],"open_limit":2}"#,
        );
        let raw = act_json(json!({
            "kind": "profile",
            "about": null,
            "skills": null,
            "socials": [{"network":"github","url":"https://github.com/travolta"}]
        }));
        let (_, _, _, act) = parse_model_reply(&raw);
        let resolved = resolve_act(act.as_ref().expect("act"), &live, ME, NOW).expect("resolved");
        match resolved {
            ResolvedAct::Profile {
                about,
                skills,
                socials,
                open_limit,
            } => {
                assert_eq!(about, "I wire halls.");
                assert_eq!(skills, vec!["electrics".to_string()]);
                assert_eq!(
                    socials,
                    vec![(
                        "github".to_string(),
                        "https://github.com/travolta".to_string()
                    )]
                );
                assert_eq!(open_limit, Some(2));
            }
            other => panic!("{other:?}"),
        }
        let tags = act_tags(
            &resolve_act(act.as_ref().unwrap(), &live, ME, NOW).unwrap(),
            ME,
        );
        assert_eq!(tags[0], vec!["from".to_string(), ME.to_string()]);
        assert_eq!(tags[1][0], "profile");
        assert_eq!(tags[1][1], "I wire halls.");
        assert!(tags.iter().any(|tag| tag[0] == "limit" && tag[1] == "2"));
    }
}
