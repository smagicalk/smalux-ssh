//! 桌面适配层的本地同步辅助。
//!
//! 这里只放“核心状态变化后，把结果同步回桌面草稿”的逻辑，避免散落在消息分发主文件里。

use crate::config::HostListModePreference;
use crate::model::{AppUpdateOutcome, CredentialKind, QuickHostAuthField};

use super::DesktopAppState;

impl DesktopAppState {
    pub(super) fn sync_quick_host_credential_ref(
        &mut self,
        kind: CredentialKind,
        credential_name: &str,
        outcome: &AppUpdateOutcome,
    ) {
        if !outcome.changed() {
            return;
        }

        let auth_field = match kind {
            CredentialKind::Password => QuickHostAuthField::PasswordSecretRef,
            CredentialKind::PrivateKey => QuickHostAuthField::PrivateKeyRef,
            CredentialKind::Certificate => QuickHostAuthField::CertificateRef,
            CredentialKind::Agent => return,
        };

        let Some(secret_ref) = self
            .core
            .storage
            .credentials
            .iter()
            .find(|credential| credential.name == credential_name && credential.kind == kind)
            .and_then(|credential| credential.secret.as_ref())
        else {
            return;
        };

        self.ui
            .set_quick_host_auth_field(auth_field, secret_ref.0.as_str());
    }

    pub(super) fn sync_workspace_visuals_from_core(&mut self, outcome: &AppUpdateOutcome) {
        if !outcome.state_changed {
            return;
        }

        self.ui.visual_settings = crate::model::VisualSettingsDraft::from_profiles(
            &self.core.config.theme,
            &self.core.config.background,
        );
    }

    pub(super) fn sync_workspace_state_from_core_import(&mut self, outcome: &AppUpdateOutcome) {
        if !outcome.state_changed {
            return;
        }

        self.sync_workspace_visuals_from_core(outcome);
        self.ui.workspace.host_list_mode = match self.core.config.workspace.host_list_mode {
            HostListModePreference::Tree => crate::model::HostListMode::Tree,
            HostListModePreference::Card => crate::model::HostListMode::Card,
        };
        self.ui.workspace.language =
            crate::model::LanguageMode::from_preference(self.core.config.workspace.language);
        self.ui.workspace.theme =
            crate::model::BuiltInTheme::from_preference(self.core.config.workspace.built_in_theme);
    }
}
