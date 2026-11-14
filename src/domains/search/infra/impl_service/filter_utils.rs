//! Escaping for the optional Meilisearch filter language.

/// Escape filter literals before constructing Meilisearch expressions.
pub(crate) fn escape_filter_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
