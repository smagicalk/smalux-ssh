//! 后端执行器向状态层回传的事件。

use crate::model::{
    HostKeyVerification, SessionId, SftpEntry, TransferId, TransferStatus, TunnelStatus,
};

/// 后端执行器产生的状态事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    Connected {
        session_id: SessionId,
    },
    HostKeyVerified {
        session_id: SessionId,
        result: HostKeyVerification,
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
            Self::Connected { session_id }
            | Self::HostKeyVerified { session_id, .. }
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
}
