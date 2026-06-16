//! 桌面视觉设置适配逻辑。
//!
//! 这些方法仍属于桌面层，因为它们一边读写 `UiState` 草稿，一边调用核心持久化入口。

use crate::model::{AppUpdateOutcome, BackgroundProfile, HostId, ThemeProfile};

use super::{DesktopAppState, draft_changed, invalid_visual_settings, missing_host};

impl DesktopAppState {
    pub(super) fn apply_visual_settings_local(&mut self) -> AppUpdateOutcome {
        let draft = self.ui.visual_settings.clone();
        let theme = match draft.build_theme_profile(&self.core.config.theme) {
            Ok(theme) => theme,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };
        let background = match draft.build_background_profile(&self.core.config.background) {
            Ok(background) => background,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };

        let outcome = self.core.apply_visual_profiles_action(theme, background);
        self.sync_workspace_visuals_from_core(&outcome);
        outcome
    }

    pub(super) fn update_host_visual_settings_draft_local(
        &mut self,
        host_id: HostId,
        field: crate::model::VisualSettingsDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        let Some((theme, background)) = self.host_visual_fallbacks(host_id) else {
            return missing_host(host_id);
        };

        self.ui
            .set_host_visual_settings_field(host_id, field, value, &theme, &background);
        draft_changed()
    }

    pub(super) fn set_host_visual_background_enabled_local(
        &mut self,
        host_id: HostId,
        enabled: bool,
    ) -> AppUpdateOutcome {
        let Some((theme, background)) = self.host_visual_fallbacks(host_id) else {
            return missing_host(host_id);
        };

        self.ui
            .set_host_visual_background_enabled(host_id, enabled, &theme, &background);
        draft_changed()
    }

    pub(super) fn apply_host_visual_settings_local(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some((fallback_theme, fallback_background)) = self.host_visual_fallbacks(host_id)
        else {
            return missing_host(host_id);
        };
        let draft = self
            .ui
            .host_visual_settings_for(host_id)
            .cloned()
            .unwrap_or_else(|| {
                crate::model::VisualSettingsDraft::from_profiles(
                    &fallback_theme,
                    &fallback_background,
                )
            });
        let theme = match draft.build_theme_profile(&fallback_theme) {
            Ok(theme) => theme,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };
        let background = match draft.build_background_profile(&fallback_background) {
            Ok(background) => background,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };

        let outcome = self
            .core
            .apply_host_visual_profiles_action(host_id, theme, background);
        self.ui.clear_host_visual_settings_draft(host_id);
        outcome
    }

    pub(super) fn clear_host_visual_settings_local(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let outcome = self.core.clear_host_visual_profiles_action(host_id);
        self.ui.clear_host_visual_settings_draft(host_id);
        outcome
    }

    pub(super) fn host_visual_fallbacks(
        &self,
        host_id: HostId,
    ) -> Option<(ThemeProfile, BackgroundProfile)> {
        self.core
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .map(|host| {
                (
                    host.theme_override
                        .clone()
                        .unwrap_or_else(|| self.core.config.theme.clone()),
                    host.background_override
                        .clone()
                        .unwrap_or_else(|| self.core.config.background.clone()),
                )
            })
    }
}
