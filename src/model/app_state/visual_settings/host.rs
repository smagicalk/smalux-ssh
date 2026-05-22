//! 主机视觉配置覆盖。

use crate::model::{BackgroundProfile, HostId, ThemeProfile, VisualSettingsDraftField};

use super::super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};
use super::outcome::missing_host;

#[path = "host_apply.rs"]
mod apply;
#[path = "host_clear.rs"]
mod clear;

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
