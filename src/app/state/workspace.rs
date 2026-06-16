//! 桌面工作区控制逻辑。

use crate::config::HostListModePreference;
use crate::model::{AppUpdateOutcome, HostId, LanguageMode, ToolPanelMode, WorkspacePage};

use super::{DesktopAppState, draft_changed};

impl DesktopAppState {
    pub(super) fn activate_terminal_page_for(&mut self, outcome: &AppUpdateOutcome) {
        if outcome.changed() {
            self.ui.workspace.active_page = WorkspacePage::Terminal;
        }
    }

    pub(super) fn activate_sftp_page_for(&mut self, outcome: &AppUpdateOutcome) {
        if outcome.changed() {
            self.ui.workspace.active_page = WorkspacePage::Sftp;
        }
    }

    pub(super) fn activate_tunnel_page_for(&mut self, outcome: &AppUpdateOutcome) {
        if outcome.changed() {
            self.ui.workspace.active_page = WorkspacePage::Tunnels;
        }
    }

    pub(super) fn set_workspace_page(&mut self, page: WorkspacePage) -> AppUpdateOutcome {
        self.ui.workspace.active_page = page;
        self.ui.workspace.set_hosts_panel_collapsed(false);
        draft_changed()
    }

    pub(super) fn navigate_workspace_page(&mut self, page: WorkspacePage) -> AppUpdateOutcome {
        if self.ui.workspace.active_page == page {
            let collapsed = !self.ui.workspace.hosts_panel_collapsed;
            self.ui.workspace.set_hosts_panel_collapsed(collapsed);
        } else {
            self.ui.workspace.active_page = page;
            self.ui.workspace.set_hosts_panel_collapsed(false);
        }
        draft_changed()
    }

    pub(super) fn toggle_host_list_mode(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_host_list_mode();
        let preference = match self.ui.workspace.host_list_mode {
            crate::model::HostListMode::Tree => HostListModePreference::Tree,
            crate::model::HostListMode::Card => HostListModePreference::Card,
        };
        let changed = self.core.config.workspace.host_list_mode != preference;
        self.core.config.workspace.host_list_mode = preference;
        self.core.storage.app_config = self.core.config.clone();

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    pub(super) fn open_tool_panel(&mut self, mode: ToolPanelMode) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_mode;
        let before_page = self.ui.workspace.active_page;
        let before_active_tab = self.core.sessions.active_tab;
        self.ui.workspace.open_tool_panel(mode);
        if matches!(mode, ToolPanelMode::Sftp) {
            self.ui.workspace.active_page = WorkspacePage::Terminal;
            if let Some(active_terminal) = self.core.terminal.active_tab {
                self.core.sessions.active_tab = Some(active_terminal);
            }
        }
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_mode
                || before_page != self.ui.workspace.active_page
                || before_active_tab != self.core.sessions.active_tab,
            ..AppUpdateOutcome::default()
        }
    }

    pub(super) fn active_remote_host_id(&self) -> Option<HostId> {
        let active_tab = self.core.sessions.active_tab?;
        self.core
            .sessions
            .tabs
            .iter()
            .find(|tab| tab.id == active_tab)
            .and_then(|tab| tab.host_id)
    }

    pub(super) fn next_theme_local(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.next_theme();
        self.sync_built_in_theme_preference()
    }

    pub(super) fn set_language_local(&mut self, language: LanguageMode) -> AppUpdateOutcome {
        let before = self.ui.workspace.language;
        self.ui.workspace.set_language(language);
        let changed = before != self.ui.workspace.language;
        self.core.config.workspace.language = language.preference();
        self.core.storage.app_config = self.core.config.clone();

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    pub(super) fn set_built_in_theme_local(
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

    pub(super) fn sync_built_in_theme_preference(&mut self) -> AppUpdateOutcome {
        let preference = self.ui.workspace.theme.preference();
        let changed = self.core.config.workspace.built_in_theme != preference;
        self.core.config.workspace.built_in_theme = preference;
        self.core.storage.app_config = self.core.config.clone();

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    pub(super) fn next_background_local(&mut self) -> AppUpdateOutcome {
        let source_count = self.core.config.background.normalized().sources.len();
        self.ui.workspace.next_background(source_count);
        draft_changed()
    }
}
