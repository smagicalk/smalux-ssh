//! 命令面板展示模型。

use crate::model::{AppState, Host, HostId};

use super::common::tags_label;

/// 命令面板结果行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct CommandPaletteItemViewModel {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub kind: &'static str,
}

pub(super) fn command_palette_results(state: &AppState) -> Vec<CommandPaletteItemViewModel> {
    let query = state
        .ui
        .workspace
        .command_palette
        .query
        .trim()
        .to_lowercase();
    let mut rows = Vec::new();

    rows.extend(
        state
            .storage
            .hosts
            .iter()
            .filter(|host| command_matches_host(host, &query))
            .take(8)
            .map(|host| CommandPaletteItemViewModel {
                id: host.id.0.to_string(),
                title: host.name.clone(),
                subtitle: format!("{}:{} · {}", host.address, host.port, tags_label(host)),
                kind: "Host",
            }),
    );

    rows.extend(
        state
            .storage
            .recent_connections
            .iter()
            .filter(|recent| command_matches_text(&recent.label, &query))
            .take(5)
            .map(|recent| CommandPaletteItemViewModel {
                id: recent.host_id.0.to_string(),
                title: recent.label.clone(),
                subtitle: "recent connection".to_owned(),
                kind: "Recent",
            }),
    );

    rows.extend(
        state
            .storage
            .command_history
            .iter()
            .rev()
            .filter(|item| command_matches_text(&item.command, &query))
            .take(8)
            .map(|item| CommandPaletteItemViewModel {
                id: item.id.0.to_string(),
                title: item.command.clone(),
                subtitle: command_history_subtitle(state, item.host_id),
                kind: "History",
            }),
    );

    rows
}

fn command_matches_host(host: &Host, query: &str) -> bool {
    command_matches_text(&host.name, query)
        || command_matches_text(&host.address, query)
        || host.tags.iter().any(|tag| command_matches_text(tag, query))
}

fn command_matches_text(text: &str, query: &str) -> bool {
    query.is_empty() || text.to_lowercase().contains(query)
}

fn command_history_subtitle(state: &AppState, host_id: Option<HostId>) -> String {
    let Some(host_id) = host_id else {
        return "global command".to_owned();
    };

    state
        .storage
        .hosts
        .iter()
        .find(|host| host.id == host_id)
        .map(|host| format!("history · {}", host.name))
        .unwrap_or_else(|| "history · deleted host".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AuthProfile, CommandHistoryId, CommandHistoryItem, HostId, RecentConnection,
    };
    use uuid::Uuid;

    fn host(name: &str, address: &str, tags: &[&str]) -> Host {
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
    fn command_palette_includes_matching_hosts_recent_and_history() {
        let mut state = AppState::default();
        let prod = host("Production API", "api.example.com", &["prod", "linux"]);
        let prod_id = prod.id;
        state.storage.upsert_host(prod);
        state
            .storage
            .upsert_host(host("Jump Box", "jump.internal", &["bastion"]));
        state.storage.record_recent_connection(RecentConnection {
            host_id: prod_id,
            label: "Production API".to_owned(),
            connected_at_unix_secs: 1,
        });
        state.storage.add_command_history(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(prod_id),
            command: "systemctl status api".to_owned(),
            working_directory: None,
            exit_code: Some(0),
            started_at_unix_secs: 2,
            duration_ms: Some(30),
        });
        state.ui.workspace.open_command_palette("production");

        let rows = command_palette_results(&state);

        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.kind == "Host"));
        assert!(rows.iter().any(|row| row.kind == "Recent"));
    }

    #[test]
    fn command_palette_searches_history_commands() {
        let mut state = AppState::default();
        let prod = host("Production API", "api.example.com", &["prod"]);
        let prod_id = prod.id;
        state.storage.upsert_host(prod);
        state.storage.add_command_history(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(prod_id),
            command: "journalctl -u api".to_owned(),
            working_directory: None,
            exit_code: Some(0),
            started_at_unix_secs: 2,
            duration_ms: Some(30),
        });
        state.ui.workspace.open_command_palette("journal");

        let rows = command_palette_results(&state);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "History");
        assert_eq!(rows[0].subtitle, "history · Production API");
    }
}
