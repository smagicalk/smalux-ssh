//! 主机列表展示模型。

use crate::model::{AppState, HostId, SessionStatus};

use super::common::{group_label, tags_label};
use super::labels::auth_label;

/// 主机列表展示行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct HostViewModel {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub auth: &'static str,
    pub group: String,
    pub tags: String,
    pub status: &'static str,
}

pub(super) fn hosts(state: &AppState) -> Vec<HostViewModel> {
    let query = state.ui.workspace.host_search_query.trim().to_lowercase();

    state
        .storage
        .hosts
        .iter()
        .filter(|host| {
            query.is_empty()
                || host.name.to_lowercase().contains(&query)
                || host.address.to_lowercase().contains(&query)
                || host
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&query))
        })
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

fn host_status_label(state: &AppState, host_id: HostId) -> &'static str {
    let status = state
        .sessions
        .tabs
        .iter()
        .rev()
        .find(|tab| tab.host_id == Some(host_id))
        .map(|tab| &tab.status);

    match status {
        Some(SessionStatus::Connected) | Some(SessionStatus::RunningCommand) => "Connected",
        Some(SessionStatus::Connecting)
        | Some(SessionStatus::Authenticating)
        | Some(SessionStatus::Reconnecting) => "Connecting",
        Some(SessionStatus::Failed { .. }) => "Failed",
        Some(SessionStatus::Disconnected) => "Disconnected",
        Some(SessionStatus::Created) => "Created",
        None => "Saved",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthProfile, Host, HostId};
    use uuid::Uuid;

    fn agent_host(name: &str, address: &str, tags: &[&str]) -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: name.to_owned(),
            group_id: None,
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            address: address.to_owned(),
            port: 22,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                key_hint: None,
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    #[test]
    fn host_rows_do_not_expose_auth_secrets() {
        let mut state = AppState::default();
        state.storage.upsert_host(Host {
            id: HostId(Uuid::new_v4()),
            name: "root".to_owned(),
            group_id: None,
            tags: Vec::new(),
            address: "example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Password {
                username: "root".to_owned(),
                secret: crate::model::SecretRef("password:root".to_owned()),
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        });

        let rows = hosts(&state);

        assert_eq!(rows[0].auth, "Password");
        assert!(!rows[0].endpoint.contains("password"));
    }

    #[test]
    fn host_rows_follow_search_query() {
        let mut state = AppState::default();
        state
            .storage
            .upsert_host(agent_host("Production", "prod.example.com", &["prod"]));
        state
            .storage
            .upsert_host(agent_host("Staging", "staging.example.com", &["stage"]));
        state.ui.workspace.set_host_search_query("prod");

        let rows = hosts(&state);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Production");
    }
}
