//! 主机视觉配置应用。

use crate::model::{HostId, VisualSettingsDraft};

use super::super::super::{AppState, AppUpdateOutcome};
use super::super::outcome::{invalid_visual_settings, missing_host};

impl AppState {
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
        let Some((fallback_theme, fallback_background)) = self.host_visual_fallbacks(host_id)
        else {
            return missing_host(host_id);
        };
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

        let changed = host.theme_override.as_ref() != Some(&theme)
            || host.background_override.as_ref() != Some(&background);
        let Some(host) = self
            .storage
            .hosts
            .iter_mut()
            .find(|host| host.id == host_id)
        else {
            return missing_host(host_id);
        };
        host.theme_override = Some(theme);
        host.background_override = Some(background);
        self.ui.clear_host_visual_settings_draft(host_id);

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }
}
