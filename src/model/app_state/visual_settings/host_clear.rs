//! 主机视觉配置清除。

use crate::model::HostId;

use super::super::super::{AppState, AppUpdateOutcome};
use super::super::outcome::missing_host;

impl AppState {
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
}
