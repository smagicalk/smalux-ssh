//! 已保存主机与分组记录的核心动作。
//!
//! 这里只处理主机/分组记录本身，不再和凭据、Known Hosts、网络资源混在一个大模块里。

use crate::core::CoreState;
use crate::model::{GroupId, HostId};

use super::AppUpdateOutcome;

impl CoreState {
    /// 删除已保存主机的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn remove_host_record_action(&mut self, host_id: HostId) -> AppUpdateOutcome {
        self.remove_host_by_id(host_id)
    }

    /// 递归删除已保存主机分组的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn remove_group_record_recursive_action(
        &mut self,
        group_id: GroupId,
    ) -> AppUpdateOutcome {
        if self.storage.remove_group_recursive(group_id) {
            return AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            };
        }

        AppUpdateOutcome {
            error: Some("分组不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }

    /// 删除已保存主机，并清理主机相关的本地索引。
    fn remove_host_by_id(&mut self, host_id: HostId) -> AppUpdateOutcome {
        if self.storage.remove_host(host_id) {
            return AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            };
        }

        AppUpdateOutcome {
            error: Some("主机不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }
}
