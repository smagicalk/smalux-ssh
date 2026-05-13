//! 会话、SFTP 和隧道运行态概览视图。

use iced::{
    Element,
    widget::{button, column, row, text},
};

use crate::model::{
    AppState, Message, SessionKind, TransferDirection, TransferStatus, TunnelStatus,
};

/// 渲染最小会话概览和隧道停止入口。
pub fn view(state: &AppState) -> Element<'_, Message> {
    column![
        session_tabs(state),
        transfers(state),
        tunnel_runtime(state),
        recent_connections(state),
        command_history(state),
    ]
    .spacing(12)
    .into()
}

fn session_tabs(state: &AppState) -> Element<'_, Message> {
    let mut tabs = column![text("Sessions").size(22)].spacing(8);

    if state.sessions.tabs.is_empty() {
        return tabs.push(text("No active session tabs.")).into();
    }

    for tab in &state.sessions.tabs {
        let line = format!("{} | {:?} | {:?}", tab.title, tab.kind, tab.status);
        let mut row = row![text(line)].spacing(8);

        if let SessionKind::Tunnel { rule_name } = &tab.kind {
            row = row.push(button("Stop").on_press(Message::StopTunnel {
                session_id: tab.id,
                rule_name: rule_name.clone(),
            }));
        }

        tabs = tabs.push(row);
    }

    tabs.into()
}

fn transfers(state: &AppState) -> Element<'_, Message> {
    let mut transfers = column![text("Transfers").size(22)].spacing(8);

    if state.sessions.transfers.is_empty() {
        return transfers.push(text("No transfer tasks.")).into();
    }

    for task in &state.sessions.transfers {
        transfers = transfers.push(text(format!(
            "{} | {} | {} -> {} | {} | {}",
            transfer_direction_label(&task.direction),
            transfer_status_label(&task.status),
            task.local_path,
            task.remote_path,
            task.transferred_bytes,
            task.progress()
        )));
    }

    transfers.into()
}

fn tunnel_runtime(state: &AppState) -> Element<'_, Message> {
    let mut tunnels = column![text("Tunnel runtime").size(22)].spacing(8);

    if state.sessions.tunnels.is_empty() {
        return tunnels.push(text("No running tunnel state.")).into();
    }

    for tunnel in &state.sessions.tunnels {
        tunnels = tunnels.push(text(format!(
            "{} | {} | {}",
            tunnel.rule_name,
            tunnel_status_label(&tunnel.status),
            tunnel.last_error.as_deref().unwrap_or("ok")
        )));
    }

    tunnels.into()
}

fn recent_connections(state: &AppState) -> Element<'_, Message> {
    let mut recent = column![text("Recent").size(22)].spacing(8);

    if state.storage.recent_connections.is_empty() {
        return recent.push(text("No recent connections.")).into();
    }

    for item in state.storage.recent_connections.iter().take(5) {
        recent = recent.push(text(format!(
            "{} | {}",
            item.label, item.connected_at_unix_secs
        )));
    }

    recent.into()
}

fn command_history(state: &AppState) -> Element<'_, Message> {
    let mut history = column![text("Command history").size(22)].spacing(8);

    if state.storage.command_history.is_empty() {
        return history.push(text("No command history.")).into();
    }

    for item in state.storage.command_history.iter().rev().take(5) {
        history = history.push(text(format!("{} | {:?}", item.command, item.exit_code)));
    }

    history.into()
}

fn tunnel_status_label(status: &TunnelStatus) -> &'static str {
    match status {
        TunnelStatus::Stopped => "stopped",
        TunnelStatus::Starting => "starting",
        TunnelStatus::Running => "running",
        TunnelStatus::Stopping => "stopping",
        TunnelStatus::Failed => "failed",
    }
}

fn transfer_direction_label(direction: &TransferDirection) -> &'static str {
    match direction {
        TransferDirection::Upload => "upload",
        TransferDirection::Download => "download",
    }
}

fn transfer_status_label(status: &TransferStatus) -> String {
    match status {
        TransferStatus::Queued => "queued".to_owned(),
        TransferStatus::Running => "running".to_owned(),
        TransferStatus::Completed => "completed".to_owned(),
        TransferStatus::Failed { reason } => format!("failed: {reason}"),
        TransferStatus::Cancelled => "cancelled".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        HostId, SessionId, SessionStatus, SessionTab, TunnelRuntimeState, TunnelStatus,
    };
    use uuid::Uuid;

    #[test]
    fn tunnel_status_labels_are_stable() {
        assert_eq!(tunnel_status_label(&TunnelStatus::Starting), "starting");
        assert_eq!(tunnel_status_label(&TunnelStatus::Failed), "failed");
    }

    #[test]
    fn summary_view_accepts_tunnel_session_state() {
        let mut state = AppState::default();
        let session_id = SessionId(Uuid::new_v4());
        state.sessions.tabs.push(SessionTab {
            id: session_id,
            host_id: Some(HostId(Uuid::new_v4())),
            kind: SessionKind::Tunnel {
                rule_name: "local-db".to_owned(),
            },
            title: "local-db".to_owned(),
            status: SessionStatus::Connected,
        });
        state.sessions.tunnels.push(TunnelRuntimeState {
            rule_name: "local-db".to_owned(),
            host_id: None,
            status: TunnelStatus::Running,
            started_at_unix_secs: Some(1),
            last_error: None,
        });

        let _element = view(&state);
    }
}
