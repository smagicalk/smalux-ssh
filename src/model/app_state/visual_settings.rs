//! 全局视觉配置应用逻辑。

use crate::model::{VisualSettingsDraft, VisualSettingsDraftField};

use super::ui_drafts::draft_changed;
use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 更新全局视觉配置草稿。
    pub(super) fn update_visual_settings_draft(
        &mut self,
        field: VisualSettingsDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_visual_settings_field(field, value);
        draft_changed()
    }

    /// 更新全局背景开关草稿。
    pub(super) fn set_visual_background_enabled(&mut self, enabled: bool) -> AppUpdateOutcome {
        self.ui.set_visual_background_enabled(enabled);
        draft_changed()
    }

    /// 将视觉配置草稿应用到运行配置和持久化快照。
    pub(super) fn apply_visual_settings(&mut self) -> AppUpdateOutcome {
        let theme_before = self.config.theme.clone();
        let background_before = self.config.background.clone();
        let draft = self.ui.visual_settings.clone();

        let theme = match draft.build_theme_profile(&self.config.theme) {
            Ok(theme) => theme,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };
        let background = match draft.build_background_profile(&self.config.background) {
            Ok(background) => background,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };

        self.config.theme = theme;
        self.config.background = background;
        self.storage.app_config = self.config.clone();
        self.ui.visual_settings =
            VisualSettingsDraft::from_profiles(&self.config.theme, &self.config.background);

        AppUpdateOutcome {
            state_changed: self.config.theme != theme_before
                || self.config.background != background_before,
            ..AppUpdateOutcome::default()
        }
    }

    /// 更新某台主机的视觉覆盖草稿。
    pub(super) fn update_host_visual_settings_draft(
        &mut self,
        host_id: crate::model::HostId,
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
    pub(super) fn set_host_visual_background_enabled(
        &mut self,
        host_id: crate::model::HostId,
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
    pub(super) fn apply_host_visual_settings(
        &mut self,
        host_id: crate::model::HostId,
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
    pub(super) fn clear_host_visual_settings(
        &mut self,
        host_id: crate::model::HostId,
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

    fn host_visual_fallbacks(
        &self,
        host_id: crate::model::HostId,
    ) -> Option<(crate::model::ThemeProfile, crate::model::BackgroundProfile)> {
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

fn invalid_visual_settings(error: String) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("视觉配置无效：{error}")),
        ..AppUpdateOutcome::default()
    }
}

fn missing_host(host_id: crate::model::HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}
