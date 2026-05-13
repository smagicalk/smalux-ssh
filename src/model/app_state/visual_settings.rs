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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthProfile, Host, HostId, ImageSource, Message};
    use uuid::Uuid;

    fn sample_host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            tags: Vec::new(),
            address: "prod.example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                key_hint: None,
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    #[test]
    fn visual_settings_messages_update_draft_and_apply_config() {
        let mut state = AppState::default();

        state.apply(Message::UpdateVisualSettingsDraft {
            field: VisualSettingsDraftField::ThemeName,
            value: "Solarized Dark".to_owned(),
        });
        state.apply(Message::UpdateVisualSettingsDraft {
            field: VisualSettingsDraftField::FontFamily,
            value: "Maple Mono".to_owned(),
        });
        state.apply(Message::UpdateVisualSettingsDraft {
            field: VisualSettingsDraftField::FontSize,
            value: "16".to_owned(),
        });
        state.apply(Message::SetVisualBackgroundEnabled { enabled: true });
        state.apply(Message::UpdateVisualSettingsDraft {
            field: VisualSettingsDraftField::BackgroundSources,
            value: "wallpapers/a.jpg, url:https://example.com/b.jpg".to_owned(),
        });
        state.apply(Message::UpdateVisualSettingsDraft {
            field: VisualSettingsDraftField::RotationIntervalSecs,
            value: "120".to_owned(),
        });
        state.apply(Message::UpdateVisualSettingsDraft {
            field: VisualSettingsDraftField::Opacity,
            value: "0.4".to_owned(),
        });
        state.apply(Message::UpdateVisualSettingsDraft {
            field: VisualSettingsDraftField::Blur,
            value: "12".to_owned(),
        });

        let outcome = state.apply(Message::ApplyVisualSettings);

        assert!(outcome.changed());
        assert_eq!(state.config.theme.name, "Solarized Dark");
        assert_eq!(state.config.theme.font_family, "Maple Mono");
        assert_eq!(state.config.theme.font_size, 16.0);
        assert!(state.config.background.enabled);
        assert_eq!(state.config.background.rotation_interval_secs, 120);
        assert_eq!(state.config.background.opacity, 0.4);
        assert_eq!(state.config.background.blur, 12.0);
        assert_eq!(
            state.config.background.sources,
            vec![
                ImageSource::LocalPath("wallpapers/a.jpg".to_owned()),
                ImageSource::Url("https://example.com/b.jpg".to_owned()),
            ]
        );
        assert_eq!(state.storage.app_config, state.config);
    }

    #[test]
    fn invalid_visual_settings_report_error_without_changing_config() {
        let mut state = AppState::default();
        let before = state.config.clone();
        state.apply(Message::UpdateVisualSettingsDraft {
            field: VisualSettingsDraftField::FontSize,
            value: "zero".to_owned(),
        });

        let outcome = state.apply(Message::ApplyVisualSettings);

        assert!(outcome.error.is_some());
        assert_eq!(state.config, before);
        assert_eq!(state.storage.app_config, before);
    }

    #[test]
    fn host_visual_settings_apply_and_clear_host_overrides() {
        let mut state = AppState::default();
        let host = sample_host();
        let host_id = host.id;
        state.storage.upsert_host(host);

        state.apply(Message::UpdateHostVisualSettingsDraft {
            host_id,
            field: VisualSettingsDraftField::ThemeName,
            value: "Prod Dark".to_owned(),
        });
        state.apply(Message::SetHostVisualBackgroundEnabled {
            host_id,
            enabled: true,
        });
        state.apply(Message::UpdateHostVisualSettingsDraft {
            host_id,
            field: VisualSettingsDraftField::BackgroundSources,
            value: "wallpapers/prod.jpg".to_owned(),
        });

        let apply_outcome = state.apply(Message::ApplyHostVisualSettings { host_id });

        assert!(apply_outcome.changed());
        assert_eq!(
            state.storage.hosts[0]
                .theme_override
                .as_ref()
                .map(|theme| theme.name.as_str()),
            Some("Prod Dark")
        );
        assert_eq!(
            state.storage.hosts[0]
                .background_override
                .as_ref()
                .map(|background| background.sources.clone()),
            Some(vec![ImageSource::LocalPath(
                "wallpapers/prod.jpg".to_owned()
            )])
        );
        assert!(state.ui.host_visual_settings_for(host_id).is_none());

        let clear_outcome = state.apply(Message::ClearHostVisualSettings { host_id });

        assert!(clear_outcome.changed());
        assert!(state.storage.hosts[0].theme_override.is_none());
        assert!(state.storage.hosts[0].background_override.is_none());
    }

    #[test]
    fn host_visual_settings_report_missing_host() {
        let mut state = AppState::default();
        let missing_host_id = HostId(Uuid::new_v4());

        let outcome = state.apply(Message::ApplyHostVisualSettings {
            host_id: missing_host_id,
        });

        assert!(outcome.error.is_some());
    }
}
