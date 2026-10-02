//! Which project a ticket belongs under.
//!
//! A description matches the one project whose title or brief uses those
//! words. When the member names no project, the ticket's own words pick
//! the best fit among the projects they hold.

use std::collections::HashSet;

/// A project the ticket can sit under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FitProject {
    /// Work item id.
    pub id: String,
    /// Project title.
    pub title: String,
    /// Project brief. Empty when the live item has none.
    pub brief: String,
}

/// The project a description points at.
///
/// `None` when no project fits, or when more than one fits equally.
pub fn match_described_project(projects: &[FitProject], description: &str) -> Option<String> {
    if parent_unspecified(description) {
        return None;
    }
    best_unique(projects, description)
}

/// The project the ticket fits.
///
/// The only project they hold is that project. Several projects need one
/// clear overlap with the ticket's title and brief.
pub fn suggest_project(projects: &[FitProject], title: &str, brief: &str) -> Option<String> {
    if projects.len() == 1 {
        return Some(projects[0].id.clone());
    }
    let work = format!("{title} {brief}");
    if parent_unspecified(&work) {
        return None;
    }
    best_unique(projects, &work)
}

/// True when `query` never names or describes a project.
pub fn parent_unspecified(query: &str) -> bool {
    significant_words(query).is_empty()
}

fn best_unique(projects: &[FitProject], query: &str) -> Option<String> {
    let words = significant_words(query);
    let mut ranked: Vec<(i32, &str)> = projects
        .iter()
        .filter_map(|project| {
            let score = score(project, query, &words);
            (score >= 4).then_some((score, project.id.as_str()))
        })
        .collect();
    if ranked.is_empty() {
        return None;
    }
    ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(right.1)));
    if ranked.len() > 1 && ranked[1].0 == ranked[0].0 {
        return None;
    }
    Some(ranked[0].1.to_string())
}

fn score(project: &FitProject, query: &str, words: &HashSet<String>) -> i32 {
    let title_words = content_words(&project.title);
    let brief_words = content_words(&project.brief);
    let mut score = 0;
    for word in words {
        if title_words.contains(word) {
            score += 4;
        } else if brief_words.contains(word) {
            score += 2;
        }
    }
    let query_phrase = phrase(query);
    let title_phrase = phrase(&project.title);
    if title_phrase.len() >= 4 && query_phrase.contains(&title_phrase) {
        score += 12;
    }
    let brief_phrase = phrase(&project.brief);
    if brief_phrase.len() >= 8 && query_phrase.contains(&brief_phrase) {
        score += 8;
    }
    if !title_words.is_empty() && title_words.iter().all(|word| words.contains(word)) {
        score += 6;
    }
    score
}

fn significant_words(text: &str) -> HashSet<String> {
    content_words(text)
        .into_iter()
        .filter(|word| !STOPWORDS.contains(&word.as_str()))
        .collect()
}

fn content_words(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| word.len() >= 3)
        .map(|word| word.to_ascii_lowercase())
        .collect()
}

fn phrase(text: &str) -> String {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

const STOPWORDS: &[&str] = &[
    "the",
    "and",
    "for",
    "with",
    "that",
    "this",
    "has",
    "have",
    "from",
    "into",
    "under",
    "about",
    "project",
    "projects",
    "ticket",
    "tickets",
    "related",
    "regarding",
    "something",
    "please",
    "just",
    "like",
    "work",
    "one",
    "thing",
    "stuff",
    "does",
    "doing",
    "want",
    "need",
    "make",
    "new",
    "our",
    "their",
    "its",
    "not",
    "but",
    "you",
    "your",
    "they",
    "them",
    "are",
    "was",
    "were",
    "been",
    "will",
    "can",
    "could",
    "should",
    "would",
    "what",
    "when",
    "where",
    "which",
    "who",
    "how",
    "all",
    "any",
    "some",
    "more",
    "than",
    "then",
    "also",
    "only",
    "very",
    "really",
    "kind",
    "sort",
    "type",
    "called",
    "named",
    "titled",
    "hold",
    "holds",
    "holding",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn project(id: &str, title: &str, brief: &str) -> FitProject {
        FitProject {
            id: id.into(),
            title: title.into(),
            brief: brief.into(),
        }
    }

    #[test]
    fn a_description_matches_the_project_it_talks_about() {
        let projects = vec![
            project("hall", "Hall roof", "Replace the leaking tiles"),
            project("land", "Public site", "Rebuild the marketing landing page"),
        ];
        assert_eq!(
            match_described_project(
                &projects,
                "it is the project that has to do with landing page"
            ),
            Some("land".into())
        );
    }

    #[test]
    fn two_projects_described_the_same_way_are_not_a_guess() {
        let projects = vec![
            project("desk", "Finish Phase 0 for Hypha Desktop App", ""),
            project("mobile", "Finish Phase 0 for Mobile", ""),
        ];
        assert_eq!(match_described_project(&projects, "Finish Phase 0"), None);
    }

    #[test]
    fn a_name_that_fits_nothing_does_not_pick_another_project() {
        let projects = vec![project("land", "Public site", "The landing page")];
        assert_eq!(match_described_project(&projects, "Aquarium lights"), None);
    }

    #[test]
    fn no_description_suggests_the_project_the_ticket_fits() {
        let projects = vec![
            project("hall", "Hall roof", "Replace the leaking tiles"),
            project("land", "Public site", "Rebuild the marketing landing page"),
        ];
        assert_eq!(
            suggest_project(
                &projects,
                "Hero on the landing page",
                "Update the homepage hero"
            ),
            Some("land".into())
        );
    }

    #[test]
    fn the_only_project_they_hold_is_the_suggestion() {
        let projects = vec![project("hall", "Hall roof", "")];
        assert_eq!(
            suggest_project(&projects, "Order tiles", "Buy the replacement tiles"),
            Some("hall".into())
        );
    }

    #[test]
    fn several_projects_with_no_overlap_are_not_a_guess() {
        let projects = vec![
            project("hall", "Hall roof", "Replace the leaking tiles"),
            project("grant", "Community grant", "Write the funding application"),
        ];
        assert_eq!(
            suggest_project(&projects, "Order tiles", "Buy replacements"),
            None
        );
    }

    #[test]
    fn naming_nothing_is_unspecified() {
        assert!(parent_unspecified(""));
        assert!(parent_unspecified("the project"));
        assert!(!parent_unspecified("landing page"));
    }
}
