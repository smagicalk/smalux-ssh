//! 快速新增主机草稿处理。
//!
//! 这里承接主机首页的创建/编辑/复制/分组弹窗。UI 只负责展示表单和发送字段变更，
//! 草稿的校验、默认值、保存到存储都集中在状态层，便于以后重写 UI 或换成别的前端。

use uuid::Uuid;

use crate::model::{
    ForwardId, GroupId, HostGroup, HostId, JumpChainId, ProxyId, QuickGroupDraft,
    QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField,
};

use super::super::{AppState, AppUpdateOutcome};
use super::draft_changed;

impl AppState {
    /// 更新快速新增主机表单草稿。
    pub(in crate::model::app_state) fn update_quick_host_draft(
        &mut self,
        field: QuickHostDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        // 字段级更新可以直接绑定输入框，避免 UI 层拼装完整 HostConfig。
        self.ui.set_quick_host_field(field, value);
        draft_changed()
    }

    /// 更新快速新增主机所属分组。
    pub(in crate::model::app_state) fn select_quick_host_group(
        &mut self,
        group_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        // group_id 为 None 表示保存到根分组“所有主机”。
        self.ui.select_quick_host_group(group_id);
        draft_changed()
    }

    /// 更新快速新增主机认证方式。
    pub(in crate::model::app_state) fn update_quick_host_auth_kind(
        &mut self,
        kind: QuickHostAuthKind,
    ) -> AppUpdateOutcome {
        // 切换认证方式时由 UiState 负责保留或清理对应字段。
        self.ui.set_quick_host_auth_kind(kind);
        draft_changed()
    }

    /// 更新快速新增主机认证字段。
    pub(in crate::model::app_state) fn update_quick_host_auth_field(
        &mut self,
        field: QuickHostAuthField,
        value: String,
    ) -> AppUpdateOutcome {
        // 认证字段可能是密码、私钥路径、agent 名称等，统一走草稿字段枚举。
        self.ui.set_quick_host_auth_field(field, value);
        draft_changed()
    }

    /// 切换当前主机草稿要使用的代理资源。
    pub(in crate::model::app_state) fn toggle_quick_host_network_proxy(
        &mut self,
        proxy_id: ProxyId,
    ) -> AppUpdateOutcome {
        if !self
            .storage
            .proxy_assets
            .iter()
            .any(|asset| asset.id == proxy_id)
        {
            return AppUpdateOutcome {
                error: Some("代理资源不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.toggle_quick_host_proxy(proxy_id);
        draft_changed()
    }

    /// 切换当前主机草稿要使用的跳板链资源。
    pub(in crate::model::app_state) fn toggle_quick_host_network_jump_chain(
        &mut self,
        chain_id: JumpChainId,
    ) -> AppUpdateOutcome {
        if !self
            .storage
            .jump_chain_assets
            .iter()
            .any(|asset| asset.id == chain_id)
        {
            return AppUpdateOutcome {
                error: Some("跳板资源不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.toggle_quick_host_jump_chain(chain_id);
        draft_changed()
    }

    /// 切换当前主机草稿要绑定的端口转发资源。
    pub(in crate::model::app_state) fn toggle_quick_host_network_forward(
        &mut self,
        forward_id: ForwardId,
    ) -> AppUpdateOutcome {
        if !self
            .storage
            .forward_assets
            .iter()
            .any(|asset| asset.id == forward_id)
        {
            return AppUpdateOutcome {
                error: Some("转发资源不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.toggle_quick_host_forward(forward_id);
        draft_changed()
    }

    /// 打开已保存主机的编辑弹窗。
    pub(in crate::model::app_state) fn open_edit_host_dialog(
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
            return AppUpdateOutcome {
                error: Some("主机不存在，无法编辑".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        // 编辑复用创建弹窗，但草稿中会记录 editing_host_id，保存时执行 upsert。
        self.ui.edit_quick_host(&host);
        self.ui.workspace.create_host_dialog_open = true;
        draft_changed()
    }

    /// 复制已保存主机，只复制主机配置本身，不复制历史、书签或片段。
    pub(in crate::model::app_state) fn duplicate_host(
        &mut self,
        host_id: HostId,
    ) -> AppUpdateOutcome {
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
        // 复制必须生成新 ID，避免覆盖原主机；历史、片段等关联数据不跟随复制。
        duplicate.id = HostId(Uuid::new_v4());
        duplicate.name = format!("{} 复制", duplicate.name);
        self.storage.upsert_host(duplicate);
        draft_changed()
    }

    /// 保存快速新增或编辑中的主机。
    pub(in crate::model::app_state) fn save_quick_host(&mut self) -> AppUpdateOutcome {
        let editing_host_id = self.ui.quick_host.editing_host_id;
        // 编辑保存时先取出现有主机，保留表单未覆盖的字段或后续扩展字段。
        let existing_host = editing_host_id.and_then(|host_id| {
            self.storage
                .hosts
                .iter()
                .find(|host| host.id == host_id)
                .cloned()
        });

        if editing_host_id.is_some() && existing_host.is_none() {
            return AppUpdateOutcome {
                error: Some("主机不存在，无法保存编辑".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let host_id = editing_host_id.unwrap_or_else(|| HostId(Uuid::new_v4()));
        // HostConfig 构造和校验在 UiState 草稿里完成，状态层只处理成功/失败分支。
        let host = match self
            .ui
            .quick_host
            .build_host_with_existing(host_id, existing_host.as_ref())
        {
            Ok(host) => host,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("主机表单无效：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        self.storage.upsert_host(host);
        // 保存成功后关闭弹窗并重置草稿，避免下次创建继承上次输入。
        self.ui.reset_quick_host();
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    /// 打开快速新增分组弹窗。
    pub(in crate::model::app_state) fn open_create_group_dialog(
        &mut self,
        parent_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        // 右键分组创建子分组时会传入 parent_id；根级创建则为 None。
        if parent_id.is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id)) {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法创建子分组".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        // 打开分组弹窗时关闭主机创建和父级选择弹窗，保证同一时间只有一个创建流程。
        self.ui.quick_group = QuickGroupDraft::with_parent(parent_id);
        self.ui.workspace.create_group_dialog_open = true;
        self.ui.workspace.create_group_parent_dialog_open = false;
        self.ui.workspace.pending_create_group_parent_id = None;
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    /// 打开新增主机弹窗，并预选保存分组。
    pub(in crate::model::app_state) fn open_create_host_dialog_in_group(
        &mut self,
        group_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        // 创建主机时预选分组，但仍允许用户在弹窗中切换到其他分组。
        if group_id.is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id)) {
            return AppUpdateOutcome {
                error: Some("分组不存在，无法创建主机".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        // 创建新主机必须从干净草稿开始，避免沿用编辑态的 host id。
        self.ui.reset_quick_host();
        self.ui.quick_host.group_id = group_id;
        self.ui.workspace.create_host_dialog_open = true;
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.workspace.create_group_parent_dialog_open = false;
        self.ui.workspace.pending_create_group_parent_id = None;
        draft_changed()
    }

    /// 打开创建分组前的父级选择弹窗。
    pub(in crate::model::app_state) fn open_create_group_parent_dialog(
        &mut self,
        parent_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        // “创建分组”入口先让用户选择父级，再进入真正填写名称的弹窗。
        if parent_id.is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id)) {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.workspace.pending_create_group_parent_id = parent_id;
        self.ui.workspace.create_group_parent_dialog_open = true;
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    /// 更新创建分组前选择的父级。
    pub(in crate::model::app_state) fn select_create_group_parent(
        &mut self,
        parent_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        // 父级可以被切回 None，表示新分组挂到根节点。
        if parent_id.is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id)) {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.workspace.pending_create_group_parent_id = parent_id;
        draft_changed()
    }

    /// 关闭创建分组前的父级选择弹窗。
    pub(in crate::model::app_state) fn close_create_group_parent_dialog(
        &mut self,
    ) -> AppUpdateOutcome {
        // 关闭父级选择时清理 pending，避免下一次打开继承旧选择。
        self.ui.workspace.create_group_parent_dialog_open = false;
        self.ui.workspace.pending_create_group_parent_id = None;
        draft_changed()
    }

    /// 确认父级选择后进入真正的创建分组弹窗。
    pub(in crate::model::app_state) fn confirm_create_group_parent(&mut self) -> AppUpdateOutcome {
        let parent_id = self.ui.workspace.pending_create_group_parent_id;
        // 复用 open_create_group_dialog 的校验和弹窗互斥逻辑。
        self.open_create_group_dialog(parent_id)
    }

    /// 更新快速新增分组名称。
    pub(in crate::model::app_state) fn update_quick_group_name(
        &mut self,
        name: String,
    ) -> AppUpdateOutcome {
        // 名称只写入草稿，保存时再 trim 和校验空值。
        self.ui.quick_group.name = name;
        draft_changed()
    }

    /// 更新快速新增分组的父级分组。
    pub(in crate::model::app_state) fn select_quick_group_parent(
        &mut self,
        parent_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        // 允许用户在创建分组弹窗内重新选择父级。
        if parent_id.is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id)) {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.quick_group.parent_id = parent_id;
        draft_changed()
    }

    /// 关闭快速新增分组弹窗。
    pub(in crate::model::app_state) fn close_create_group_dialog(&mut self) -> AppUpdateOutcome {
        // 取消创建时丢弃名称和父级选择，保持弹窗下一次打开是默认态。
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.quick_group = QuickGroupDraft::default();
        draft_changed()
    }

    /// 保存快速新增分组。
    pub(in crate::model::app_state) fn save_quick_group(&mut self) -> AppUpdateOutcome {
        let name = self.ui.quick_group.name.trim();
        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("分组名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let parent_id = self.ui.quick_group.parent_id;
        // 保存前再次确认父级仍存在，防止弹窗打开期间分组被其他操作删除。
        if parent_id.is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id)) {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法保存".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        // 分组目前只包含名称和父级。后续加图标、颜色时应扩展 HostGroup，而不是让 UI
        // 直接维护并行数组。
        self.storage.upsert_group(HostGroup {
            id: GroupId(Uuid::new_v4()),
            name: name.to_owned(),
            parent_id,
        });
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.quick_group = QuickGroupDraft::default();
        draft_changed()
    }
}
