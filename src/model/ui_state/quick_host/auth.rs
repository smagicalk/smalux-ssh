//! 快速新增主机认证方式的展示标签。

use super::{QuickHostAgentSource, QuickHostAuthKind};

pub(super) fn quick_host_agent_source_label(source: QuickHostAgentSource) -> &'static str {
    match source {
        QuickHostAgentSource::Auto => "Auto",
        QuickHostAgentSource::OpenSsh => "OpenSSH",
        QuickHostAgentSource::Pageant => "Pageant",
        QuickHostAgentSource::CustomNamedPipe => "Custom",
    }
}

pub(super) fn quick_host_auth_kind_label(kind: QuickHostAuthKind) -> &'static str {
    match kind {
        QuickHostAuthKind::Password => "Password",
        QuickHostAuthKind::Key => "Key",
        QuickHostAuthKind::Agent => "ssh-agent",
        QuickHostAuthKind::Certificate => "Certificate",
    }
}
