//! 主机列表展示模型。

mod filter;
mod status;
mod types;

#[cfg(test)]
mod tests;

use crate::model::AppState;

use super::common::{group_label, tags_label};
use super::labels::auth_label;
use filter::host_matches_query;
use status::host_status_label;

pub(in crate::app) use types::HostViewModel;

pub(super) fn hosts(state: &AppState) -> Vec<HostViewModel> {
    let query = state.ui.workspace.host_search_query.trim().to_lowercase();

    state
        .storage
        .hosts
        .iter()
        .filter(|host| host_matches_query(host, &query))
        .map(|host| HostViewModel {
            id: host.id.0.to_string(),
            name: host.name.clone(),
            endpoint: format!("{}:{}", host.address, host.port),
            auth: auth_label(&host.auth),
            group: group_label(state, host),
            tags: tags_label(host),
            status: host_status_label(state, host.id),
        })
        .collect()
}
