//! 工作区页面与 Hosts 列表状态。

use crate::config::HostListModePreference;
use crate::model::{HostListMode, WorkspacePage};

use super::super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};

impl AppState {
    /// 切换当前一级工作区页面。
    pub(in crate::model::app_state) fn set_workspace_page(
        &mut self,
        page: WorkspacePage,
    ) -> AppUpdateOutcome {
        self.ui.workspace.active_page = page;
        self.ui.workspace.set_hosts_panel_collapsed(false);
        draft_changed()
    }

    /// 通过左侧导航进入页面；重复点击当前页面会折叠或展开 Hosts 面板。
    pub(in crate::model::app_state) fn navigate_workspace_page(
        &mut self,
        page: WorkspacePage,
    ) -> AppUpdateOutcome {
        if self.ui.workspace.active_page == page {
            let collapsed = !self.ui.workspace.hosts_panel_collapsed;
            self.ui.workspace.set_hosts_panel_collapsed(collapsed);
        } else {
            self.ui.workspace.active_page = page;
            self.ui.workspace.set_hosts_panel_collapsed(false);
        }

        draft_changed()
    }

    /// 切换 Hosts 列表展示方式。
    pub(in crate::model::app_state) fn toggle_host_list_mode(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_host_list_mode();
        let preference = host_list_mode_preference(self.ui.workspace.host_list_mode);
        let changed = self.config.workspace.host_list_mode != preference;
        self.config.workspace.host_list_mode = preference;
        self.storage.app_config = self.config.clone();

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    /// 更新 Hosts 面板搜索条件。
    pub(in crate::model::app_state) fn update_host_search_query(
        &mut self,
        query: String,
    ) -> AppUpdateOutcome {
        self.ui.workspace.set_host_search_query(query);
        draft_changed()
    }

    /// 更新密钥页分组树搜索条件。
    pub(in crate::model::app_state) fn update_credential_search_query(
        &mut self,
        query: String,
    ) -> AppUpdateOutcome {
        self.ui.workspace.set_credential_search_query(query);
        draft_changed()
    }

    /// 更新片段页分组树搜索条件。
    pub(in crate::model::app_state) fn update_snippet_search_query(
        &mut self,
        query: String,
    ) -> AppUpdateOutcome {
        self.ui.workspace.set_snippet_search_query(query);
        draft_changed()
    }

    /// 折叠或展开密钥页分组树节点。
    pub(in crate::model::app_state) fn toggle_credential_tree_node(
        &mut self,
        node_id: String,
    ) -> AppUpdateOutcome {
        self.ui.workspace.toggle_credential_tree_node(node_id);
        draft_changed()
    }

    /// 折叠或展开片段页分组树节点。
    pub(in crate::model::app_state) fn toggle_snippet_tree_node(
        &mut self,
        node_id: String,
    ) -> AppUpdateOutcome {
        self.ui.workspace.toggle_snippet_tree_node(node_id);
        draft_changed()
    }

    /// 更新新建会话弹窗搜索条件。
    pub(in crate::model::app_state) fn update_new_session_search_query(
        &mut self,
        query: String,
    ) -> AppUpdateOutcome {
        self.ui.workspace.set_new_session_search_query(query);
        draft_changed()
    }
}

fn host_list_mode_preference(mode: HostListMode) -> HostListModePreference {
    match mode {
        HostListMode::Tree => HostListModePreference::Tree,
        HostListMode::Card => HostListModePreference::Card,
    }
}
