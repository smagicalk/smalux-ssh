//! 主机视觉配置覆盖。

use crate::model::{
    BackgroundProfile, HostId, ThemeProfile, VisualSettingsDraft, VisualSettingsDraftField,
};

use super::super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};
use super::outcome::{invalid_visual_settings, missing_host};

impl AppState {
    /// 更新某台主机的视觉覆盖草稿。
    pub(in crate::model::app_state) fn update_host_visual_settings_draft(
        &mut self,
        host_id: HostId,
        field: VisualSettingsDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        let Some((theme, background)) = self.host_visual_fallbacks(host_id) else {
            return missing_host(host_id);
        };

        self.ui
            .set_host_visual_settings_field(host_id, field, value, &theme, &background);
        draft_changed()
    }

    /// 更新某台主机的背景开关草稿。
    pub(in crate::model::app_state) fn set_host_visual_background_enabled(
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

    /// 应用某台主机的主题和背景覆盖。
    pub(in crate::model::app_state) fn apply_host_visual_settings(
        &mut self,
        host_id: HostId,
    ) -> AppUpdateOutcome {
        let Some(host) = self
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
        else {
            return missing_host(host_id);
        };
        let fallback_theme = host
            .theme_override
            .clone()
            .unwrap_or_else(|| self.config.theme.clone());
        let fallback_background = host
            .background_override
            .clone()
            .unwrap_or_else(|| self.config.background.clone());
        let draft = self
            .ui
            .host_visual_settings_for(host_id)
            .cloned()
            .unwrap_or_else(|| {
                VisualSettingsDraft::from_profiles(&fallback_theme, &fallback_background)
            });
        let theme = match draft.build_theme_profile(&fallback_theme) {
            Ok(theme) => theme,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };
        let background = match draft.build_background_profile(&fallback_background) {
            Ok(background) => background,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };

        let Some(host) = self
            .storage
            .hosts
            .iter_mut()
            .find(|host| host.id == host_id)
        else {
            return missing_host(host_id);
        };
        let changed = host.theme_override.as_ref() != Some(&theme)
            || host.background_override.as_ref() != Some(&background);
        host.theme_override = Some(theme);
        host.background_override = Some(background);
        self.ui.clear_host_visual_settings_draft(host_id);

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    /// 清除某台主机的主题和背景覆盖，恢复全局配置。
    pub(in crate::model::app_state) fn clear_host_visual_settings(
        &mut self,
        host_id: HostId,
    ) -> AppUpdateOutcome {
        let Some(host) = self
            .storage
            .hosts
            .iter_mut()
            .find(|host| host.id == host_id)
        else {
            return missing_host(host_id);
        };

        let changed = host.theme_override.is_some() || host.background_override.is_some();
        host.theme_override = None;
        host.background_override = None;
        self.ui.clear_host_visual_settings_draft(host_id);

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    fn host_visual_fallbacks(&self, host_id: HostId) -> Option<(ThemeProfile, BackgroundProfile)> {
        self.storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .map(|host| {
                (
                    host.theme_override
                        .clone()
                        .unwrap_or_else(|| self.config.theme.clone()),
                    host.background_override
                        .clone()
                        .unwrap_or_else(|| self.config.background.clone()),
                )
            })
    }
}
