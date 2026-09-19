//! E-2 gold cases load, gold parses, negatives are ≥ half of each suite,
//! and the README job table matches Org agent § 1.
//!
//! Development plan E-2 "Proves": *each case loads and its gold parses;
//! negatives are ≥ half of each suite; every job in Org agent § 1 with
//! model output has ≥ 1 case set.*

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use crate::fixtures::cases::{
    cases_dir, load_all_suites, load_vacuous_titles, parse_case_gold, MODEL_OUTPUT_JOBS,
    ORG_AGENT_JOBS, SUITE_FILES,
};
use crate::fixtures::{load_org, load_sequence, org_names};

#[test]
fn every_case_loads_and_its_gold_parses() {
    let suites = load_all_suites().expect("cases/");
    let names: BTreeSet<String> = suites.keys().cloned().collect();
    let expected: BTreeSet<String> = SUITE_FILES.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(names, expected);
    let orgs: BTreeSet<String> = org_names().expect("orgs/").into_iter().collect();
    let mut ids = BTreeSet::new();
    let mut total = 0usize;
    for (name, suite) in &suites {
        assert_eq!(&suite.suite, name);
        assert!(!suite.cases.is_empty(), "{name}: no cases");
        for case in &suite.cases {
            assert!(ids.insert(case.id.clone()), "duplicate case id {}", case.id);
            assert!(
                orgs.contains(&case.org),
                "{}: unknown org {}",
                case.id,
                case.org
            );
            assert_eq!(case.seed.org, case.org, "{}: seed.org", case.id);
            parse_case_gold(case).unwrap_or_else(|e| panic!("{}: {e}", case.id));
            if let Some(seq) = &case.seed.sequence {
                let loaded = load_sequence(seq).unwrap_or_else(|e| panic!("{}: {e}", case.id));
                assert_eq!(loaded.case.org, case.org, "{}: sequence org", case.id);
                if let Some(snap) = &case.seed.snapshot {
                    loaded
                        .snapshot(snap)
                        .unwrap_or_else(|e| panic!("{}: snapshot {snap}: {e}", case.id));
                }
            } else {
                load_org(&case.org).unwrap_or_else(|e| panic!("{}: {e}", case.id));
            }
            total += 1;
        }
    }
    assert!(
        total >= 100,
        "E-2 first cut is four suites plus § 5, got {total}"
    );
}

#[test]
fn negatives_are_at_least_half_of_each_suite() {
    let suites = load_all_suites().expect("cases/");
    for (name, suite) in &suites {
        let total = suite.cases.len();
        let silence = suite.cases.iter().filter(|c| c.gold.silence).count();
        assert!(
            silence * 2 >= total,
            "{name}: silence {silence} / {total} is under half (AI evaluation § Negatives matter more than positives)"
        );
    }
}

#[test]
fn first_cut_includes_sequence_and_who_is_needed_cases() {
    let suites = load_all_suites().expect("cases/");
    let ids: BTreeSet<&str> = suites
        .values()
        .flat_map(|s| s.cases.iter().map(|c| c.id.as_str()))
        .collect();
    for required in [
        "j2-weekday-hall-gate-first",
        "j2-weekday-hall-next-wave-granted",
        "j2-weekday-hall-outcome-changes-plan",
        "j2-no-invented-order",
        "j2-iberia-site-gate-only",
        "j2-hall-electrics-unfilled",
        "j2-requires-over-availability",
        "j2-dutch-law-unfilled",
        "j1-river-electrics-unfilled",
        "j1b-energy-unfilled-dutch-law",
    ] {
        assert!(ids.contains(required), "missing first-cut case {required}");
    }
}

#[test]
fn every_model_output_job_has_a_case_set() {
    let suites = load_all_suites().expect("cases/");
    let mut by_job: BTreeMap<&str, usize> = BTreeMap::new();
    for suite in suites.values() {
        for case in &suite.cases {
            *by_job.entry(case.job.as_str()).or_insert(0) += 1;
        }
    }
    for job in MODEL_OUTPUT_JOBS {
        let n = by_job.get(job).copied().unwrap_or(0);
        assert!(n >= 1, "{job} has model output and no case (Org agent § 1)");
    }
    assert!(
        by_job.get("J8").copied().unwrap_or(0) >= 1,
        "J8/J8b share done-from-talk; J8 itself needs §5.5 cases"
    );
}

#[test]
fn readme_job_table_matches_org_agent_section_1() {
    let readme = fs::read_to_string(cases_dir().parent().expect("eval/").join("README.md"))
        .expect("tests/eval/README.md");
    let suites = load_all_suites().expect("cases/");
    let mut by_job: BTreeMap<String, usize> = BTreeMap::new();
    for suite in suites.values() {
        for case in &suite.cases {
            *by_job.entry(case.job.clone()).or_insert(0) += 1;
        }
    }

    let mut seen = BTreeSet::new();
    for line in readme.lines() {
        let cols: Vec<&str> = line
            .split('|')
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .collect();
        if cols.len() < 4
            || !cols[0].starts_with('J')
            || !cols[0]
                .as_bytes()
                .get(1)
                .is_some_and(|b| b.is_ascii_digit())
        {
            continue;
        }
        let job = cols[0];
        assert!(
            ORG_AGENT_JOBS.contains(&job),
            "README job {job} is not an Org agent § 1 row"
        );
        let model = cols[1];
        if MODEL_OUTPUT_JOBS.contains(&job) {
            assert_eq!(model, "yes", "{job} has model output");
            let listed: usize = cols[3]
                .parse()
                .unwrap_or_else(|_| panic!("{job}: cases column {}", cols[3]));
            let actual = by_job.get(job).copied().unwrap_or(0);
            assert_eq!(
                listed, actual,
                "{job}: README says {listed}, files have {actual}"
            );
            assert!(listed >= 1, "{job}: README case count");
        }
        seen.insert(job.to_string());
    }
    for job in ORG_AGENT_JOBS {
        assert!(seen.contains(*job), "README table missing {job}");
    }
}

#[test]
fn vacuous_titles_and_judge_prompt_v1_are_present() {
    let titles = load_vacuous_titles().expect("vacuous-titles.json");
    assert!(
        titles.len() >= 10,
        "vacuous-title list is short: {}",
        titles.len()
    );
    for must in ["Make a plan", "Do research", "Kick-off"] {
        assert!(
            titles.iter().any(|t| t.eq_ignore_ascii_case(must)),
            "vacuous-titles.json must include {must}"
        );
    }
    let prompt =
        fs::read_to_string(cases_dir().join("judge-prompt-v1.md")).expect("judge-prompt-v1.md");
    assert!(prompt.contains("≥ 11 of 14") || prompt.contains(">= 11 of 14"));
    for (i, q) in [
        "Serves the cited line",
        "Not already covered",
        "The named person would recognise it",
        "Size of one holder",
        "Specific to this org",
        "Right next thing",
        "Who this really needs",
    ]
    .iter()
    .enumerate()
    {
        assert!(prompt.contains(q), "judge prompt missing Q{}: {q}", i + 1);
    }
    let readme =
        fs::read_to_string(cases_dir().parent().expect("eval/").join("README.md")).expect("README");
    assert!(readme.contains("Cohen's κ") || readme.contains("Cohen's k"));
    assert!(readme.contains("0.70"));
}
