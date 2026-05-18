//! 后端执行命令模型。

use crate::model::{Host, HostId, KnownHostEntry, SessionId};

use super::{
    BackendAuth, PtyRequest, RemoteCommandRequest, SftpRequest, TunnelStartRequest,
    TunnelStopRequest,
};

/// 后端执行器接收的命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendCommand {
    Connect {
        session_id: SessionId,
        target: ConnectionTarget,
    },
    OpenShell {
        session_id: SessionId,
        pty: PtyRequest,
    },
    RunCommand {
        session_id: SessionId,
        request: RemoteCommandRequest,
    },
    SendShellInput {
        session_id: SessionId,
        input: String,
    },
    DrainSessionOutput {
        session_id: SessionId,
    },
    Sftp {
        session_id: SessionId,
        request: SftpRequest,
    },
    StartTunnel {
        session_id: SessionId,
        request: TunnelStartRequest,
    },
    StopTunnel {
        session_id: SessionId,
        request: TunnelStopRequest,
    },
    Disconnect {
        session_id: SessionId,
    },
}

/// SSH 连接目标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionTarget {
    pub host_id: HostId,
    pub address: String,
    pub port: u16,
    pub auth: BackendAuth,
    pub known_hosts: Vec<KnownHostEntry>,
}

impl ConnectionTarget {
    /// 从已保存主机配置生成后端连接目标。
    pub fn from_host(host: &Host) -> Self {
        Self::from_host_with_known_hosts(host, Vec::new())
    }

    /// 从已保存主机配置和 Known Hosts 快照生成后端连接目标。
    pub fn from_host_with_known_hosts(host: &Host, known_hosts: Vec<KnownHostEntry>) -> Self {
        Self {
            host_id: host.id,
            address: host.address.clone(),
            port: host.port,
            auth: BackendAuth::from(&host.auth),
            known_hosts,
        }
    }

    /// 返回 `host:port` 展示字符串。
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

impl BackendCommand {
    /// 返回命令关联的会话标识。
    pub fn session_id(&self) -> SessionId {
        match self {
            Self::Connect { session_id, .. }
            | Self::OpenShell { session_id, .. }
            | Self::RunCommand { session_id, .. }
            | Self::SendShellInput { session_id, .. }
            | Self::DrainSessionOutput { session_id }
            | Self::Sftp { session_id, .. }
            | Self::StartTunnel { session_id, .. }
            | Self::StopTunnel { session_id, .. }
            | Self::Disconnect { session_id } => *session_id,
        }
    }

    /// 返回命令类型，便于执行器路由和测试断言。
    pub fn kind(&self) -> BackendCommandKind {
        match self {
            Self::Connect { .. } => BackendCommandKind::Connect,
            Self::OpenShell { .. } => BackendCommandKind::OpenShell,
            Self::RunCommand { .. } => BackendCommandKind::RunCommand,
            Self::SendShellInput { .. } => BackendCommandKind::SendShellInput,
            Self::DrainSessionOutput { .. } => BackendCommandKind::DrainSessionOutput,
            Self::Sftp { .. } => BackendCommandKind::Sftp,
            Self::StartTunnel { .. } => BackendCommandKind::StartTunnel,
            Self::StopTunnel { .. } => BackendCommandKind::StopTunnel,
            Self::Disconnect { .. } => BackendCommandKind::Disconnect,
        }
    }
}

/// 后端命令类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendCommandKind {
    Connect,
    OpenShell,
    RunCommand,
    SendShellInput,
    DrainSessionOutput,
    Sftp,
    StartTunnel,
    StopTunnel,
    Disconnect,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthProfile, KeyAlgorithm, KnownHostEntry, SecretRef};
    use crate::terminal::TerminalSize;
    use uuid::Uuid;

    fn host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            tags: Vec::new(),
            address: "example.com".to_owned(),
            port: 2222,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                key_hint: Some("id_ed25519".to_owned()),
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    #[test]
    fn connection_target_keeps_host_identity_and_auth() {
        let host = host();

        let target = ConnectionTarget::from_host_with_known_hosts(
            &host,
            vec![KnownHostEntry::untrusted(
                "example.com",
                2222,
                KeyAlgorithm::Ed25519,
                "SHA256:demo",
            )],
        );

        assert_eq!(target.host_id, host.id);
        assert_eq!(target.endpoint(), "example.com:2222");
        assert_eq!(target.auth.username(), "deploy");
        assert_eq!(target.known_hosts.len(), 1);
    }

    #[test]
    fn backend_command_exposes_session_id() {
        let session_id = SessionId(Uuid::new_v4());
        let command = BackendCommand::OpenShell {
            session_id,
            pty: PtyRequest::xterm(TerminalSize::default()),
        };

        assert_eq!(command.session_id(), session_id);
        assert_eq!(command.kind(), BackendCommandKind::OpenShell);
    }

    #[test]
    fn shell_input_command_exposes_session_id() {
        let session_id = SessionId(Uuid::new_v4());
        let command = BackendCommand::SendShellInput {
            session_id,
            input: "ls".to_owned(),
        };

        assert_eq!(command.session_id(), session_id);
        assert_eq!(command.kind(), BackendCommandKind::SendShellInput);
    }

    #[test]
    fn password_target_does_not_inline_plain_secret() {
        let mut host = host();
        host.auth = AuthProfile::Password {
            username: "root".to_owned(),
            secret: SecretRef("password:root".to_owned()),
        };

        let target = ConnectionTarget::from_host(&host);

        assert!(matches!(
            target.auth,
            BackendAuth::Password {
                secret: SecretRef(ref value),
                ..
            } if value == "password:root"
        ));
    }

    #[test]
    fn drain_session_output_command_exposes_session_id() {
        let session_id = SessionId(Uuid::new_v4());
        let command = BackendCommand::DrainSessionOutput { session_id };

        assert_eq!(command.session_id(), session_id);
        assert_eq!(command.kind(), BackendCommandKind::DrainSessionOutput);
    }
}
