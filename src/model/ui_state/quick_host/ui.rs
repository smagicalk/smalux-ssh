//! 快速新增主机 UI 草稿字段更新。

use crate::model::UiState;

use super::{QuickHostAuthField, QuickHostAuthKind, QuickHostDraft, QuickHostDraftField};

impl UiState {
    /// 更新快速新增主机表单字段。
    pub fn set_quick_host_field(&mut self, field: QuickHostDraftField, value: impl Into<String>) {
        let value = value.into();

        match field {
            QuickHostDraftField::Name => self.quick_host.name = value,
            QuickHostDraftField::Address => self.quick_host.address = value,
            QuickHostDraftField::Port => self.quick_host.port = value,
            QuickHostDraftField::Username => self.quick_host.username = value,
            QuickHostDraftField::Tags => self.quick_host.tags = value,
        }
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
            QuickHostAuthField::PasswordSecretRef => {
                self.quick_host.auth.password_secret_ref = value
            }
            QuickHostAuthField::PrivateKeyRef => self.quick_host.auth.private_key_ref = value,
            QuickHostAuthField::PassphraseRef => self.quick_host.auth.passphrase_ref = value,
            QuickHostAuthField::KeyHint => self.quick_host.auth.key_hint = value,
            QuickHostAuthField::CertificateRef => self.quick_host.auth.certificate_ref = value,
        }
    }

    /// 清空快速新增主机表单，保留默认 SSH 端口。
    pub fn reset_quick_host(&mut self) {
        self.quick_host = QuickHostDraft::default();
    }
}
