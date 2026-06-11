//! 快速新增主机 UI 草稿字段更新。

use crate::model::{ForwardId, GroupId, JumpChainId, ProxyId, UiState};

use super::{
    QuickHostAgentSource, QuickHostAuthField, QuickHostAuthKind, QuickHostDraft,
    QuickHostDraftField, truncate_host_name,
};

impl UiState {
    /// 更新快速新增主机表单字段。
    pub fn set_quick_host_field(&mut self, field: QuickHostDraftField, value: impl Into<String>) {
        let value = value.into();

        match field {
            QuickHostDraftField::Name => self.quick_host.name = truncate_host_name(value.trim()),
            QuickHostDraftField::Address => self.quick_host.address = value,
            QuickHostDraftField::Port => self.quick_host.port = value,
            QuickHostDraftField::Username => self.quick_host.username = value,
            QuickHostDraftField::Tags => self.quick_host.tags = value,
            QuickHostDraftField::IconKey => self.quick_host.icon_key = value,
        }
    }

    /// 更新快速新增主机所属分组。
    pub fn select_quick_host_group(&mut self, group_id: Option<GroupId>) {
        self.quick_host.group_id = group_id;
    }

    /// 更新快速新增主机的认证方式。
    pub fn set_quick_host_auth_kind(&mut self, kind: QuickHostAuthKind) {
        self.quick_host.auth.kind = kind;
    }

    /// 更新快速新增主机的认证字段。
    pub fn set_quick_host_auth_field(
        &mut self,
        field: QuickHostAuthField,
        value: impl Into<String>,
    ) {
        let value = value.into();

        match field {
            QuickHostAuthField::AgentSource => {
                self.quick_host.auth.agent_source = parse_agent_source(&value)
            }
            QuickHostAuthField::AgentCustomPipe => self.quick_host.auth.agent_custom_pipe = value,
            QuickHostAuthField::PasswordSecretRef => {
                self.quick_host.auth.password_secret_ref = value
            }
            QuickHostAuthField::PrivateKeyRef => self.quick_host.auth.private_key_ref = value,
            QuickHostAuthField::PassphraseRef => self.quick_host.auth.passphrase_ref = value,
            QuickHostAuthField::KeyHint => self.quick_host.auth.key_hint = value,
            QuickHostAuthField::CertificateRef => self.quick_host.auth.certificate_ref = value,
        }
    }

    /// 切换快速新增主机使用的代理资产。
    pub fn toggle_quick_host_proxy(&mut self, proxy_id: ProxyId) {
        toggle_id(&mut self.quick_host.network.proxy_ids, proxy_id);
    }

    /// 切换快速新增主机使用的跳板链资产。
    pub fn toggle_quick_host_jump_chain(&mut self, chain_id: JumpChainId) {
        toggle_id(&mut self.quick_host.network.jump_chain_ids, chain_id);
    }

    /// 切换快速新增主机绑定的端口转发资产。
    pub fn toggle_quick_host_forward(&mut self, forward_id: ForwardId) {
        toggle_id(&mut self.quick_host.network.forward_ids, forward_id);
    }

    /// 清空快速新增主机表单，保留默认 SSH 端口。
    pub fn reset_quick_host(&mut self) {
        self.quick_host = QuickHostDraft::default();
    }

    /// 用已保存主机填充表单，进入编辑模式。
    pub fn edit_quick_host(&mut self, host: &crate::model::Host) {
        self.quick_host = QuickHostDraft::from_host(host);
    }
}

fn parse_agent_source(value: &str) -> QuickHostAgentSource {
    match value {
        "OpenSSH" => QuickHostAgentSource::OpenSsh,
        "Pageant" => QuickHostAgentSource::Pageant,
        "Custom" => QuickHostAgentSource::CustomNamedPipe,
        _ => QuickHostAgentSource::Auto,
    }
}

fn toggle_id<T: PartialEq>(items: &mut Vec<T>, id: T) {
    if let Some(index) = items.iter().position(|item| item == &id) {
        items.remove(index);
    } else {
        items.push(id);
    }
}
