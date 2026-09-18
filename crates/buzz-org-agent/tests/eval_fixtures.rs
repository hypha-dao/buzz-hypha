//! Every E-1 fixture loads, verifies, decodes as its `buzz_core::intelligent_org`
//! type with the Protocol §4 tag set, and round-trips through that type;
//! the seeds' counts match the Prototype map's tables; the locale seeds
//! share ids with `en`; the sequence and who-is-needed fixtures say what
//! AI evaluation § Test data asks of them.
//!
//! Development plan E-1 "Proves": *the loader applies each seed without
//! error; counts match the map's tables.* A-1's `OrgState::apply` is not
//! here yet, so "applies" is the loader's `roundtrip` over every event.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use buzz_core::intelligent_org::{
    DraftKind, DraftOutcomeStatus, DraftPayload, ProposalKind, ProposalStatus, WorkItem,
    WorkItemState,
};
use buzz_core::kind::{
    KIND_IO_DONE, KIND_IO_DRAFT, KIND_IO_DRAFT_OUTCOME, KIND_IO_PROFILE, KIND_IO_PROPOSAL,
    KIND_IO_SHAPERS, KIND_IO_WORK_ITEM, KIND_NIP43_MEMBERSHIP_LIST,
};
use buzz_core::{verify_event, Event};
use buzz_org_agent::fixtures::{
    self, load_org, load_sequence, load_who_is_needed, org_names, roundtrip, sequence_names,
    Decoded, OrgFixture, SequenceFixture,
};
use serde_json::Value;

const KIND_CHAT: u32 = 9;

fn kind(e: &Event) -> u32 {
    u32::from(e.kind.as_u16())
}

fn tag(e: &Event, name: &str) -> Option<String> {
    e.tags
        .iter()
        .map(|t| t.as_slice())
        .find(|t| t.len() >= 2 && t[0] == name)
        .map(|t| t[1].clone())
}

/// Every event list a fixture ships, by path — seeds in every locale, health
/// gold, and every sequence delta.
fn every_event_list() -> Vec<(String, Vec<Event>)> {
    let mut out = Vec::new();
    for org in org_names().expect("orgs/") {
        let fixture = load_org(&org).expect(&org);
        for (locale, events) in &fixture.seeds {
            out.push((format!("orgs/{org}/seed[{locale}]"), events.clone()));
        }
        for (locale, events) in &fixture.health {
            out.push((format!("orgs/{org}/health-gold[{locale}]"), events.clone()));
        }
    }
    for name in sequence_names().expect("sequences/") {
        let seq = load_sequence(&name).expect(&name);
        for (file, events) in &seq.deltas {
            out.push((format!("sequences/{name}/{file}"), events.clone()));
        }
    }
    out
}

/// The newest event per `(kind, d)` — what the relay serves for a
/// parameterized-replaceable kind — decoded.
fn latest_state(events: &[Event]) -> BTreeMap<(u32, String), (Event, Decoded)> {
    let mut out = BTreeMap::new();
    for e in events {
        let k = kind(e);
        if (39000..40000).contains(&k) {
            let d = tag(e, "d").unwrap_or_default();
            let decoded = roundtrip(e).expect("state event decodes");
            out.insert((k, d), (e.clone(), decoded));
        }
    }
    out
}

fn items(state: &BTreeMap<(u32, String), (Event, Decoded)>) -> BTreeMap<String, WorkItem> {
    state
        .values()
        .filter_map(|(_, d)| match d {
            Decoded::WorkItem(w) => Some((w.id.clone(), w.clone())),
            _ => None,
        })
        .collect()
}

// ── every event ──────────────────────────────────────────────────────────────

#[test]
fn every_fixture_event_verifies_decodes_and_round_trips() {
    let lists = every_event_list();
    assert!(
        lists.len() >= 3 + 2 + 4 * 4,
        "fixture files present: {}",
        lists.len()
    );
    let mut total = 0usize;
    let mut kinds = BTreeSet::new();
    for (path, events) in &lists {
        assert!(!events.is_empty(), "{path} is empty");
        let mut seen = BTreeSet::new();
        for e in events {
            verify_event(e).unwrap_or_else(|err| panic!("{path}: {} — {err}", e.id.to_hex()));
            assert!(
                seen.insert(e.id),
                "{path}: duplicate event {}",
                e.id.to_hex()
            );
            roundtrip(e).unwrap_or_else(|err| panic!("{path}: {err}"));
            kinds.insert(kind(e));
            total += 1;
        }
    }
    assert!(total > 1_000, "the fixtures hold {total} events");
    for k in [
        39100,
        39101,
        39102,
        39103,
        39104,
        39105,
        50001,
        50002,
        50003,
        50004,
        50005,
        50006,
        50007,
        50009,
        50019,
        50021,
        50100,
        50101,
        KIND_CHAT,
        KIND_NIP43_MEMBERSHIP_LIST,
    ] {
        assert!(kinds.contains(&k), "no fixture carries kind {k}");
    }
}

#[test]
fn every_event_list_is_in_relay_order_with_the_seed_first() {
    for (path, events) in every_event_list() {
        for pair in events.windows(2) {
            assert!(
                pair[0].created_at <= pair[1].created_at,
                "{path}: {} ({}) is after {} ({})",
                pair[0].id.to_hex(),
                pair[0].created_at,
                pair[1].id.to_hex(),
                pair[1].created_at
            );
        }
    }
    for name in sequence_names().expect("sequences/") {
        let seq = load_sequence(&name).expect(&name);
        for stage in seq.case.snapshots.keys() {
            let snapshot = seq.snapshot(stage).expect(stage);
            let seed_end = seq.seed.last().map(|e| e.created_at).expect("seed");
            for e in &snapshot[seq.seed.len()..] {
                assert!(
                    e.created_at >= seed_end,
                    "{name}/{stage}: delta precedes the seed"
                );
            }
        }
    }
}

#[test]
fn state_events_are_signed_by_the_relay_and_drafts_by_the_agent() {
    for org in org_names().expect("orgs/") {
        let fixture = load_org(&org).expect(&org);
        let m = &fixture.manifest;
        for e in fixture.seeds.values().flatten() {
            let k = kind(e);
            let signer = e.pubkey.to_hex();
            if (39100..39106).contains(&k)
                || k == KIND_NIP43_MEMBERSHIP_LIST
                || (39000..39003).contains(&k)
            {
                assert_eq!(
                    signer,
                    m.relay,
                    "{org}: kind {k} {} is not relay-signed",
                    e.id.to_hex()
                );
            } else if k == KIND_IO_DRAFT || k == 50101 {
                assert_eq!(
                    signer,
                    m.agent,
                    "{org}: kind {k} {} is not agent-signed",
                    e.id.to_hex()
                );
            } else if (50001..50022).contains(&k) {
                assert!(
                    m.people.values().any(|p| *p == signer) || signer == m.agent,
                    "{org}: command {k} {} signed by a stranger {signer}",
                    e.id.to_hex()
                );
            }
        }
    }
}

// ── seeds: the Prototype map's tables ────────────────────────────────────────

struct Tally {
    roots: HashMap<WorkItemState, usize>,
    tickets: HashMap<WorkItemState, usize>,
    depth: u32,
    proposals: HashMap<(ProposalKind, ProposalStatus), usize>,
    shapers: usize,
    members: usize,
    profiles: usize,
    drafts_accepted: usize,
}

fn tally(fixture: &OrgFixture) -> Tally {
    let seed = &fixture.seeds["en"];
    let state = latest_state(seed);
    let mut t = Tally {
        roots: HashMap::new(),
        tickets: HashMap::new(),
        depth: 0,
        proposals: HashMap::new(),
        shapers: 0,
        members: 0,
        profiles: 0,
        drafts_accepted: 0,
    };
    for (_, d) in state.values() {
        match d {
            Decoded::WorkItem(w) => {
                let bag = if w.parent.is_none() {
                    &mut t.roots
                } else {
                    &mut t.tickets
                };
                *bag.entry(w.state).or_default() += 1;
                t.depth = t.depth.max(w.depth);
            }
            Decoded::Proposal(p) => *t.proposals.entry((p.kind, p.status)).or_default() += 1,
            Decoded::Shapers(s) => t.shapers = s.shapers.len(),
            Decoded::Profile(_) => t.profiles += 1,
            Decoded::DraftOutcome(o) if o.status == DraftOutcomeStatus::Accepted => {
                t.drafts_accepted += 1
            }
            _ => {}
        }
    }
    let membership = seed
        .iter()
        .rev()
        .find(|e| kind(e) == KIND_NIP43_MEMBERSHIP_LIST)
        .expect("a membership list");
    t.members = membership
        .tags
        .iter()
        .filter(|t| t.as_slice().first().map(String::as_str) == Some("member"))
        .count();
    t
}

fn count(bag: &HashMap<WorkItemState, usize>, s: WorkItemState) -> usize {
    bag.get(&s).copied().unwrap_or(0)
}

fn proposals(t: &Tally, kind: ProposalKind) -> HashMap<ProposalStatus, usize> {
    t.proposals
        .iter()
        .filter(|((k, _), _)| *k == kind)
        .map(|((_, s), n)| (*s, *n))
        .collect()
}

/// Prototype map § Data: River has 5 projects (`stall`, `weekday`, `growers`,
/// `currency`, `harvest`), 3 live tickets (`covers`, `setup`, `prices`), 14
/// seed proposals, 5 threads, 2 Shapers, 7 members. §3: `approved && dri` →
/// a held `39101`; `!approved` → an open `39102 t=project` and no `39101`;
/// money/join proposals dropped; direction history → passed `39102`s.
#[test]
fn river_seed_counts_match_the_prototype_map() {
    let fixture = load_org("river").expect("river");
    let t = tally(&fixture);
    let roots: usize = t.roots.values().sum();
    let open_projects = proposals(&t, ProposalKind::Project);
    assert_eq!(roots, 3, "stall, growers, currency are approved");
    assert_eq!(
        open_projects.get(&ProposalStatus::Open),
        Some(&2),
        "weekday and harvest are open proposals"
    );
    assert_eq!(open_projects.get(&ProposalStatus::Passed), Some(&3));
    assert_eq!(
        roots + open_projects[&ProposalStatus::Open],
        5,
        "five projects"
    );
    // Every approved River project has a DRI: held roots only, the stall in review.
    assert_eq!(count(&t.roots, WorkItemState::Open), 0);
    assert_eq!(
        count(&t.roots, WorkItemState::Accepted) + count(&t.roots, WorkItemState::InReview),
        3
    );
    // The tree: 15 `tickets[]` rows + 3 live tickets + `pricesChildren` (2).
    let tickets: usize = t.tickets.values().sum();
    assert_eq!(tickets, 20);
    assert_eq!(count(&t.tickets, WorkItemState::Done), 9, "done rows");
    assert_eq!(
        count(&t.tickets, WorkItemState::Accepted),
        9,
        "doing rows → offered + accepted"
    );
    assert_eq!(count(&t.tickets, WorkItemState::Open), 2, "open rows");
    assert_eq!(
        count(&t.tickets, WorkItemState::Offered),
        0,
        "no waiting rows in River"
    );
    // 14 `seedProposals`: 3 project + 4 direction kept; 5 money + 2 join dropped.
    // Direction history adds the versions before the head; Shapers adds the seats.
    assert_eq!(
        proposals(&t, ProposalKind::Direction)[&ProposalStatus::Passed],
        9
    );
    assert_eq!(
        proposals(&t, ProposalKind::Shapers)[&ProposalStatus::Passed],
        2
    );
    assert!(
        proposals(&t, ProposalKind::Money).is_empty(),
        "money proposals are dropped"
    );
    assert!(
        proposals(&t, ProposalKind::Join).is_empty(),
        "join proposals are dropped"
    );
    assert_eq!(t.shapers, 2, "Maya and Sam");
    // `space.members` (7) plus Rafi (HOLDERS), You (the reader), Eli (a persona), and the agent.
    assert_eq!(t.members, 11);
    assert_eq!(fixture.manifest.people.len(), 10);
    for name in ["Maya", "Sam", "Lea", "Jun", "Noor", "Tom", "Priya"] {
        assert!(
            fixture.manifest.people.contains_key(name),
            "{name} is a member"
        );
    }
    // 5 threads + #shapers + the two DMs the seed messages need; the rest are project homes.
    assert!(
        fixture.manifest.rooms.len() >= 7,
        "rooms: {}",
        fixture.manifest.rooms.len()
    );
    assert_eq!(fixture.manifest.rooms.values().filter(|r| r.dm).count(), 2);
    assert_eq!(fixture.manifest.locales, vec!["en", "pt"]);
    assert_eq!(fixture.manifest.direction.len(), 4);
    assert!(t.profiles >= 3, "Rafi, Priya, Lea have profiles");
    assert!(
        t.drafts_accepted >= 3,
        "agent-drafted rows carry a 50100 + 39104 accepted"
    );
}

/// Prototype map § Data: Energy has 6 projects (`iberia`, `ems`, `islands`,
/// `carbon`, `playbook`, `hardware`), a tree up to six levels, 3 Shapers.
#[test]
fn energy_seed_counts_match_the_prototype_map() {
    let fixture = load_org("energy").expect("energy");
    let t = tally(&fixture);
    let roots: usize = t.roots.values().sum();
    let open_projects = proposals(&t, ProposalKind::Project);
    assert_eq!(roots, 4, "iberia, ems, islands, playbook are approved");
    assert_eq!(
        open_projects.get(&ProposalStatus::Open),
        Some(&2),
        "carbon and hardware are open proposals"
    );
    assert_eq!(
        roots + open_projects[&ProposalStatus::Open],
        6,
        "six projects"
    );
    assert_eq!(t.depth, 6, "a tree up to six levels");
    // 30 `tickets[]` rows + 2 live tickets + `e-muni`'s children (2).
    let tickets: usize = t.tickets.values().sum();
    assert_eq!(tickets, 34);
    assert_eq!(count(&t.tickets, WorkItemState::Done), 13);
    assert_eq!(count(&t.tickets, WorkItemState::Accepted), 15);
    assert_eq!(count(&t.tickets, WorkItemState::Open), 4);
    assert_eq!(
        count(&t.tickets, WorkItemState::Offered),
        2,
        "waiting rows → offered"
    );
    assert_eq!(
        proposals(&t, ProposalKind::Direction)[&ProposalStatus::Passed],
        9
    );
    assert!(
        proposals(&t, ProposalKind::Money).is_empty(),
        "money proposals are dropped"
    );
    assert_eq!(t.shapers, 3);
    assert_eq!(fixture.manifest.locales, vec!["en", "es"]);
    assert_eq!(fixture.manifest.direction.len(), 4);
}

/// Prototype map §3: cold start is one founder, four `39100` at version 1,
/// one `39103`, an empty tree, no profiles.
#[test]
fn cold_seed_is_one_founder_four_artifacts_and_an_empty_tree() {
    let fixture = load_org("cold").expect("cold");
    let t = tally(&fixture);
    assert!(t.roots.is_empty() && t.tickets.is_empty(), "empty tree");
    assert_eq!(t.shapers, 1);
    assert_eq!(t.members, 2, "the founder and the agent");
    assert_eq!(t.profiles, 0);
    let state = latest_state(&fixture.seeds["en"]);
    let heads: Vec<u32> = state
        .values()
        .filter_map(|(_, d)| match d {
            Decoded::Direction(a) => Some(a.version),
            _ => None,
        })
        .collect();
    assert_eq!(heads, vec![1, 1, 1, 1]);
    assert_eq!(fixture.manifest.locales, vec!["en"]);
    assert!(fixture.health.is_empty());
}

/// Manifest `counts` is the `en` seed's per-kind tally — the number a case
/// quotes without re-counting.
#[test]
fn manifest_counts_match_the_en_seed() {
    for org in org_names().expect("orgs/") {
        let fixture = load_org(&org).expect(&org);
        let mut counts: BTreeMap<String, u32> = BTreeMap::new();
        for e in &fixture.seeds["en"] {
            *counts.entry(kind(e).to_string()).or_default() += 1;
        }
        assert_eq!(counts, fixture.manifest.counts, "{org}");
        let state = latest_state(&fixture.seeds["en"]);
        for (key, id) in &fixture.manifest.items {
            assert!(
                state.contains_key(&(KIND_IO_WORK_ITEM, id.clone())),
                "{org}: item {key} → {id}"
            );
        }
        for (key, p) in &fixture.manifest.proposals {
            let (_, d) = &state[&(KIND_IO_PROPOSAL, p.id.clone())];
            let Decoded::Proposal(got) = d else {
                panic!("{org}: {key}")
            };
            assert_eq!(
                serde_json::to_value(got.status).unwrap(),
                Value::String(p.status.clone()),
                "{org}: {key}"
            );
        }
        let (_, shapers) = &state[&(KIND_IO_SHAPERS, "shapers".to_owned())];
        let Decoded::Shapers(s) = shapers else {
            panic!("{org}: shapers")
        };
        assert_eq!(s.shapers, fixture.manifest.shapers, "{org}: Shaper set");
        assert_eq!(s.agent.as_deref(), Some(fixture.manifest.agent.as_str()));
        assert!(s.agent_hosted);
    }
}

/// Health gold: one `50101` per project with `health` in `data.ts`, on a
/// live root, `band` from `pct` — the gold for move 4, never state.
#[test]
fn health_gold_names_live_roots() {
    for org in ["river", "energy"] {
        let fixture = load_org(org).expect(org);
        let items = items(&latest_state(&fixture.seeds["en"]));
        let gold = &fixture.health["en"];
        assert!(!gold.is_empty(), "{org} has health gold");
        for e in gold {
            let Decoded::Health(h) = roundtrip(e).expect("50101") else {
                panic!("not health")
            };
            let root = &items[&h.item];
            assert!(root.parent.is_none(), "{org}: health on a root");
            assert_ne!(root.state, WorkItemState::Done);
            assert!(
                h.factors.is_empty(),
                "{org}: factors are agent output, not seeded"
            );
            assert!(
                !h.sentences.is_empty(),
                "{org}: `text` became the sentences"
            );
            assert!(
                h.sentences.iter().all(|s| s.rows.is_empty()),
                "{org}: rows are agent output"
            );
            assert!((0.0..=1.0).contains(&h.pct));
        }
        assert!(
            !fixture.seeds["en"].iter().any(|e| kind(e) == 50101),
            "{org}: health gold is not in the seed"
        );
    }
}

// ── locales ──────────────────────────────────────────────────────────────────

/// An id-like string: hex, uuid, slug, `l_<hash>`, `39100:<pk>:<slug>#<line>`.
fn id_like(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-_:#.@".contains(c))
}

/// Event ids of two seeds: a string that is an event id in `en` may become
/// the matching event's id in the other locale; every other id is shared.
struct EventIds<'a> {
    en: BTreeSet<String>,
    other: BTreeSet<String>,
    locale: &'a str,
}

impl EventIds<'_> {
    fn same_string(&self, path: &str, a: &str, b: &str) {
        if a == b {
            return;
        }
        if self.en.contains(a) && self.other.contains(b) {
            return;
        }
        assert!(
            !(id_like(a) || id_like(b)),
            "{path}: an id differs across locales\n  en: {a}\n  {}: {b}",
            self.locale
        );
    }

    fn same_shape(&self, path: &str, en: &Value, other: &Value) {
        match (en, other) {
            (Value::Object(a), Value::Object(b)) => {
                assert_eq!(
                    a.keys().collect::<Vec<_>>(),
                    b.keys().collect::<Vec<_>>(),
                    "{path}: keys"
                );
                for (k, v) in a {
                    self.same_shape(&format!("{path}.{k}"), v, &b[k]);
                }
            }
            (Value::Array(a), Value::Array(b)) => {
                assert_eq!(a.len(), b.len(), "{path}: length");
                for (i, (x, y)) in a.iter().zip(b).enumerate() {
                    self.same_shape(&format!("{path}[{i}]"), x, y);
                }
            }
            (Value::String(a), Value::String(b)) => self.same_string(path, a, b),
            (a, b) => assert_eq!(a, b, "{path}"),
        }
    }

    fn same_tags(&self, path: &str, a: &Event, b: &Event) {
        let rows = |e: &Event| -> Vec<Vec<String>> {
            e.tags.iter().map(|t| t.as_slice().to_vec()).collect()
        };
        let (a, b) = (rows(a), rows(b));
        assert_eq!(a.len(), b.len(), "{path}: tag count");
        for (i, (x, y)) in a.iter().zip(&b).enumerate() {
            assert_eq!(x.len(), y.len(), "{path}: tag {i} arity");
            for (j, (p, q)) in x.iter().zip(y).enumerate() {
                self.same_string(&format!("{path} tag[{i}][{j}]"), p, q);
            }
        }
    }
}

/// Prototype map § Locales: `pt`/`es` translate `text`, `brief`, `title`,
/// and messages; ids, pubkeys, timestamps, and tags are shared with `en` —
/// event ids and the `receipt`/`e` tags that cite them necessarily follow the
/// translated content.
#[test]
fn locale_seeds_share_ids_pubkeys_timestamps_and_tags_with_en() {
    let mut checked = 0;
    for org in org_names().expect("orgs/") {
        let fixture = load_org(&org).expect(&org);
        let en = &fixture.seeds["en"];
        for (locale, seed) in &fixture.seeds {
            if locale == "en" {
                continue;
            }
            assert_eq!(en.len(), seed.len(), "{org}/{locale}: same event count");
            let ids = EventIds {
                en: en.iter().map(|e| e.id.to_hex()).collect(),
                other: seed.iter().map(|e| e.id.to_hex()).collect(),
                locale,
            };
            let mut translated = 0;
            for (i, (a, b)) in en.iter().zip(seed).enumerate() {
                let at = format!("{org}/{locale}[{i}] kind {}", kind(a));
                assert_eq!(a.kind, b.kind, "{at}");
                assert_eq!(a.pubkey, b.pubkey, "{at}");
                assert_eq!(a.created_at, b.created_at, "{at}");
                ids.same_tags(&at, a, b);
                if a.content != b.content {
                    translated += 1;
                }
                match (
                    serde_json::from_str::<Value>(&a.content),
                    serde_json::from_str::<Value>(&b.content),
                ) {
                    (Ok(x), Ok(y)) => ids.same_shape(&at, &x, &y),
                    (Err(_), Err(_)) => assert!(
                        !(39100..39106).contains(&kind(a)) && !(50000..50200).contains(&kind(a)),
                        "{at}: an org kind carries JSON"
                    ),
                    _ => panic!("{at}: one locale is JSON, the other is not"),
                }
            }
            assert!(
                translated * 2 > en.len(),
                "{org}/{locale}: only {translated} of {} events translated",
                en.len()
            );
            checked += 1;
        }
        for (locale, gold) in &fixture.health {
            assert_eq!(
                gold.len(),
                fixture.health["en"].len(),
                "{org}/{locale}: health gold"
            );
        }
    }
    assert_eq!(checked, 2, "river/pt and energy/es");
}

// ── sequences ────────────────────────────────────────────────────────────────

fn drafts_in(events: &[Event]) -> Vec<(Event, buzz_core::intelligent_org::TicketDraft)> {
    events
        .iter()
        .filter(|e| kind(e) == KIND_IO_DRAFT)
        .filter_map(|e| match roundtrip(e).expect("draft") {
            Decoded::Draft(DraftKind::Ticket, DraftPayload::Ticket(d)) => Some((e.clone(), d)),
            _ => None,
        })
        .collect()
}

/// AI evaluation § Test data: multi-step briefs whose gold order has a
/// `why_gold`; each plan piece is a phrase of the brief; `after` names
/// earlier pieces; exactly one gate; a piece is held iff a predecessor is
/// neither live nor done; `draft_now` is what the agent drafts before the
/// gate; the parent is a held item in the seed.
#[test]
fn sequence_plans_are_ordered_gated_and_grounded_in_the_brief() {
    let names = sequence_names().expect("sequences/");
    assert_eq!(
        names,
        [
            "andalusia",
            "hall-electrics",
            "iberia-pilot",
            "weekday-hall"
        ]
    );
    for name in &names {
        let seq = load_sequence(name).expect(name);
        let c = &seq.case;
        assert_eq!(&c.name, name);
        assert!(!c.why_gold.is_empty(), "{name}: why_gold");
        assert!(c.plan.len() >= 3, "{name}: a multi-step brief");
        let keys: Vec<&str> = c.plan.iter().map(|p| p.key.as_str()).collect();
        let mut seen = BTreeSet::new();
        for (i, p) in c.plan.iter().enumerate() {
            assert_eq!(p.order as usize, i + 1, "{name}: {} order", p.key);
            assert!(
                c.parent
                    .brief
                    .to_lowercase()
                    .contains(&p.piece.to_lowercase()),
                "{name}: piece {:?} is not a phrase of the brief",
                p.piece
            );
            for a in &p.after {
                assert!(
                    seen.contains(a.as_str()),
                    "{name}: {} follows {a}, which is not earlier",
                    p.key
                );
            }
            seen.insert(p.key.as_str());
            if p.covered_by.is_none() {
                assert!(
                    p.title.is_some() && p.requires.is_some(),
                    "{name}: {} needs title and requires",
                    p.key
                );
            }
        }
        assert_eq!(
            c.plan.iter().filter(|p| p.gate).count(),
            1,
            "{name}: one gate"
        );
        assert_eq!(
            c.plan.iter().find(|p| p.gate).map(|p| p.key.as_str()),
            Some(c.gate.as_str())
        );
        assert!(keys.contains(&c.gate.as_str()));
        // Before the gate, the drafts are the pieces that can start now and
        // nothing already covers; a held piece names the predecessor it waits on.
        let before = seq.snapshot("before").expect("before");
        let items = items(&latest_state(&before));
        let parent = &items[&c.parent.id];
        assert_eq!(
            parent.state,
            WorkItemState::Accepted,
            "{name}: parent is held"
        );
        assert_eq!(parent.dri.as_deref(), Some(c.parent.holder.as_str()));
        assert_eq!(parent.brief, c.parent.brief);
        assert_eq!(parent.root, c.parent.root);
        assert_eq!(parent.depth, c.parent.depth);
        let live_or_done = |id: &str| {
            items.get(id).is_some_and(|i| {
                matches!(
                    i.state,
                    WorkItemState::Accepted | WorkItemState::InReview | WorkItemState::Done
                )
            })
        };
        let mut expect_now = Vec::new();
        for p in &c.plan {
            if let Some(cover) = &p.covered_by {
                assert!(
                    live_or_done(cover),
                    "{name}: {} covered by {cover}, which is not live or done",
                    p.key
                );
                assert!(p.held.is_none());
                continue;
            }
            let blocked: Vec<&str> = p
                .after
                .iter()
                .filter(|a| {
                    let pred = c.plan.iter().find(|q| &q.key == *a).expect("plan key");
                    !pred.covered_by.as_deref().is_some_and(live_or_done)
                })
                .map(String::as_str)
                .collect();
            match (&p.held, blocked.first()) {
                (Some(held), Some(first)) => {
                    let pred = c.plan.iter().find(|q| q.key == *first).expect("plan key");
                    assert_eq!(
                        held,
                        &format!("after {}", pred.piece),
                        "{name}: {} held",
                        p.key
                    );
                }
                (None, None) => expect_now.push(p.key.clone()),
                (held, blocked) => {
                    panic!("{name}: {} held={held:?} but blocked by {blocked:?}", p.key)
                }
            }
        }
        assert_eq!(c.draft_now, expect_now, "{name}: draft_now");
        assert!(
            c.draft_now.contains(&c.gate),
            "{name}: the gate is drafted first"
        );
        assert!(
            drafts_in(&seq.deltas["before.json"]).is_empty(),
            "{name}: before.json holds no drafts — they are the move's output"
        );
        for stage in ["before", "gate", "outcome_a", "outcome_b"] {
            assert!(c.snapshots.contains_key(stage), "{name}: stage {stage}");
        }
    }
}

/// The gate stage seeds what the agent's first wave produced: one accepted
/// `50100 t=ticket` per `draft_now` key, needing the parent's holder, gate
/// flagged on the gate alone, the whole plan in `coverage`; each became a
/// `39101` under the parent; the gate holder accepted the gate.
#[test]
fn sequence_gate_stage_is_the_first_wave_accepted() {
    for name in sequence_names().expect("sequences/") {
        let seq = load_sequence(&name).expect(&name);
        let c = &seq.case;
        let g = &c.gate_stage;
        let delta = &seq.deltas["gate.json"];
        let drafts = drafts_in(delta);
        assert_eq!(
            drafts.len(),
            c.draft_now.len(),
            "{name}: one draft per draft_now piece"
        );
        let state = latest_state(&seq.snapshot("gate").expect("gate"));
        let items = items(&state);
        let by_key: HashMap<&str, &fixtures::PlanPiece> =
            c.plan.iter().map(|p| (p.key.as_str(), p)).collect();
        for key in &c.draft_now {
            let piece = by_key[key.as_str()];
            let draft_id = &g.drafts[key];
            let (e, d) = drafts
                .iter()
                .find(|(e, _)| e.id.to_hex() == *draft_id)
                .unwrap_or_else(|| panic!("{name}: draft for {key} in gate.json"));
            assert_eq!(d.parent, c.parent.id, "{name}/{key}: parent");
            assert_eq!(
                tag(e, "n").as_deref(),
                Some(c.parent.holder.as_str()),
                "{name}/{key}: needs the holder"
            );
            assert_eq!(d.covers, piece.piece);
            assert_eq!(d.gate, piece.gate, "{name}/{key}: gate flag");
            assert_eq!(Some(&d.title), piece.title.as_ref());
            assert_eq!(Some(&d.requires), piece.requires.as_ref());
            assert_eq!(
                d.suggested_holder, piece.suggested_holder,
                "{name}/{key}: suggested holder"
            );
            assert_eq!(d.unfilled, piece.unfilled, "{name}/{key}: unfilled");
            assert_eq!(
                d.coverage.len(),
                c.plan.len(),
                "{name}/{key}: coverage is the whole plan"
            );
            // Protocol §4.3: `coverage[].after` names pieces; the plan's `after` names keys.
            for (cov, p) in d.coverage.iter().zip(&c.plan) {
                let after: Vec<&str> = p
                    .after
                    .iter()
                    .map(|k| by_key[k.as_str()].piece.as_str())
                    .collect();
                assert_eq!(
                    (cov.piece.as_str(), cov.order, &cov.held),
                    (p.piece.as_str(), p.order, &p.held),
                    "{name}/{key}: coverage row {}",
                    p.key
                );
                assert_eq!(
                    cov.after, after,
                    "{name}/{key}: coverage row {} after",
                    p.key
                );
                assert_eq!(cov.covered_by, p.covered_by);
            }
            // A first-wave piece follows only pieces already covered by a live
            // or done item — those are its `after`; it never waits on a draft.
            let covered_after: Vec<String> = piece
                .after
                .iter()
                .filter_map(|k| by_key[k.as_str()].covered_by.clone())
                .collect();
            assert_eq!(
                d.after, covered_after,
                "{name}/{key}: after names the live items it follows"
            );
            // Accepted, and the item it became is under the parent.
            let outcome = state
                .get(&(KIND_IO_DRAFT_OUTCOME, draft_id.clone()))
                .unwrap_or_else(|| panic!("{name}/{key}: 39104"));
            let Decoded::DraftOutcome(o) = &outcome.1 else {
                panic!()
            };
            assert_eq!(o.status, DraftOutcomeStatus::Accepted);
            assert_eq!(o.decided_by.as_deref(), Some(c.parent.holder.as_str()));
            let item = &items[&g.items[key]];
            assert_eq!(item.parent.as_deref(), Some(c.parent.id.as_str()));
            assert_eq!(item.draft.as_deref(), Some(draft_id.as_str()));
            assert_eq!(item.title, d.title);
        }
        let gate_item = &items[&g.gate_item];
        assert_eq!(g.gate_item, g.items[&c.gate]);
        assert_eq!(
            gate_item.state,
            WorkItemState::Accepted,
            "{name}: the gate is held"
        );
        assert_eq!(gate_item.dri.as_deref(), Some(g.gate_holder.as_str()));
        let parent = &items[&c.parent.id];
        assert_eq!(
            parent.children.open
                + parent.children.offered
                + parent.children.accepted
                + parent.children.done,
            c.draft_now.len() as u32,
            "{name}: parent children counts"
        );
    }
}

/// Each outcome closes the gate with an `io_done` citing the holder's
/// message, seeds no next-wave drafts (they are the gold the agent must
/// produce), and names a next wave whose `after` points at live items and
/// whose `must_mention` words appear in what the holder said.
#[test]
fn sequence_outcomes_close_the_gate_and_leave_the_next_wave_to_the_agent() {
    for name in sequence_names().expect("sequences/") {
        let seq = load_sequence(&name).expect(&name);
        let c = &seq.case;
        assert_eq!(c.outcomes.keys().collect::<Vec<_>>(), ["a", "b"]);
        let mut next_waves = Vec::new();
        for (label, o) in &c.outcomes {
            let delta = &seq.deltas[&o.file];
            assert_eq!(o.file, format!("outcome-{label}.json"));
            assert!(
                drafts_in(delta).is_empty(),
                "{name}/{label}: the next wave is not seeded"
            );
            assert!(!o.why_gold.is_empty() && !o.said.is_empty());
            let said = delta
                .iter()
                .find(|e| e.id.to_hex() == o.receipt)
                .unwrap_or_else(|| panic!("{name}/{label}: receipt {} in the delta", o.receipt));
            assert_eq!(kind(said), KIND_CHAT);
            assert_eq!(said.content, o.said);
            assert_eq!(
                said.pubkey.to_hex(),
                c.gate_stage.gate_holder,
                "{name}/{label}: the gate holder said it"
            );
            let done = delta
                .iter()
                .find(|e| {
                    kind(e) == KIND_IO_DONE
                        && tag(e, "i").as_deref() == Some(c.gate_stage.gate_item.as_str())
                })
                .unwrap_or_else(|| panic!("{name}/{label}: io_done on the gate"));
            assert!(
                done.tags
                    .iter()
                    .any(|t| t.as_slice().get(1) == Some(&o.receipt)),
                "{name}/{label}: io_done cites the holder's message"
            );
            let stage = format!("outcome_{label}");
            let items = items(&latest_state(&seq.snapshot(&stage).expect("stage")));
            assert_eq!(
                items[&c.gate_stage.gate_item].state,
                WorkItemState::Done,
                "{name}/{label}: gate is done"
            );
            assert!(!o.next_wave.is_empty(), "{name}/{label}: a next wave");
            let plan_keys: BTreeSet<&str> = c.plan.iter().map(|p| p.key.as_str()).collect();
            for piece in &o.next_wave {
                if let Some(replaces) = &piece.replaces {
                    assert!(
                        plan_keys.contains(replaces.as_str()),
                        "{name}/{label}: {} replaces a plan piece",
                        piece.key
                    );
                    assert!(
                        piece.title.is_some(),
                        "{name}/{label}: a re-scoped piece has a title"
                    );
                } else if !plan_keys.contains(piece.key.as_str()) {
                    // A step the brief never named, which the outcome added.
                    assert!(
                        piece.title.is_some() && !piece.requires.is_empty() && piece.gate,
                        "{name}/{label}: {} is neither a plan piece nor a new gate with a title and requires",
                        piece.key
                    );
                    assert!(
                        !o.held.is_empty(),
                        "{name}/{label}: a new gate holds the rest of the plan"
                    );
                }
                for after in &piece.after {
                    let dep = items
                        .get(after)
                        .unwrap_or_else(|| panic!("{name}/{label}: after {after} exists"));
                    assert!(
                        matches!(dep.state, WorkItemState::Accepted | WorkItemState::Done),
                        "{name}/{label}: after names a live or done item"
                    );
                }
                for word in &piece.must_mention {
                    assert!(
                        o.said.to_lowercase().contains(&word.to_lowercase()),
                        "{name}/{label}: {word:?} is in what was said"
                    );
                }
            }
            for key in o.held.iter().chain(&o.not_redrafted) {
                assert!(
                    plan_keys.contains(key.as_str()),
                    "{name}/{label}: {key} is a plan piece"
                );
            }
            for key in &o.not_redrafted {
                assert!(
                    !o.next_wave.iter().any(|p| &p.key == key),
                    "{name}/{label}: {key} is not drafted again"
                );
            }
            assert!(!o.fails.is_empty(), "{name}/{label}: what the judge fails");
            next_waves.push((o.next_wave.clone(), o.held.clone(), o.elsewhere.clone()));
        }
        assert_ne!(
            next_waves[0], next_waves[1],
            "{name}: the outcome changes the plan"
        );
    }
}

// ── who is needed ────────────────────────────────────────────────────────────

/// AI evaluation § Test data: profiles tuned so `requires` is met by one,
/// two, or nobody — checked against the seed's `39105`s and the items each
/// candidate holds, with exactly one of the two at `open_limit`.
#[test]
fn who_is_needed_requirements_are_met_by_one_two_or_nobody() {
    for org in ["river", "energy"] {
        let w = load_who_is_needed(org).expect(org);
        assert_eq!(w.org, org);
        let fixture = load_org(org).expect(org);
        let state = latest_state(&fixture.seeds["en"]);
        let items = items(&state);
        let profiles: Vec<_> = state
            .values()
            .filter_map(|(_, d)| match d {
                Decoded::Profile(p) => Some(p.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            profiles.len(),
            state.keys().filter(|(k, _)| *k == KIND_IO_PROFILE).count()
        );
        let expects: BTreeSet<&str> = w.requirements.iter().map(|r| r.expect.as_str()).collect();
        assert_eq!(
            expects,
            ["nobody", "one", "two"].into_iter().collect(),
            "{org}: one, two, and nobody"
        );
        for r in &w.requirements {
            let mut have: Vec<String> = profiles
                .iter()
                .filter(|p| p.skills.iter().any(|s| s.slug == r.requires))
                .map(|p| p.pubkey.clone())
                .collect();
            have.sort();
            let mut listed: Vec<String> = r.candidates.iter().map(|c| c.pubkey.clone()).collect();
            listed.sort();
            assert_eq!(
                have, listed,
                "{org}/{}: candidates are the profiles with the skill",
                r.requires
            );
            let want = match r.expect.as_str() {
                "one" => 1,
                "two" => 2,
                "nobody" => 0,
                other => panic!("{org}: expect {other}"),
            };
            assert_eq!(r.candidates.len(), want, "{org}/{}", r.requires);
            for c in &r.candidates {
                let profile = profiles
                    .iter()
                    .find(|p| p.pubkey == c.pubkey)
                    .expect("profile");
                assert_eq!(profile.open_limit, c.open_limit);
                assert_eq!(
                    fixture.manifest.people.get(&c.name),
                    Some(&c.pubkey),
                    "{org}: {}",
                    c.name
                );
                let mut open: Vec<String> = items
                    .values()
                    .filter(|i| {
                        i.dri.as_deref() == Some(c.pubkey.as_str())
                            && matches!(i.state, WorkItemState::Accepted | WorkItemState::InReview)
                    })
                    .map(|i| i.id.clone())
                    .collect();
                open.sort();
                let mut listed = c.open_items.clone();
                listed.sort();
                assert_eq!(open, listed, "{org}/{}: {} holds", r.requires, c.name);
                assert_eq!(
                    c.at_limit,
                    c.open_limit.is_some_and(|l| open.len() as u32 >= l),
                    "{org}: {} at limit",
                    c.name
                );
            }
            match want {
                0 => {
                    assert_eq!(r.gold.suggested_holder, None);
                    assert!(
                        r.gold
                            .unfilled
                            .as_deref()
                            .is_some_and(|u| u.contains(&r.requires.replace('-', " "))),
                        "{org}/{}: unfilled names the gap",
                        r.requires
                    );
                }
                1 => {
                    assert!(
                        !r.candidates[0].at_limit,
                        "{org}/{}: the only candidate is free",
                        r.requires
                    );
                    assert_eq!(
                        r.gold.suggested_holder.as_ref(),
                        Some(&r.candidates[0].pubkey)
                    );
                    assert_eq!(
                        r.gold.matched_skills.as_deref(),
                        Some(&[r.requires.clone()][..])
                    );
                }
                _ => {
                    let at_limit: Vec<&str> = r
                        .candidates
                        .iter()
                        .filter(|c| c.at_limit)
                        .map(|c| c.name.as_str())
                        .collect();
                    assert_eq!(
                        at_limit.len(),
                        1,
                        "{org}/{}: exactly one of two at open_limit",
                        r.requires
                    );
                    let free = r.candidates.iter().find(|c| !c.at_limit).expect("free");
                    assert_eq!(
                        r.gold.suggested_holder.as_ref(),
                        Some(&free.pubkey),
                        "{org}/{}: skill over availability picks the free one",
                        r.requires
                    );
                }
            }
        }
    }
    // Energy: Rowan is the one member at open_limit in the seed (Prototype map §3).
    let energy = load_who_is_needed("energy").expect("energy");
    let rowan = energy
        .requirements
        .iter()
        .flat_map(|r| &r.candidates)
        .find(|c| c.name == "Rowan")
        .expect("Rowan is a candidate");
    assert!(rowan.at_limit, "Rowan at open_limit in Energy");
}

/// `sequence.json` pieces that `requires` a skill get the who-is-needed
/// answer for it: the fixture's `suggested_holder` is the gold's.
#[test]
fn sequence_pieces_reuse_the_who_is_needed_gold() {
    let mut checked = 0;
    for name in sequence_names().expect("sequences/") {
        let seq = load_sequence(&name).expect(&name);
        let w = load_who_is_needed(&seq.case.org).expect("who-is-needed");
        for p in &seq.case.plan {
            let Some(requires) = &p.requires else {
                continue;
            };
            for skill in requires {
                if let Some(r) = w.requirements.iter().find(|r| &r.requires == skill) {
                    assert_eq!(
                        p.suggested_holder, r.gold.suggested_holder,
                        "{name}/{}: {skill}",
                        p.key
                    );
                    if r.expect == "nobody" {
                        let gap = skill.replace('-', " ");
                        assert!(
                            p.unfilled.as_deref().is_some_and(|u| u.contains(&gap)),
                            "{name}/{}: unfilled names the gap {gap:?}: {:?}",
                            p.key,
                            p.unfilled
                        );
                    } else {
                        assert_eq!(p.unfilled, None, "{name}/{}: someone fits", p.key);
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(
        checked >= 2,
        "sequences exercise the who-is-needed requirements ({checked})"
    );
}

/// The Rust side of idempotence: `manifest.now` is the fixture's present and
/// every seed event precedes it; sequence deltas start after it.
#[test]
fn seeds_precede_now_and_sequence_deltas_follow_it() {
    for org in org_names().expect("orgs/") {
        let fixture = load_org(&org).expect(&org);
        let now = fixture.manifest.now;
        for e in fixture.seeds.values().flatten() {
            assert!(
                e.created_at.as_secs() <= now,
                "{org}: {} is after now",
                e.id.to_hex()
            );
        }
    }
    for name in sequence_names().expect("sequences/") {
        let seq: SequenceFixture = load_sequence(&name).expect(&name);
        let now = load_org(&seq.case.org).expect("org").manifest.now;
        for (file, delta) in &seq.deltas {
            for e in delta {
                assert!(
                    e.created_at.as_secs() > now,
                    "{name}/{file}: {} is not after now",
                    e.id.to_hex()
                );
            }
        }
    }
}
