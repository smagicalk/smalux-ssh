//! 主机视觉配置清除。

use crate::core::CoreState;
use crate::model::HostId;

use super::super::super::AppUpdateOutcome;
use super::super::outcome::missing_host;

impl CoreState {
    /// 清除主机视觉覆盖的稳定核心入口。
    pub(crate) fn clear_host_visual_profiles_action(
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

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }
}
