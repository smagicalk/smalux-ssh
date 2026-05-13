//! 后端执行器向状态层回传的事件。

use crate::model::{
    HostKeyVerification, SessionId, SftpEntry, TransferId, TransferStatus, TunnelStatus,
};

/// 后端执行器产生的状态事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    Connecting {
        session_id: SessionId,
        endpoint: String,
    },
    Connected {
        session_id: SessionId,
    },
    Authenticating {
        session_id: SessionId,
        username: String,
    },
    Authenticated {
        session_id: SessionId,
    },
    HostKeyVerified {
        session_id: SessionId,
        result: HostKeyVerification,
    },
    ShellOpened {
        session_id: SessionId,
    },
    RemoteCommandStarted {
        session_id: SessionId,
        command: String,
    },
    Output {
        session_id: SessionId,
        line: String,
    },
    CommandExited {
        session_id: SessionId,
        exit_code: Option<i32>,
    },
    SftpEntries {
        session_id: SessionId,
        remote_path: String,
        entries: Vec<SftpEntry>,
    },
    TransferProgress {
        session_id: SessionId,
        transfer_id: TransferId,
        transferred_bytes: u64,
        status: TransferStatus,
    },
    TunnelStatusChanged {
        session_id: SessionId,
        rule_name: String,
        status: TunnelStatus,
    },
    Failed {
        session_id: SessionId,
        reason: String,
    },
    Disconnected {
        session_id: SessionId,
    },
}

impl BackendEvent {
    /// 返回事件关联的会话标识。
    pub fn session_id(&self) -> SessionId {
        match self {
            Self::Connecting { session_id, .. }
            | Self::Connected { session_id }
            | Self::Authenticating { session_id, .. }
            | Self::Authenticated { session_id }
            | Self::HostKeyVerified { session_id, .. }
            | Self::ShellOpened { session_id }
            | Self::RemoteCommandStarted { session_id, .. }
            | Self::Output { session_id, .. }
            | Self::CommandExited { session_id, .. }
            | Self::SftpEntries { session_id, .. }
            | Self::TransferProgress { session_id, .. }
            | Self::TunnelStatusChanged { session_id, .. }
            | Self::Failed { session_id, .. }
            | Self::Disconnected { session_id } => *session_id,
        }
    }

    /// 判断事件是否代表终止态。
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Failed { .. } | Self::Disconnected { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn backend_event_reports_session_and_terminal_state() {
        let session_id = SessionId(Uuid::new_v4());
        let output = BackendEvent::Output {
            session_id,
            line: "hello".to_owned(),
        };
        let failed = BackendEvent::Failed {
            session_id,
            reason: "network".to_owned(),
        };

        assert_eq!(output.session_id(), session_id);
        assert!(!output.is_terminal());
        assert!(failed.is_terminal());
    }

    #[test]
    fn backend_connection_events_report_session() {
        let session_id = SessionId(Uuid::new_v4());
        let events = [
            BackendEvent::Connecting {
                session_id,
                endpoint: "example.com:22".to_owned(),
            },
            BackendEvent::Authenticating {
                session_id,
                username: "deploy".to_owned(),
            },
            BackendEvent::Authenticated { session_id },
            BackendEvent::ShellOpened { session_id },
            BackendEvent::RemoteCommandStarted {
                session_id,
                command: "uptime".to_owned(),
            },
        ];

        assert!(events.iter().all(|event| event.session_id() == session_id));
    }
}
