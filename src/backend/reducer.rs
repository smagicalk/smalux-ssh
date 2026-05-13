//! 后端事件到 UI 状态的归约逻辑。

use crate::model::{SessionStatus, TunnelStatus};
use crate::session::SessionManager;
use crate::terminal::TerminalManager;

use super::BackendEvent;

/// 后端事件应用结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendEventOutcome {
    pub session_updated: bool,
    pub terminal_updated: bool,
}

impl BackendEventOutcome {
    /// 是否至少更新了一个状态管理器。
    pub fn changed(self) -> bool {
        self.session_updated || self.terminal_updated
    }
}

/// 将后端事件归约到会话和终端状态。
pub fn apply_backend_event(
    sessions: &mut SessionManager,
    terminal: &mut TerminalManager,
    event: BackendEvent,
) -> BackendEventOutcome {
    match event {
        BackendEvent::Connected { session_id } => BackendEventOutcome {
            session_updated: sessions.set_status(session_id, SessionStatus::Connected),
            terminal_updated: false,
        },
        BackendEvent::HostKeyVerified { session_id, .. } => BackendEventOutcome {
            session_updated: sessions.set_status(session_id, SessionStatus::Authenticating),
            terminal_updated: false,
        },
        BackendEvent::Output { session_id, line } => BackendEventOutcome {
            session_updated: false,
            terminal_updated: terminal.append_output(session_id, line),
        },
        BackendEvent::CommandExited {
            session_id,
            exit_code,
        } => {
            let status = match exit_code {
                Some(0) => SessionStatus::Disconnected,
                Some(code) => SessionStatus::Failed {
                    reason: format!("remote command exited with {code}"),
                },
                None => SessionStatus::Disconnected,
            };

            BackendEventOutcome {
                session_updated: sessions.set_status(session_id, status),
                terminal_updated: false,
            }
        }
        BackendEvent::SftpEntries {
            session_id,
            remote_path,
            entries,
        } => BackendEventOutcome {
            session_updated: sessions.set_sftp_entries_for_session(
                session_id,
                remote_path,
                entries,
            ),
            terminal_updated: false,
        },
        BackendEvent::TransferProgress {
            transfer_id,
            transferred_bytes,
            status,
            ..
        } => BackendEventOutcome {
            session_updated: sessions.update_transfer_progress(
                transfer_id,
                transferred_bytes,
                status,
            ),
            terminal_updated: false,
        },
        BackendEvent::TunnelStatusChanged {
            rule_name, status, ..
        } => BackendEventOutcome {
            session_updated: apply_tunnel_status(sessions, &rule_name, status),
            terminal_updated: false,
        },
        BackendEvent::Failed { session_id, reason } => {
            let session_updated = sessions.set_status(
                session_id,
                SessionStatus::Failed {
                    reason: reason.clone(),
                },
            );
            let sftp_updated = sessions.fail_sftp_browser_for_session(session_id, reason);

            BackendEventOutcome {
                session_updated: session_updated || sftp_updated,
                terminal_updated: false,
            }
        }
        BackendEvent::Disconnected { session_id } => BackendEventOutcome {
            session_updated: sessions.set_status(session_id, SessionStatus::Disconnected),
            terminal_updated: false,
        },
    }
}

fn apply_tunnel_status(
    sessions: &mut SessionManager,
    rule_name: &str,
    status: TunnelStatus,
) -> bool {
    if matches!(status, TunnelStatus::Failed) {
        sessions.fail_tunnel(rule_name, "backend tunnel failed")
    } else {
        sessions.set_tunnel_status(rule_name, status)
    }
}

#[cfg(test)]
mod tests;
