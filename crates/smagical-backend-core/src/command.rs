//! 后端执行命令模型。
//!
//! 后端命令是状态层和执行器之间的唯一协议。状态层只排队这些命令，执行器只返回
//! `BackendEvent`，两边不互相读取内部结构，从而保持 UI、状态和 IO 解耦。

use smagical_core::{Host, HostId, KnownHostEntry, SessionId};

use super::{
    BackendAuth, PtyRequest, RemoteCommandRequest, SftpRequest, TunnelStartRequest,
    TunnelStopRequest,
};

/// 后端执行器接收的命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendCommand {
    /// 建立远程 SSH 连接。
    Connect {
        session_id: SessionId,
        target: ConnectionTarget,
    },
    /// 在已连接 SSH 会话上打开交互式 shell。
    OpenShell {
        session_id: SessionId,
        pty: PtyRequest,
    },
    /// 打开本地 PTY shell。
    OpenLocalShell {
        session_id: SessionId,
        pty: PtyRequest,
    },
    /// 执行一次性远程命令。
    RunCommand {
        session_id: SessionId,
        request: RemoteCommandRequest,
    },
    /// 向交互式 shell 写入输入。
    SendShellInput {
        session_id: SessionId,
        input: String,
    },
    /// 抽取交互式 shell 的增量输出。
    DrainSessionOutput { session_id: SessionId },
    /// 执行 SFTP 请求。
    Sftp {
        session_id: SessionId,
        request: SftpRequest,
    },
    /// 启动 SSH 隧道。
    StartTunnel {
        session_id: SessionId,
        request: TunnelStartRequest,
    },
    /// 停止 SSH 隧道。
    StopTunnel {
        session_id: SessionId,
        request: TunnelStopRequest,
    },
    /// 断开会话并释放后端资源。
    Disconnect { session_id: SessionId },
}

/// SSH 连接目标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionTarget {
    /// 对应保存主机 ID，用于回写历史和状态。
    pub host_id: HostId,
    /// SSH 目标地址。
    pub address: String,
    /// SSH 端口。
    pub port: u16,
    /// 后端可解析的认证引用。
    pub auth: BackendAuth,
    /// 连接时使用的 Known Hosts 快照。
    pub known_hosts: Vec<KnownHostEntry>,
}

impl ConnectionTarget {
    /// 从已保存主机配置生成后端连接目标。
    pub fn from_host(host: &Host) -> Self {
        // 默认不带 known_hosts，测试和简单调用可使用；正式连接通常使用 with_known_hosts。
        Self::from_host_with_known_hosts(host, Vec::new())
    }

    /// 从已保存主机配置和 Known Hosts 快照生成后端连接目标。
    pub fn from_host_with_known_hosts(host: &Host, known_hosts: Vec<KnownHostEntry>) -> Self {
        // 这里只做数据转换，不读取 SecretRef 指向的明文。
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
            | Self::OpenLocalShell { session_id, .. }
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
            Self::OpenShell { .. } | Self::OpenLocalShell { .. } => BackendCommandKind::OpenShell,
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
    /// 建立连接。
    Connect,
    /// 打开 shell，本地和远程 shell 共用一种 kind。
    OpenShell,
    /// 执行远程命令。
    RunCommand,
    /// 发送 shell 输入。
    SendShellInput,
    /// 抽取 shell 输出。
    DrainSessionOutput,
    /// SFTP 请求。
    Sftp,
    /// 启动隧道。
    StartTunnel,
    /// 停止隧道。
    StopTunnel,
    /// 断开连接。
    Disconnect,
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::{
        AgentSource, AuthProfile, KeyAlgorithm, KnownHostEntry, SecretRef, TunnelKind, TunnelRule,
    };
    use smagical_terminal::TerminalSize;
    use uuid::Uuid;

    fn host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            icon_key: "server".to_owned(),
            tags: Vec::new(),
            address: "example.com".to_owned(),
            port: 2222,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                source: AgentSource::Auto,
                key_hint: Some("id_ed25519".to_owned()),
            },
            proxies: Vec::new(),
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
    fn local_shell_command_exposes_session_id() {
        let session_id = SessionId(Uuid::new_v4());
        let command = BackendCommand::OpenLocalShell {
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

    #[test]
    fn backend_commands_expose_session_id_and_kind_for_all_routes() {
        let session_id = SessionId(Uuid::new_v4());
        let tunnel_request = TunnelStartRequest::new(TunnelRule {
            name: "proxy".to_owned(),
            kind: TunnelKind::Dynamic,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 1080,
            target_host: String::new(),
            target_port: 0,
            auto_start: false,
        })
        .expect("动态隧道规则应该有效");
        let commands = vec![
            (
                BackendCommand::Connect {
                    session_id,
                    target: ConnectionTarget::from_host(&host()),
                },
                BackendCommandKind::Connect,
            ),
            (
                BackendCommand::RunCommand {
                    session_id,
                    request: RemoteCommandRequest::exec("uptime"),
                },
                BackendCommandKind::RunCommand,
            ),
            (
                BackendCommand::Sftp {
                    session_id,
                    request: SftpRequest::ListDir {
                        remote_path: "/".to_owned(),
                    },
                },
                BackendCommandKind::Sftp,
            ),
            (
                BackendCommand::StartTunnel {
                    session_id,
                    request: tunnel_request,
                },
                BackendCommandKind::StartTunnel,
            ),
            (
                BackendCommand::StopTunnel {
                    session_id,
                    request: TunnelStopRequest::by_name("proxy"),
                },
                BackendCommandKind::StopTunnel,
            ),
            (
                BackendCommand::Disconnect { session_id },
                BackendCommandKind::Disconnect,
            ),
        ];

        for (command, expected_kind) in commands {
            assert_eq!(command.session_id(), session_id);
            assert_eq!(command.kind(), expected_kind);
        }
    }
}
