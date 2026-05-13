//! 工作区保存和恢复视图。

use iced::{
    Element,
    widget::{button, column, row, text},
};

use crate::model::{AppState, Message, WorkspaceLayoutNode};

/// 渲染工作区快照操作入口。
pub fn view(state: &AppState) -> Element<'_, Message> {
    column![
        text("Workspace").size(22),
        workspace_summary(state),
        row![
            button("Save workspace").on_press(Message::SaveWorkspaceSnapshot),
            button("Restore workspace").on_press(Message::RestoreWorkspaceSnapshot),
            button("Clear workspace").on_press(Message::ClearWorkspaceSnapshot),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .into()
}

fn workspace_summary(state: &AppState) -> Element<'_, Message> {
    let Some(workspace) = &state.storage.workspace else {
        return text("No saved workspace.").into();
    };

    text(format!(
        "{} | tabs: {} | active: {} | layout leaves: {}",
        workspace.name,
        workspace.tabs.len(),
        workspace
            .active_tab
            .map(|session_id| session_id.0.to_string())
            .unwrap_or_else(|| "-".to_owned()),
        workspace
            .layout
            .as_ref()
            .map(WorkspaceLayoutNode::leaf_count)
            .unwrap_or(0),
    ))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SessionId, SessionKind, WorkspaceState, WorkspaceTabSnapshot};
    use uuid::Uuid;

    #[test]
    fn workspace_view_accepts_empty_state() {
        let state = AppState::default();

        let _element = view(&state);
    }

    #[test]
    fn workspace_view_accepts_saved_snapshot() {
        let mut state = AppState::default();
        let session_id = SessionId(Uuid::new_v4());
        let mut workspace = WorkspaceState::empty("restore");
        workspace.upsert_tab(WorkspaceTabSnapshot {
            session_id,
            host_id: None,
            kind: SessionKind::Shell,
            title: "shell".to_owned(),
            working_directory: None,
        });
        workspace.rebuild_linear_layout(crate::model::SplitAxis::Horizontal);
        state.storage.save_workspace(workspace);

        let _element = view(&state);
    }
}
