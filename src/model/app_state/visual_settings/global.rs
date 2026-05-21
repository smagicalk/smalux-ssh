//! 全局视觉配置草稿与应用。

use crate::model::{VisualSettingsDraft, VisualSettingsDraftField};

use super::super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};
use super::outcome::invalid_visual_settings;

impl AppState {
    /// 更新全局视觉配置草稿。
    pub(in crate::model::app_state) fn update_visual_settings_draft(
        &mut self,
        field: VisualSettingsDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_visual_settings_field(field, value);
        draft_changed()
    }

    /// 更新全局背景开关草稿。
    pub(in crate::model::app_state) fn set_visual_background_enabled(
        &mut self,
        enabled: bool,
    ) -> AppUpdateOutcome {
        self.ui.set_visual_background_enabled(enabled);
        draft_changed()
    }

    /// 将视觉配置草稿应用到运行配置和持久化快照。
    pub(in crate::model::app_state) fn apply_visual_settings(&mut self) -> AppUpdateOutcome {
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
