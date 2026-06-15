//! 快速新增主机草稿处理。
//!
//! 这里承接主机首页的创建/编辑/复制/分组弹窗。UI 只负责展示表单和发送字段变更，
//! 草稿的校验、默认值、保存到存储都集中在状态层，便于以后重写 UI 或换成别的前端。

use uuid::Uuid;

use crate::core::CoreState;
use crate::model::{Host, HostGroup, HostId};

use super::super::AppUpdateOutcome;

impl CoreState {
    /// 保存或更新一个已经过桌面草稿校验的主机记录。
    pub(crate) fn save_host_record(
        &mut self,
        host: Host,
        editing_host_id: Option<HostId>,
    ) -> AppUpdateOutcome {
        if editing_host_id.is_some() && !self.storage.hosts.iter().any(|item| item.id == host.id) {
            return AppUpdateOutcome {
                error: Some("主机不存在，无法保存编辑".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.storage.upsert_host(host);
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 保存一个已经过桌面草稿校验的主机分组。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn save_group_record(&mut self, group: HostGroup) -> AppUpdateOutcome {
        self.storage.upsert_group(group);
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 复制一个已保存主机。
    pub(crate) fn duplicate_host_record(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(source) = self
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some("主机不存在，无法复制".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        let mut duplicate = source;
        duplicate.id = HostId(Uuid::new_v4());
        duplicate.name = format!("{} 复制", duplicate.name);
        self.storage.upsert_host(duplicate);
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }
}
