//! 全局视觉配置草稿与应用。

use crate::core::CoreState;
use crate::model::{BackgroundProfile, ThemeProfile};

use super::super::AppUpdateOutcome;

impl CoreState {
    /// 应用已经过草稿校验的全局视觉配置。
    pub(crate) fn apply_visual_profiles_action(
        &mut self,
        theme: ThemeProfile,
        background: BackgroundProfile,
    ) -> AppUpdateOutcome {
        let theme_before = self.config.theme.clone();
        let background_before = self.config.background.clone();

        self.config.theme = theme;
        self.config.background = background;
        self.storage.app_config = self.config.clone();

        AppUpdateOutcome {
            state_changed: self.config.theme != theme_before
                || self.config.background != background_before,
            ..AppUpdateOutcome::default()
        }
    }
}
