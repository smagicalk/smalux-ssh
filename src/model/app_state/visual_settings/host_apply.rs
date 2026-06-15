//! 主机视觉配置应用。

use crate::core::CoreState;
use crate::model::{BackgroundProfile, HostId, ThemeProfile};

use super::super::super::AppUpdateOutcome;
use super::super::outcome::missing_host;

impl CoreState {
    /// 应用已经过草稿校验的主机视觉覆盖。
    pub(crate) fn apply_host_visual_profiles_action(
        &mut self,
        host_id: HostId,
        theme: ThemeProfile,
        background: BackgroundProfile,
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

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }
}
