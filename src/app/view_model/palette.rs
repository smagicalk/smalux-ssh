//! 命令面板展示模型。

mod history;
mod matching;

use crate::model::AppState;

use super::common::tags_label;
use history::command_history_subtitle;
use matching::{command_matches_host, command_matches_text};

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

#[cfg(test)]
mod tests;
