use crate::model::Host;

pub(super) fn command_matches_host(host: &Host, query: &str) -> bool {
    command_matches_text(&host.name, query)
        || command_matches_text(&host.address, query)
        || host.tags.iter().any(|tag| command_matches_text(tag, query))
}

pub(super) fn command_matches_text(text: &str, query: &str) -> bool {
    query.is_empty() || text.to_lowercase().contains(query)
}
