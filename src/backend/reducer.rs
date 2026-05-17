//! 后端事件到 UI 状态的归约逻辑。

use crate::model::{LOCAL_TERMINAL_SESSION_ID, SessionStatus, TunnelStatus};
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
        BackendEvent::Connecting { session_id, .. } => BackendEventOutcome {
            session_updated: sessions.set_status(session_id, SessionStatus::Connecting),
            terminal_updated: false,
        },
        BackendEvent::Connected { session_id } => BackendEventOutcome {
            session_updated: sessions.set_status(session_id, SessionStatus::Connected),
            terminal_updated: false,
        },
        BackendEvent::Authenticating { session_id, .. }
        | BackendEvent::HostKeyVerified { session_id, .. } => BackendEventOutcome {
            session_updated: sessions.set_status(session_id, SessionStatus::Authenticating),
            terminal_updated: false,
        },
        BackendEvent::Authenticated { session_id } | BackendEvent::ShellOpened { session_id } => {
            BackendEventOutcome {
                session_updated: sessions.set_status(session_id, SessionStatus::Connected),
                terminal_updated: false,
            }
        }
        BackendEvent::RemoteCommandStarted { session_id, .. } => BackendEventOutcome {
            session_updated: sessions.set_status(session_id, SessionStatus::RunningCommand),
            terminal_updated: false,
        },
        BackendEvent::Output { session_id, line } => {
            let is_duplicate_local_echo = session_id == LOCAL_TERMINAL_SESSION_ID
                && terminal.suppress_duplicate_echo(
                    session_id,
                    crate::backend::LocalShellProfile::default_for_platform().prompt,
                    &line,
                );

            BackendEventOutcome {
                session_updated: false,
                terminal_updated: if is_duplicate_local_echo {
                    false
                } else {
                    terminal.append_output(session_id, line)
                },
            }
        }
        BackendEvent::ClearTerminal { session_id } => BackendEventOutcome {
            session_updated: false,
            terminal_updated: terminal.clear_output(session_id),
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
            total_bytes,
            transferred_bytes,
            status,
            ..
        } => BackendEventOutcome {
            session_updated: sessions.update_transfer_progress(
                transfer_id,
                total_bytes,
                transferred_bytes,
                status,
            ),
            terminal_updated: false,
        },
        BackendEvent::SftpFailed { session_id, reason } => BackendEventOutcome {
            session_updated: sessions.fail_sftp_browser_for_session(session_id, reason),
            terminal_updated: false,
        },
        BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name,
            status,
        } => {
            let tunnel_updated = apply_tunnel_status(sessions, &rule_name, status.clone());
            let session_updated = match status {
                TunnelStatus::Running => sessions.set_status(session_id, SessionStatus::Connected),
                TunnelStatus::Stopped => {
                    sessions.set_status(session_id, SessionStatus::Disconnected)
                }
                TunnelStatus::Failed => sessions.set_status(
                    session_id,
                    SessionStatus::Failed {
                        reason: "backend tunnel failed".to_owned(),
                    },
                ),
                TunnelStatus::Starting | TunnelStatus::Stopping => false,
            };

            BackendEventOutcome {
                session_updated: tunnel_updated || session_updated,
                terminal_updated: false,
            }
        }
        BackendEvent::Failed { session_id, reason } => {
            let session_updated = sessions.set_status(
                session_id,
                SessionStatus::Failed {
                    reason: reason.clone(),
                },
            );
            let sftp_updated = sessions.fail_sftp_browser_for_session(session_id, reason.clone());
            let tunnel_updated = sessions.fail_tunnel_for_session(session_id, reason);

            BackendEventOutcome {
                session_updated: session_updated || sftp_updated || tunnel_updated,
                terminal_updated: false,
            }
        }
        BackendEvent::Disconnected { session_id } => {
            let session_updated = sessions.set_status(session_id, SessionStatus::Disconnected);
            let sftp_updated =
                sessions.fail_sftp_browser_for_session(session_id, "SFTP 会话已断开");

            BackendEventOutcome {
                session_updated: session_updated || sftp_updated,
                terminal_updated: false,
            }
        }
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
