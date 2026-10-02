//! Who fits a new piece of work, and how a chat update merges a profile.
//!
//! Fit is the overlap between the work's words and a member's skills and
//! about, then how many pieces they already hold. A member at their
//! `open_limit` is not suggested. A profile chat keeps every field the
//! member did not mention.

use std::collections::HashSet;

use buzz_core::intelligent_org::{
    normalize_socials, skill_slug, ProfileSocial, PROFILE_ABOUT_MAX_CHARS, PROFILE_MAX_SKILLS,
    PROFILE_SKILL_LABEL_MAX_CHARS,
};

/// A member the holder picker can see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FitPerson {
    /// Member pubkey.
    pub pubkey: String,
    /// Their about.
    pub about: String,
    /// Skill labels, in their words.
    pub skills: Vec<String>,
    /// Self-set cap, when they set one.
    pub open_limit: Option<u32>,
    /// Pieces they already hold (`accepted` or `in_review`).
    pub held: u32,
}

/// The member whose skills and about match the work, and who still has room.
///
/// Higher overlap wins. A lighter workload breaks a tie. Nobody with a zero
/// score is named — open, with no suggestion, is the answer when nobody fits.
pub fn suggest_holder(people: &[FitPerson], title: &str, brief: &str) -> Option<String> {
    let work = words(&format!("{title} {brief}"));
    if work.is_empty() {
        return None;
    }
    let mut best: Option<(i32, u32, &str)> = None;
    for person in people {
        if person.open_limit.is_some_and(|limit| person.held >= limit) {
            continue;
        }
        let score = fit_score(&person.skills, &person.about, &work);
        if score <= 0 {
            continue;
        }
        let replace = match best {
            None => true,
            Some((best_score, best_held, best_pk)) => {
                score > best_score
                    || (score == best_score && person.held < best_held)
                    || (score == best_score
                        && person.held == best_held
                        && person.pubkey.as_str() < best_pk)
            }
        };
        if replace {
            best = Some((score, person.held, person.pubkey.as_str()));
        }
    }
    best.map(|(_, _, pubkey)| pubkey.to_string())
}

fn fit_score(skills: &[String], about: &str, work: &HashSet<String>) -> i32 {
    let mut score = 0;
    for skill in skills {
        let parts = words(skill);
        if !parts.is_empty() && parts.iter().all(|part| work.contains(part)) {
            score += 3;
        }
    }
    let about_hits = words(about)
        .into_iter()
        .filter(|word| word.len() >= 4 && work.contains(word))
        .count();
    score + i32::try_from(about_hits.min(4)).unwrap_or(4)
}

fn words(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| word.len() >= 3)
        .map(|word| word.to_ascii_lowercase())
        .collect()
}

/// The profile already stored for the speaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredProfile {
    /// About text.
    pub about: String,
    /// Skill labels.
    pub skills: Vec<String>,
    /// Social links.
    pub socials: Vec<ProfileSocial>,
    /// Open-piece cap.
    pub open_limit: Option<u32>,
}

/// Fields the member just stated. `None` keeps the stored value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilePatch {
    /// Replacement about.
    pub about: Option<String>,
    /// Replacement skill list.
    pub skills: Option<Vec<String>>,
    /// Replacement social links.
    pub socials: Option<Vec<ProfileSocial>>,
}

/// The profile to save. `None` when the patch is empty or changes nothing.
pub fn merge_profile(current: &StoredProfile, patch: &ProfilePatch) -> Option<StoredProfile> {
    if patch.about.is_none() && patch.skills.is_none() && patch.socials.is_none() {
        return None;
    }
    let about = patch
        .about
        .as_deref()
        .map(clean_about)
        .unwrap_or_else(|| current.about.clone());
    let skills = patch
        .skills
        .as_deref()
        .map(clean_skills)
        .unwrap_or_else(|| current.skills.clone());
    let socials = match &patch.socials {
        Some(raw) => normalize_socials(raw).unwrap_or_else(|_| current.socials.clone()),
        None => current.socials.clone(),
    };
    let next = StoredProfile {
        about,
        skills,
        socials,
        open_limit: current.open_limit,
    };
    if next == *current {
        None
    } else {
        Some(next)
    }
}

fn clean_about(about: &str) -> String {
    let trimmed = about.trim();
    if trimmed.chars().count() <= PROFILE_ABOUT_MAX_CHARS {
        return trimmed.to_string();
    }
    trimmed.chars().take(PROFILE_ABOUT_MAX_CHARS).collect()
}

fn clean_skills(labels: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for label in labels {
        let label = label.trim();
        if label.is_empty() || label.chars().count() > PROFILE_SKILL_LABEL_MAX_CHARS {
            continue;
        }
        let slug = skill_slug(label);
        if slug.is_empty() || !seen.insert(slug) {
            continue;
        }
        out.push(label.to_string());
        if out.len() == PROFILE_MAX_SKILLS {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(
        pubkey: &str,
        skills: &[&str],
        about: &str,
        held: u32,
        limit: Option<u32>,
    ) -> FitPerson {
        FitPerson {
            pubkey: pubkey.into(),
            about: about.into(),
            skills: skills.iter().map(|skill| (*skill).to_string()).collect(),
            open_limit: limit,
            held,
        }
    }

    #[test]
    fn the_lighter_matching_member_is_suggested() {
        let ada = "aa".repeat(32);
        let bea = "bb".repeat(32);
        let people = vec![
            person(&ada, &["rust"], "", 2, None),
            person(
                &bea,
                &["rust", "desktop"],
                "Ships the desktop app",
                0,
                Some(3),
            ),
        ];
        assert_eq!(
            suggest_holder(&people, "Desktop relay", "Rust client work"),
            Some(bea)
        );
    }

    #[test]
    fn a_member_at_their_limit_is_not_suggested() {
        let ada = "aa".repeat(32);
        let people = vec![person(&ada, &["grants"], "grant writing", 1, Some(1))];
        assert_eq!(suggest_holder(&people, "Write the grant", ""), None);
    }

    #[test]
    fn no_overlap_leaves_the_holder_open() {
        let ada = "aa".repeat(32);
        let people = vec![person(&ada, &["cooking"], "Tuesday kitchen", 0, None)];
        assert_eq!(suggest_holder(&people, "Ship the relay", "postgres"), None);
    }

    #[test]
    fn a_social_update_keeps_the_about_they_did_not_mention() {
        let current = StoredProfile {
            about: "I wire halls.".into(),
            skills: vec!["electrics".into()],
            socials: vec![],
            open_limit: Some(2),
        };
        let next = merge_profile(
            &current,
            &ProfilePatch {
                about: None,
                skills: None,
                socials: Some(vec![ProfileSocial {
                    network: "github".into(),
                    url: "https://github.com/travolta".into(),
                }]),
            },
        )
        .expect("changed");
        assert_eq!(next.about, "I wire halls.");
        assert_eq!(next.skills, vec!["electrics".to_string()]);
        assert_eq!(next.open_limit, Some(2));
        assert_eq!(next.socials[0].network, "github");
    }
}
