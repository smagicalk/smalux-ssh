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
}

fn invalid_visual_settings(error: String) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("视觉配置无效：{error}")),
        ..AppUpdateOutcome::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ImageSource, Message};

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
}
