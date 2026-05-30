use crate::model::{AppState, HostId};

use super::super::i18n::{locale_for_state, tr};

pub(super) fn command_history_subtitle(state: &AppState, host_id: Option<HostId>) -> String {
    let locale = locale_for_state(state);
    let Some(host_id) = host_id else {
        return tr(locale, "palette.history_global").to_owned();
    };

    state
        .storage
        .hosts
        .iter()
        .find(|host| host.id == host_id)
        .map(|host| format!("{} · {}", tr(locale, "palette.history_prefix"), host.name))
        .unwrap_or_else(|| tr(locale, "palette.history_deleted_host").to_owned())
}
