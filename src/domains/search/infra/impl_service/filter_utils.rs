//! `MeiliSearch` filter string parsing and escaping utilities.

/// Split a `MeiliSearch` filter string on ` AND ` while respecting quoted strings.
/// A naive `split(" AND ")` would break on values like `"Rock AND Roll"`.
pub(super) fn split_filter_clauses(filter_str: &str) -> Vec<&str> {
    let bytes = filter_str.as_bytes();
    let len = bytes.len();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;

    while i < len {
        if bytes[i] == b'"' {
            // Skip to closing quote, handling escapes
            i += 1;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2; // skip escaped char
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else if i + 5 <= len && &filter_str[i..i + 5] == " AND " {
            parts.push(filter_str[start..i].trim());
            i += 5;
            start = i;
        } else {
            i += 1;
        }
    }
    if start < len {
        parts.push(filter_str[start..].trim());
    }
    parts
}

/// Parsed filter values extracted from the `MeiliSearch` filter string.
pub(super) struct ParsedFilters {
    /// Director name filter.
    pub director: Option<String>,
    /// Studio name filter.
    pub studio: Option<String>,
    /// Label name filter.
    pub label: Option<String>,
    /// Genre name filter.
    pub genre: Option<String>,
    /// Date range start (inclusive).
    pub date_from: Option<String>,
    /// Date range end (inclusive).
    pub date_to: Option<String>,
}

/// Parse the MeiliSearch-style filter string into individual filter values.
pub(super) fn parse_filters(filter_str: &str) -> ParsedFilters {
    let mut filters = ParsedFilters {
        director: None,
        studio: None,
        label: None,
        genre: None,
        date_from: None,
        date_to: None,
    };

    for part in split_filter_clauses(filter_str) {
        let part = part.trim();
        if let Some(val) = extract_filter_value(part, "director_name = ") {
            filters.director = Some(val);
        } else if let Some(val) = extract_filter_value(part, "studio_name = ") {
            filters.studio = Some(val);
        } else if let Some(val) = extract_filter_value(part, "label_name = ") {
            filters.label = Some(val);
        } else if let Some(val) = extract_filter_value(part, "genre_names = ") {
            filters.genre = Some(val);
        } else if let Some(val) = extract_filter_value(part, "date >= \"") {
            filters.date_from = Some(val);
        } else if let Some(val) = extract_filter_value(part, "date <= \"") {
            filters.date_to = Some(val);
        }
    }

    filters
}

/// Extract a quoted value after the given prefix from a filter clause.
pub(super) fn extract_filter_value(part: &str, prefix: &str) -> Option<String> {
    part.strip_prefix(prefix).map(|rest| {
        // The prefix may or may not include the opening quote.
        let val = rest.strip_prefix('"').unwrap_or(rest);

        // Scan forward to find the closing unescaped quote.
        let end = find_closing_quote(val);
        let val = &val[..end];

        // Unescape MeiliSearch filter escaping: \\ → \, \" → "
        val.replace("\\\"", "\"").replace("\\\\", "\\")
    })
}

/// Find the position of the closing unescaped double-quote in a string.
/// Returns the length of the string if no unescaped quote is found.
pub(super) fn find_closing_quote(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            // Skip escaped character
            i += 2;
        } else if bytes[i] == b'"' {
            return i;
        } else {
            i += 1;
        }
    }
    s.len()
}

/// Escape double quotes in filter values to prevent filter injection.
///
/// Shared by both search service and `MeiliSearch` repo to avoid duplication.
pub(crate) fn escape_filter_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_filters_empty() {
        let f = parse_filters("");
        assert!(f.director.is_none());
        assert!(f.studio.is_none());
        assert!(f.label.is_none());
        assert!(f.genre.is_none());
        assert!(f.date_from.is_none());
        assert!(f.date_to.is_none());
    }

    #[test]
    fn test_parse_filters_single_field() {
        let f = parse_filters("director_name = \"John\"");
        assert_eq!(f.director.as_deref(), Some("John"));
        assert!(f.studio.is_none());
    }

    #[test]
    fn test_parse_filters_all_fields() {
        let f = parse_filters(
            "permission <= 10 AND director_name = \"Alice\" AND studio_name = \"Studio X\" \
             AND label_name = \"Label Y\" AND genre_names = \"Drama\" \
             AND date >= \"2024-01-01\" AND date <= \"2024-12-31\"",
        );
        assert_eq!(f.director.as_deref(), Some("Alice"));
        assert_eq!(f.studio.as_deref(), Some("Studio X"));
        assert_eq!(f.label.as_deref(), Some("Label Y"));
        assert_eq!(f.genre.as_deref(), Some("Drama"));
        assert_eq!(f.date_from.as_deref(), Some("2024-01-01"));
        assert_eq!(f.date_to.as_deref(), Some("2024-12-31"));
    }

    #[test]
    fn test_parse_filters_ignores_unknown() {
        let f = parse_filters("permission <= 5 AND unknown_field = \"val\"");
        assert!(f.director.is_none());
    }

    #[test]
    fn test_split_filter_clauses_basic() {
        let parts = split_filter_clauses("a = 1 AND b = 2");
        assert_eq!(parts, vec!["a = 1", "b = 2"]);
    }

    #[test]
    fn test_split_filter_clauses_with_quotes() {
        let parts =
            split_filter_clauses("studio_name = \"Rock AND Roll\" AND director_name = \"John\"");
        assert_eq!(
            parts,
            vec![
                "studio_name = \"Rock AND Roll\"",
                "director_name = \"John\""
            ]
        );
    }

    #[test]
    fn test_split_filter_clauses_single() {
        let parts = split_filter_clauses("permission <= 5");
        assert_eq!(parts, vec!["permission <= 5"]);
    }

    #[test]
    fn test_split_filter_clauses_empty() {
        let parts: Vec<&str> = split_filter_clauses("");
        assert!(parts.is_empty());
    }

    #[test]
    fn test_extract_filter_value_basic() {
        assert_eq!(
            extract_filter_value("director_name = \"John\"", "director_name = "),
            Some("John".to_owned())
        );
        assert_eq!(
            extract_filter_value("date >= \"2024-01-01\"", "date >= \""),
            Some("2024-01-01".to_owned())
        );
    }

    #[test]
    fn test_extract_filter_value_unescapes() {
        // MeiliSearch-escaped quotes and backslashes should be unescaped
        assert_eq!(
            extract_filter_value(
                "director_name = \"John \\\"The Rock\\\"\"",
                "director_name = "
            ),
            Some("John \"The Rock\"".to_owned())
        );
        assert_eq!(
            extract_filter_value("studio_name = \"C:\\\\Path\"", "studio_name = "),
            Some("C:\\Path".to_owned())
        );
    }

    #[test]
    fn test_extract_filter_value_no_match() {
        assert_eq!(extract_filter_value("other = 5", "director_name = "), None);
    }

    #[test]
    fn test_find_closing_quote_basic() {
        assert_eq!(find_closing_quote("John\""), 4);
        assert_eq!(find_closing_quote("John"), 4); // no quote
        assert_eq!(find_closing_quote(r#"John \"Rock\""#), 13); // all escaped, no closing quote → full length
    }
}
