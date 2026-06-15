//! 快速新增主机表单草稿构建。

use crate::model::host_draft::{
    DEFAULT_QUICK_HOST_ICON_KEY, build_host_from_draft, quick_host_draft_from_host,
};
use crate::model::{GroupId, Host, HostId, HostNetworkSelection};

use super::{QuickHostAuthDraft, QuickHostDraftError};

/// 快速新增主机表单草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickHostDraft {
    pub editing_host_id: Option<HostId>,
    pub group_id: Option<GroupId>,
    pub network: HostNetworkSelection,
    pub name: String,
    pub address: String,
    pub port: String,
    pub username: String,
    pub icon_key: String,
    pub tags: String,
    pub auth: QuickHostAuthDraft,
}

impl Default for QuickHostDraft {
    fn default() -> Self {
        Self {
            editing_host_id: None,
            group_id: None,
            network: HostNetworkSelection::default(),
            name: String::new(),
            address: String::new(),
            port: "22".to_owned(),
            username: String::new(),
            icon_key: DEFAULT_QUICK_HOST_ICON_KEY.to_owned(),
            tags: String::new(),
            auth: QuickHostAuthDraft::default(),
        }
    }
}

impl QuickHostDraft {
    /// 从已保存主机生成编辑草稿。
    pub fn from_host(host: &Host) -> Self {
        quick_host_draft_from_host(host)
    }

    /// 将表单草稿转换为可保存主机。
    pub fn build_host(&self, id: HostId) -> Result<Host, QuickHostDraftError> {
        self.build_host_with_existing(id, None)
    }

    /// 将表单草稿转换为可保存主机，并保留当前表单暂未暴露的高级配置。
    pub fn build_host_with_existing(
        &self,
        id: HostId,
        existing: Option<&Host>,
    ) -> Result<Host, QuickHostDraftError> {
        build_host_from_draft(self, id, existing)
    }

    /// 兼容旧调用的快捷入口，内部仍走统一主机构建逻辑。
    pub fn build_agent_host(&self, id: HostId) -> Result<Host, QuickHostDraftError> {
        self.build_host(id)
    }
}
