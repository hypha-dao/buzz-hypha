//! Work prompt for one ticket. The compiler does not call a model.
//!
//! Sections, in order: goal, why, done when, constraints, where the output
//! goes, report back. A `code` ticket adds the repository, the branch, and
//! at most ten paths from a bounded digest. A failed digest publishes nothing.

use std::time::Duration;

/// One file the digest may name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoFile {
    /// Path relative to the repository root.
    pub path: String,
    /// Size in bytes. The digest sums these and stops when the cap is passed.
    pub bytes: u64,
}

/// A listing of one commit. The digest reads this and does not clone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoListing {
    /// Commit the listing was taken at.
    pub commit: String,
    /// Files the walker saw, before selection.
    pub files: Vec<RepoFile>,
    /// How long the walk took.
    pub elapsed: Duration,
}

/// Paths kept for one commit. A later listing of the same commit is not walked again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedDigest {
    /// Commit these paths were selected from.
    pub commit: String,
    /// Selected paths, at most [`DIGEST_MAX_PATHS`].
    pub paths: Vec<String>,
}

/// Why a digest did not produce paths. The cache is left as it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestError {
    /// More files than [`DIGEST_MAX_FILES`].
    TooManyFiles,
    /// More bytes than [`DIGEST_MAX_BYTES`].
    TooManyBytes,
    /// The walk took longer than [`DIGEST_MAX_TIME`].
    TimedOut,
}

/// Caps for one digest. The defaults are the production limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigestLimits {
    /// Files the walker may return.
    pub max_files: usize,
    /// Sum of file sizes.
    pub max_bytes: u64,
    /// Walk time.
    pub max_time: Duration,
    /// Paths named in the prompt.
    pub max_paths: usize,
}

/// How many files a digest will read.
pub const DIGEST_MAX_FILES: usize = 200;
/// How many bytes a digest will count.
pub const DIGEST_MAX_BYTES: u64 = 1_048_576;
/// How long a digest may take.
pub const DIGEST_MAX_TIME: Duration = Duration::from_secs(5);
/// Paths named on a code ticket.
pub const DIGEST_MAX_PATHS: usize = 10;

impl Default for DigestLimits {
    fn default() -> Self {
        Self {
            max_files: DIGEST_MAX_FILES,
            max_bytes: DIGEST_MAX_BYTES,
            max_time: DIGEST_MAX_TIME,
            max_paths: DIGEST_MAX_PATHS,
        }
    }
}

/// Select paths for `listing`. A cache hit for the same commit returns the
/// stored paths and does not look at the listing. A failure leaves `cache` unchanged.
pub fn digest_paths(
    cache: &mut Option<CachedDigest>,
    listing: &RepoListing,
    limits: DigestLimits,
) -> Result<Vec<String>, DigestError> {
    if let Some(cached) = cache.as_ref() {
        if cached.commit == listing.commit {
            return Ok(cached.paths.clone());
        }
    }
    if listing.files.len() > limits.max_files {
        return Err(DigestError::TooManyFiles);
    }
    let bytes = listing
        .files
        .iter()
        .fold(0u64, |sum, file| sum.saturating_add(file.bytes));
    if bytes > limits.max_bytes {
        return Err(DigestError::TooManyBytes);
    }
    if listing.elapsed > limits.max_time {
        return Err(DigestError::TimedOut);
    }
    let paths = select_paths(&listing.files, limits.max_paths);
    *cache = Some(CachedDigest {
        commit: listing.commit.clone(),
        paths: paths.clone(),
    });
    Ok(paths)
}

/// What a code digest returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeDigest {
    /// Repository coordinate or URL.
    pub repo: String,
    /// Commit the paths were taken from.
    pub commit: String,
    /// Paths the digest kept.
    pub paths: Vec<String>,
}

/// The ticket the prompt is about. `kind` is `code` or another step kind.
#[derive(Debug, Clone, Copy)]
pub struct WorkPromptInput<'a> {
    /// Ticket uuid.
    pub ticket_id: &'a str,
    /// Ticket title.
    pub title: &'a str,
    /// This step, in the holder's words.
    pub step: &'a str,
    /// The objective the project serves.
    pub objective: &'a str,
    /// Where the project starts.
    pub change_from: &'a str,
    /// Where the project ends.
    pub change_to: &'a str,
    /// Checks a person could answer yes or no.
    pub done_when: &'a [String],
    /// Refusal and rule lines, already labeled.
    pub constraints: &'a [String],
    /// `code`, `writing`, or another step kind.
    pub kind: &'a str,
}

/// Render the prompt, or `None` when a code digest failed.
///
/// A writing ticket ignores `digest`. A code ticket with `Err` returns
/// `None` and the caller must not publish.
pub fn prompt_text(
    input: &WorkPromptInput<'_>,
    digest: Result<Option<CodeDigest>, DigestError>,
) -> Option<String> {
    let code = if input.kind == "code" {
        match digest {
            Ok(Some(code)) => Some(code),
            _ => return None,
        }
    } else {
        None
    };
    Some(render(input, code.as_ref()))
}

/// Call `publish` only when `text` is `Some`. A failed digest passes `None`
/// and `publish` is not called.
pub fn publish_if_ready(text: Option<String>, publish: &mut dyn FnMut(&str)) -> bool {
    let Some(text) = text else {
        return false;
    };
    publish(&text);
    true
}

/// Branch name `io/<id4>-<slug>` for a ticket.
pub fn work_branch(id: &str, title: &str) -> String {
    let prefix: String = id.chars().take(4).collect();
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in title.chars() {
        let mapped = if c.is_ascii_alphanumeric() {
            Some(c.to_ascii_lowercase())
        } else if c.is_whitespace() || c == '-' || c == '_' {
            Some('-')
        } else {
            None
        };
        match mapped {
            Some('-') if prev_dash || slug.is_empty() => {}
            Some('-') => {
                slug.push('-');
                prev_dash = true;
            }
            Some(ch) => {
                slug.push(ch);
                prev_dash = false;
            }
            None => {}
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        format!("io/{prefix}")
    } else {
        format!("io/{prefix}-{slug}")
    }
}

fn render(input: &WorkPromptInput<'_>, code: Option<&CodeDigest>) -> String {
    let mut out = String::new();
    out.push_str("Goal\n");
    out.push_str(input.title);
    out.push_str("\n\nWhy\n");
    out.push_str(input.objective);
    out.push('\n');
    out.push_str(input.change_from);
    out.push_str(" → ");
    out.push_str(input.change_to);
    out.push('\n');
    out.push_str(input.step);
    out.push_str("\n\nDone when\n");
    for line in input.done_when {
        out.push_str("- ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("\nConstraints\n");
    for line in input.constraints {
        out.push_str("- ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("\nWhere the output goes\n");
    if code.is_some() {
        out.push_str("the branch named below\n");
    } else {
        out.push_str("the ticket thread\n");
    }
    out.push_str("\nReport back\n");
    out.push_str("Post what changed on the ticket, and which done-when lines are met.\n");
    if let Some(code) = code {
        out.push_str("\nRepository\n");
        out.push_str(&code.repo);
        out.push('@');
        out.push_str(&code.commit);
        out.push_str("\n\nBranch\n");
        out.push_str(&work_branch(input.ticket_id, input.title));
        out.push_str("\n\nPaths\n");
        for path in &code.paths {
            out.push_str("- ");
            out.push_str(path);
            out.push('\n');
        }
    }
    out
}

fn select_paths(files: &[RepoFile], max_paths: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut push = |path: &str| {
        if out.len() < max_paths && !out.iter().any(|kept: &String| kept == path) {
            out.push(path.to_owned());
        }
    };
    if let Some(file) = files.iter().find(|file| named(&file.path, "README.md")) {
        push(&file.path);
    }
    if let Some(file) = files.iter().find(|file| named(&file.path, "AGENTS.md")) {
        push(&file.path);
    } else if let Some(file) = files
        .iter()
        .find(|file| named(&file.path, "CONTRIBUTING.md") || named(&file.path, "CONTRIBUTING"))
    {
        push(&file.path);
    }
    for want in [
        "Cargo.toml",
        "package.json",
        "pubspec.yaml",
        "go.mod",
        "pyproject.toml",
    ] {
        for file in files {
            if named(&file.path, want) {
                push(&file.path);
            }
        }
    }
    out
}

fn named(path: &str, want: &str) -> bool {
    let depth = path.matches('/').count();
    if depth > 2 {
        return false;
    }
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(want))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn writing<'a>(done: &'a [String]) -> WorkPromptInput<'a> {
        WorkPromptInput {
            ticket_id: "7f3a0000-0000-4000-8000-000000000001",
            title: "Write the note",
            step: "Write the note",
            objective: "A weekday hall",
            change_from: "no evening",
            change_to: "one paid night",
            done_when: done,
            constraints: &[],
            kind: "writing",
        }
    }

    #[test]
    fn a_writing_ticket_prompt_contains_every_done_when_and_no_path() {
        let done = vec!["note filed".into(), "reader can find it".into()];
        let text = prompt_text(&writing(&done), Ok(None)).expect("writing prompt");
        for line in &done {
            assert!(text.contains(line), "{text}");
        }
        assert!(!text.contains("Paths"), "{text}");
        assert!(!text.contains('/'), "{text}");
        assert!(!text.contains("README"), "{text}");
        assert!(text.find("Goal").unwrap() < text.find("Why").unwrap());
        assert!(text.find("Why").unwrap() < text.find("Done when").unwrap());
        assert!(text.find("Done when").unwrap() < text.find("Constraints").unwrap());
        assert!(text.find("Constraints").unwrap() < text.find("Where the output goes").unwrap());
        assert!(text.find("Where the output goes").unwrap() < text.find("Report back").unwrap());
    }

    #[test]
    fn a_code_ticket_names_only_paths_the_digest_returned() {
        let done = vec!["door opens".into()];
        let input = WorkPromptInput {
            title: "Patch the door",
            step: "Patch the door",
            kind: "code",
            ..writing(&done)
        };
        let files = vec![
            RepoFile {
                path: "README.md".into(),
                bytes: 10,
            },
            RepoFile {
                path: "src/secret.rs".into(),
                bytes: 10,
            },
            RepoFile {
                path: "Cargo.toml".into(),
                bytes: 10,
            },
            RepoFile {
                path: "a/b/c/README.md".into(),
                bytes: 10,
            },
        ];
        let mut cache = None;
        let paths = digest_paths(
            &mut cache,
            &RepoListing {
                commit: "abc1234".into(),
                files,
                elapsed: Duration::ZERO,
            },
            DigestLimits::default(),
        )
        .expect("digest");
        assert_eq!(paths, vec!["README.md".to_owned(), "Cargo.toml".to_owned()]);
        let text = prompt_text(
            &input,
            Ok(Some(CodeDigest {
                repo: "30617:aa:weekday".into(),
                commit: "abc1234".into(),
                paths: paths.clone(),
            })),
        )
        .expect("code prompt");
        for path in &paths {
            assert!(text.contains(path), "{text}");
        }
        assert!(!text.contains("secret.rs"), "{text}");
        assert!(!text.contains("a/b/c/README.md"), "{text}");
        assert!(text.contains("io/7f3a-patch-the-door"), "{text}");
    }

    #[test]
    fn a_failed_digest_does_not_publish_a_prompt() {
        let done = vec!["door opens".into()];
        let input = WorkPromptInput {
            kind: "code",
            ..writing(&done)
        };
        let mut cache = None;
        let err = digest_paths(
            &mut cache,
            &RepoListing {
                commit: "abc1234".into(),
                files: vec![
                    RepoFile {
                        path: "README.md".into(),
                        bytes: 1,
                    };
                    DIGEST_MAX_FILES + 1
                ],
                elapsed: Duration::ZERO,
            },
            DigestLimits::default(),
        )
        .expect_err("too many files");
        assert!(cache.is_none(), "a failure does not fill the cache");
        let text = prompt_text(&input, Err(err));
        assert!(text.is_none());
        let mut calls = 0;
        assert!(!publish_if_ready(text, &mut |_| calls += 1));
        assert_eq!(calls, 0);
    }

    #[test]
    fn a_matching_commit_is_not_walked_again_and_a_failure_keeps_the_cache() {
        let mut cache = None;
        let first = digest_paths(
            &mut cache,
            &RepoListing {
                commit: "abc1234".into(),
                files: vec![RepoFile {
                    path: "README.md".into(),
                    bytes: 4,
                }],
                elapsed: Duration::ZERO,
            },
            DigestLimits::default(),
        )
        .expect("first");
        assert_eq!(first, vec!["README.md".to_owned()]);
        let again = digest_paths(
            &mut cache,
            &RepoListing {
                commit: "abc1234".into(),
                files: vec![RepoFile {
                    path: "secret.rs".into(),
                    bytes: DIGEST_MAX_BYTES + 1,
                }],
                elapsed: DIGEST_MAX_TIME + Duration::from_secs(1),
            },
            DigestLimits::default(),
        )
        .expect("cached");
        assert_eq!(again, vec!["README.md".to_owned()]);
        let failed = digest_paths(
            &mut cache,
            &RepoListing {
                commit: "def5678".into(),
                files: vec![],
                elapsed: DIGEST_MAX_TIME + Duration::from_secs(1),
            },
            DigestLimits::default(),
        );
        assert_eq!(failed, Err(DigestError::TimedOut));
        assert_eq!(cache.expect("kept").commit, "abc1234");
    }
}
