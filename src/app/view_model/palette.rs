//! 命令面板展示模型。

mod history;
mod matching;

use crate::app::state::AsDesktopStateView;

use super::common::tags_label;
use super::i18n::{locale_for_state, tr};
use history::command_history_subtitle;
use matching::{command_matches_host, command_matches_text};

/// 命令面板结果行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct CommandPaletteItemViewModel {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub kind_key: &'static str,
    pub kind: &'static str,
}

pub(super) fn command_palette_results(
    state: impl AsDesktopStateView,
) -> Vec<CommandPaletteItemViewModel> {
    let state = state.as_desktop_state_view();
    let query = state
        .ui
        .workspace
        .command_palette
        .query
        .trim()
        .to_lowercase();
    let mut rows = Vec::new();
    let locale = locale_for_state(state);

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
                subtitle: format!(
                    "{}:{} · {}",
                    host.address,
                    host.port,
                    tags_label(state, host)
                ),
                kind_key: "Host",
                kind: tr(locale, "palette.kind.host"),
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
                subtitle: tr(locale, "palette.recent_subtitle").to_owned(),
                kind_key: "Recent",
                kind: tr(locale, "palette.kind.recent"),
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
                kind_key: "History",
                kind: tr(locale, "palette.kind.history"),
            }),
    );

    rows
}

#[cfg(test)]
mod tests;
