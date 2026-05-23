//! 主机列表搜索过滤。

use crate::model::Host;

pub(super) fn host_matches_query(host: &Host, query: &str) -> bool {
    query.is_empty()
        || host.name.to_lowercase().contains(query)
        || host.address.to_lowercase().contains(query)
        || host
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}
