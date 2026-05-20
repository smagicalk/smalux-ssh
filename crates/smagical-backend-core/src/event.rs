//! 后端执行器向状态层回传的事件。

use smagical_core::{
    HostKeyVerification, KeyAlgorithm, SessionId, SftpEntry, TransferId, TransferStatus,
    TunnelStatus,
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
        host: String,
        port: u16,
        key_algorithm: KeyAlgorithm,
        fingerprint: String,
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
    ClearTerminal {
        session_id: SessionId,
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
        total_bytes: Option<u64>,
        transferred_bytes: u64,
        status: TransferStatus,
    },
    SftpFailed {
        session_id: SessionId,
        reason: String,
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
            | Self::ClearTerminal { session_id }
            | Self::CommandExited { session_id, .. }
            | Self::SftpEntries { session_id, .. }
            | Self::TransferProgress { session_id, .. }
            | Self::SftpFailed { session_id, .. }
            | Self::TunnelStatusChanged { session_id, .. }
            | Self::Failed { session_id, .. }
            | Self::Disconnected { session_id } => *session_id,
        }
    }

    /// 判断事件是否代表终止态。
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::CommandExited { .. } | Self::Failed { .. } | Self::Disconnected { .. }
        )
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
        let exited = BackendEvent::CommandExited {
            session_id,
            exit_code: Some(0),
        };

        assert_eq!(output.session_id(), session_id);
        assert!(!output.is_terminal());
        assert!(exited.is_terminal());
        assert!(failed.is_terminal());
    }

    #[test]
    fn sftp_failed_event_is_not_terminal() {
        let session_id = SessionId(Uuid::new_v4());
        let event = BackendEvent::SftpFailed {
            session_id,
            reason: "permission denied".to_owned(),
        };

        assert_eq!(event.session_id(), session_id);
        assert!(!event.is_terminal());
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

    #[test]
    fn backend_data_events_report_session_without_becoming_terminal() {
        let session_id = SessionId(Uuid::new_v4());
        let transfer_id = TransferId(Uuid::new_v4());
        let events = [
            BackendEvent::HostKeyVerified {
                session_id,
                host: "example.com".to_owned(),
                port: 22,
                key_algorithm: KeyAlgorithm::Ed25519,
                fingerprint: "SHA256:demo".to_owned(),
                result: HostKeyVerification::Trusted,
            },
            BackendEvent::SftpEntries {
                session_id,
                remote_path: "/home/ops".to_owned(),
                entries: vec![SftpEntry {
                    name: "deploy.sh".to_owned(),
                    remote_path: "/home/ops/deploy.sh".to_owned(),
                    kind: smagical_core::SftpEntryKind::File,
                    size: Some(2048),
                    modified_at_unix_secs: Some(1_700_000_000),
                    permissions: Some(0o755),
                }],
            },
            BackendEvent::TransferProgress {
                session_id,
                transfer_id,
                total_bytes: Some(4096),
                transferred_bytes: 1024,
                status: TransferStatus::Running,
            },
            BackendEvent::TunnelStatusChanged {
                session_id,
                rule_name: "proxy".to_owned(),
                status: TunnelStatus::Running,
            },
        ];

        assert!(events.iter().all(|event| event.session_id() == session_id));
        assert!(events.iter().all(|event| !event.is_terminal()));
    }

    #[test]
    fn disconnected_event_is_terminal() {
        let session_id = SessionId(Uuid::new_v4());
        let event = BackendEvent::Disconnected { session_id };

        assert_eq!(event.session_id(), session_id);
        assert!(event.is_terminal());
    }
}
