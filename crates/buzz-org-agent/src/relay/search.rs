//! NIP-50 one-shot REQ (Org agent § 4.3). A-1 lands the shape; THINK-R calls it later.

/// Cap on hits per search.
pub const SEARCH_LIMIT: usize = 20;
/// Cap on the query string.
pub const SEARCH_QUERY_MAX: usize = 200;

/// A bounded NIP-50 query. The live path issues a one-shot REQ; the
/// harness answers from [`crate::relay::FakeRelay`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    /// The search string, already truncated.
    pub query: String,
}

impl SearchQuery {
    /// Truncate to [`SEARCH_QUERY_MAX`].
    pub fn new(raw: &str) -> Self {
        let query = if raw.len() > SEARCH_QUERY_MAX {
            raw.chars().take(SEARCH_QUERY_MAX).collect()
        } else {
            raw.to_owned()
        };
        Self { query }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_is_capped() {
        let q = SearchQuery::new(&"x".repeat(500));
        assert_eq!(q.query.len(), SEARCH_QUERY_MAX);
    }
}
