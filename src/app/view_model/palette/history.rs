use crate::model::{AppState, HostId};

pub(super) fn command_history_subtitle(state: &AppState, host_id: Option<HostId>) -> String {
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
