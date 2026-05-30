//! 工作区分栏尺寸与侧栏状态。

use super::super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};

impl AppState {
    /// 调整 Hosts 面板宽度。
    pub(in crate::model::app_state) fn resize_hosts_panel(
        &mut self,
        width: i32,
    ) -> AppUpdateOutcome {
        let before = self.ui.workspace.hosts_panel_width;
        self.ui.workspace.set_hosts_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.hosts_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 调整右侧活动栏宽度。
    pub(in crate::model::app_state) fn resize_activity_panel(
        &mut self,
        width: i32,
    ) -> AppUpdateOutcome {
        let before = self.ui.workspace.activity_panel_width;
        self.ui.workspace.set_activity_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.activity_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 调整 D 区域内部工具/SFTP 分栏宽度。
    pub(in crate::model::app_state) fn resize_tool_panel(
        &mut self,
        width: i32,
    ) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_width;
        self.ui.workspace.set_tool_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 折叠或展开右侧详情栏。
    pub(in crate::model::app_state) fn toggle_right_sidebar(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_right_sidebar();
        draft_changed()
    }

    /// 切换到下一个内置主题。
    pub(in crate::model::app_state) fn next_theme(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.next_theme();
        self.sync_built_in_theme_preference()
    }

    /// 折叠或展开主机树分组。
    pub(in crate::model::app_state) fn toggle_host_tree_group(
        &mut self,
        group_id: Option<crate::model::GroupId>,
    ) -> AppUpdateOutcome {
        self.ui.workspace.toggle_host_tree_group(group_id);
        draft_changed()
    }

    /// 设置 UI 语言偏好。
    pub(in crate::model::app_state) fn set_language(
        &mut self,
        language: crate::model::LanguageMode,
    ) -> AppUpdateOutcome {
        let before = self.ui.workspace.language;
        self.ui.workspace.set_language(language);
        let changed = before != self.ui.workspace.language;
        self.config.workspace.language = language.preference();
        self.storage.app_config = self.config.clone();

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    /// 设置内置主题偏好。
    pub(in crate::model::app_state) fn set_built_in_theme(
        &mut self,
        theme: crate::model::BuiltInTheme,
    ) -> AppUpdateOutcome {
        let before = self.ui.workspace.theme;
        self.ui.workspace.set_built_in_theme(theme);
        let outcome = self.sync_built_in_theme_preference();
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.theme || outcome.state_changed,
            ..outcome
        }
    }

    fn sync_built_in_theme_preference(&mut self) -> AppUpdateOutcome {
        let preference = self.ui.workspace.theme.preference();
        let changed = self.config.workspace.built_in_theme != preference;
        self.config.workspace.built_in_theme = preference;
        self.storage.app_config = self.config.clone();

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    /// 打开新增主机弹窗。
    pub(in crate::model::app_state) fn open_create_host_dialog(&mut self) -> AppUpdateOutcome {
        self.ui.reset_quick_host();
        self.ui.workspace.create_host_dialog_open = true;
        self.ui.workspace.create_group_dialog_open = false;
        draft_changed()
    }

    /// 关闭新增主机弹窗。
    pub(in crate::model::app_state) fn close_create_host_dialog(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }
}
