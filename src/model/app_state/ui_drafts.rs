//! UI 输入草稿消息处理。

use crate::backend::BackendCommand;
use crate::model::HostId;
use crate::model::QuickHostAuthField;
use crate::model::QuickHostAuthKind;
use crate::model::QuickHostDraftField;
use crate::model::SessionId;
use crate::model::SessionKind;
use crate::model::SftpActionDraftField;
use crate::model::ToolPanelMode;
use crate::model::WorkspacePage;
use crate::terminal::TerminalTabState;
use uuid::Uuid;

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 更新快速新增主机表单草稿。
    pub(super) fn update_quick_host_draft(
        &mut self,
        field: QuickHostDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_field(field, value);
        draft_changed()
    }

    /// 更新快速新增主机认证方式。
    pub(super) fn update_quick_host_auth_kind(
        &mut self,
        kind: QuickHostAuthKind,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_auth_kind(kind);
        draft_changed()
    }

    /// 更新快速新增主机认证字段。
    pub(super) fn update_quick_host_auth_field(
        &mut self,
        field: QuickHostAuthField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_auth_field(field, value);
        draft_changed()
    }

    /// 保存快速新增主机。
    pub(super) fn save_quick_host(&mut self) -> AppUpdateOutcome {
        let host_id = HostId(Uuid::new_v4());
        let host = match self.ui.quick_host.build_host(host_id) {
            Ok(host) => host,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("主机表单无效：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        self.storage.upsert_host(host);
        self.ui.reset_quick_host();
        draft_changed()
    }

    /// 更新某台主机的远程命令输入草稿。
    pub(super) fn update_host_command_draft(
        &mut self,
        host_id: HostId,
        command: String,
    ) -> AppUpdateOutcome {
        self.ui.set_remote_command(host_id, command);
        draft_changed()
    }

    /// 更新某台主机的 SFTP 初始路径输入草稿。
    pub(super) fn update_host_sftp_initial_dir_draft(
        &mut self,
        host_id: HostId,
        initial_dir: String,
    ) -> AppUpdateOutcome {
        self.ui.set_sftp_initial_dir(host_id, initial_dir);
        draft_changed()
    }

    /// 更新 SFTP 操作草稿。
    pub(super) fn update_sftp_action_draft(
        &mut self,
        host_id: HostId,
        field: SftpActionDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_sftp_action_field(host_id, field, value);
        draft_changed()
    }

    /// 更新终端输入草稿。
    pub(super) fn update_terminal_input_draft(
        &mut self,
        session_id: SessionId,
        input: String,
    ) -> AppUpdateOutcome {
        self.ui.set_terminal_input(session_id, input);
        draft_changed()
    }

    /// 向终端输入草稿追加可见字符。
    pub(super) fn append_terminal_input_draft(
        &mut self,
        session_id: SessionId,
        text: String,
    ) -> AppUpdateOutcome {
        let filtered = printable_terminal_input(&text);
        if filtered.is_empty() {
            return AppUpdateOutcome::default();
        }

        self.ui.append_terminal_input(session_id, filtered);
        draft_changed()
    }

    /// 删除终端输入草稿的最后一个字符。
    pub(super) fn backspace_terminal_input_draft(
        &mut self,
        session_id: SessionId,
    ) -> AppUpdateOutcome {
        let before = self.ui.terminal_input_for(session_id).to_owned();
        self.ui.backspace_terminal_input(session_id);
        AppUpdateOutcome {
            state_changed: before != self.ui.terminal_input_for(session_id),
            ..AppUpdateOutcome::default()
        }
    }

    /// 把当前终端输入草稿发送到 Shell 后端。
    pub(super) fn send_terminal_input(&mut self, session_id: SessionId) -> AppUpdateOutcome {
        if session_id == crate::model::LOCAL_TERMINAL_SESSION_ID {
            ensure_local_terminal_tab(self, session_id);
        }

        let Some(tab) = self
            .sessions
            .tabs
            .iter()
            .find(|tab| tab.id == session_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到会话：{}", session_id.0)),
                ..AppUpdateOutcome::default()
            };
        };

        if !matches!(tab.kind, SessionKind::LocalShell | SessionKind::Shell) {
            return AppUpdateOutcome {
                error: Some("只有 Shell 标签页支持交互输入".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let host_id = if matches!(tab.kind, SessionKind::LocalShell) {
            None
        } else {
            tab.host_id
        };
        if !matches!(tab.kind, SessionKind::LocalShell) && host_id.is_none() {
            return AppUpdateOutcome {
                error: Some("Shell 会话缺少主机标识".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        let input = self.ui.terminal_input_for(session_id).to_owned();
        let trimmed = input.trim().to_owned();
        if trimmed.is_empty() && !matches!(tab.kind, SessionKind::LocalShell) {
            return AppUpdateOutcome {
                error: Some("终端输入不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if !trimmed.is_empty() {
            self.storage
                .add_command_history(crate::model::CommandHistoryItem {
                    id: crate::model::CommandHistoryId(Uuid::new_v4()),
                    host_id,
                    command: input.clone(),
                    working_directory: None,
                    exit_code: None,
                    started_at_unix_secs: unix_now_secs(),
                    duration_ms: None,
                });
        }

        self.backend_commands.push(BackendCommand::SendShellInput {
            session_id,
            input: format!("{input}\n"),
        });
        if matches!(tab.kind, SessionKind::LocalShell) && !trimmed.is_empty() {
            self.terminal.append_local_echo(
                session_id,
                crate::backend::LocalShellProfile::default_for_platform().prompt,
                &input,
            );
        }
        self.ui.clear_terminal_input(session_id);

        AppUpdateOutcome {
            state_changed: true,
            queued_backend_commands: 1,
            ..AppUpdateOutcome::default()
        }
    }

    /// 切换当前一级工作区页面。
    pub(super) fn set_workspace_page(&mut self, page: WorkspacePage) -> AppUpdateOutcome {
        self.ui.workspace.active_page = page;
        draft_changed()
    }

    /// 切换 Hosts 列表展示方式。
    pub(super) fn toggle_host_list_mode(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_host_list_mode();
        draft_changed()
    }

    /// 更新 Hosts 面板搜索条件。
    pub(super) fn update_host_search_query(&mut self, query: String) -> AppUpdateOutcome {
        self.ui.workspace.set_host_search_query(query);
        draft_changed()
    }

    /// 调整 Hosts 面板宽度。
    pub(super) fn resize_hosts_panel(&mut self, width: i32) -> AppUpdateOutcome {
        let before = self.ui.workspace.hosts_panel_width;
        self.ui.workspace.set_hosts_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.hosts_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 调整右侧活动栏宽度。
    pub(super) fn resize_activity_panel(&mut self, width: i32) -> AppUpdateOutcome {
        let before = self.ui.workspace.activity_panel_width;
        self.ui.workspace.set_activity_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.activity_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 调整 D 区域内部工具/SFTP 分栏宽度。
    pub(super) fn resize_tool_panel(&mut self, width: i32) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_width;
        self.ui.workspace.set_tool_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 打开 D 区域内部辅助分栏。
    pub(super) fn open_tool_panel(&mut self, mode: ToolPanelMode) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_mode;
        let before_page = self.ui.workspace.active_page;
        let before_active_tab = self.sessions.active_tab;
        self.ui.workspace.open_tool_panel(mode);
        if matches!(mode, ToolPanelMode::Sftp) {
            self.ui.workspace.active_page = WorkspacePage::Terminal;
            if let Some(active_terminal) = self.terminal.active_tab {
                self.sessions.active_tab = Some(active_terminal);
            }
        }
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_mode
                || before_page != self.ui.workspace.active_page
                || before_active_tab != self.sessions.active_tab,
            ..AppUpdateOutcome::default()
        }
    }

    /// 关闭 D 区域内部辅助分栏。
    pub(super) fn close_tool_panel(&mut self) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_mode;
        self.ui.workspace.close_tool_panel();
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_mode,
            ..AppUpdateOutcome::default()
        }
    }

    /// 折叠或展开右侧详情栏。
    pub(super) fn toggle_right_sidebar(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_right_sidebar();
        draft_changed()
    }

    /// 打开命令面板。
    pub(super) fn open_command_palette(&mut self, query: String) -> AppUpdateOutcome {
        self.ui.workspace.open_command_palette(query);
        draft_changed()
    }

    /// 更新命令面板查询。
    pub(super) fn update_command_palette_query(&mut self, query: String) -> AppUpdateOutcome {
        self.ui.workspace.command_palette.query = query;
        self.ui.workspace.command_palette.open = true;
        draft_changed()
    }

    /// 关闭命令面板。
    pub(super) fn close_command_palette(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.close_command_palette();
        draft_changed()
    }

    /// 切换到下一张背景轮播图。
    pub(super) fn next_background(&mut self) -> AppUpdateOutcome {
        let source_count = self.config.background.normalized().sources.len();
        self.ui.workspace.next_background(source_count);
        draft_changed()
    }
}

pub(super) fn draft_changed() -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        ..AppUpdateOutcome::default()
    }
}

fn printable_terminal_input(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control() && !('\u{e000}'..='\u{f8ff}').contains(ch))
        .collect()
}

fn ensure_local_terminal_tab(state: &mut AppState, session_id: SessionId) -> bool {
    let had_session = state.sessions.tabs.iter().any(|tab| tab.id == session_id);
    if !had_session {
        state
            .sessions
            .open_local_shell_tab(session_id, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
    }

    let had_terminal = state
        .terminal
        .tabs
        .iter()
        .any(|tab| tab.session_id == session_id);
    if !had_terminal {
        state.terminal.open_tab(TerminalTabState::new(
            session_id,
            crate::model::DEFAULT_LOCAL_TERMINAL_TITLE,
        ));
    }

    !had_session || !had_terminal
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AuthProfile;
    use crate::model::Message;
    use crate::model::QuickHostAuthField;
    use crate::model::QuickHostAuthKind;
    use crate::model::QuickHostDraftField;
    use crate::model::SecretRef;
    use crate::model::SessionId;
    use crate::model::SftpActionDraftField;
    use uuid::Uuid;

    #[test]
    fn quick_host_draft_message_updates_form_only() {
        let mut state = AppState::default();

        let outcome = state.apply(Message::UpdateQuickHostDraft {
            field: QuickHostDraftField::Address,
            value: "example.com".to_owned(),
        });

        assert!(outcome.changed());
        assert_eq!(state.ui.quick_host.address, "example.com");
        assert_eq!(state.storage.host_count(), 0);
    }

    #[test]
    fn quick_host_auth_messages_update_auth_draft_only() {
        let mut state = AppState::default();

        let kind_outcome = state.apply(Message::UpdateQuickHostAuthKind {
            kind: QuickHostAuthKind::Password,
        });
        let field_outcome = state.apply(Message::UpdateQuickHostAuthField {
            field: QuickHostAuthField::PasswordSecretRef,
            value: "password:root".to_owned(),
        });

        assert!(kind_outcome.changed());
        assert!(field_outcome.changed());
        assert!(matches!(
            state.ui.quick_host.auth.kind,
            QuickHostAuthKind::Password
        ));
        assert_eq!(
            state.ui.quick_host.auth.password_secret_ref,
            "password:root"
        );
        assert_eq!(state.storage.host_count(), 0);
    }

    #[test]
    fn save_quick_host_creates_agent_host_and_resets_form() {
        let mut state = AppState::default();
        state
            .ui
            .set_quick_host_field(QuickHostDraftField::Address, "prod.example.com".to_owned());
        state
            .ui
            .set_quick_host_field(QuickHostDraftField::Username, "deploy".to_owned());
        state
            .ui
            .set_quick_host_field(QuickHostDraftField::Tags, "prod,linux".to_owned());

        let outcome = state.apply(Message::SaveQuickHost);

        assert!(outcome.changed());
        assert_eq!(state.storage.host_count(), 1);
        assert_eq!(state.storage.hosts[0].name, "prod.example.com");
        assert_eq!(state.storage.hosts[0].tags, vec!["prod", "linux"]);
        assert_eq!(state.ui.quick_host.address, "");
        assert_eq!(state.ui.quick_host.port, "22");
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn save_quick_host_honors_selected_password_auth() {
        let mut state = AppState::default();
        state
            .ui
            .set_quick_host_field(QuickHostDraftField::Address, "root.example.com".to_owned());
        state
            .ui
            .set_quick_host_field(QuickHostDraftField::Username, "root".to_owned());
        state
            .ui
            .set_quick_host_auth_kind(QuickHostAuthKind::Password);
        state.ui.set_quick_host_auth_field(
            QuickHostAuthField::PasswordSecretRef,
            "password:root".to_owned(),
        );

        let outcome = state.apply(Message::SaveQuickHost);

        assert!(outcome.changed());
        assert_eq!(state.storage.host_count(), 1);
        assert!(matches!(
            &state.storage.hosts[0].auth,
            AuthProfile::Password {
                username,
                secret: SecretRef(secret_ref),
            } if username == "root" && secret_ref == "password:root"
        ));
    }

    #[test]
    fn save_quick_host_rejects_invalid_form_without_side_effects() {
        let mut state = AppState::default();

        let outcome = state.apply(Message::SaveQuickHost);

        assert!(outcome.changed());
        assert!(outcome.error.is_some());
        assert_eq!(state.storage.host_count(), 0);
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn command_draft_message_updates_ui_state_only() {
        let mut state = AppState::default();
        let host_id = HostId(Uuid::new_v4());

        let outcome = state.apply(Message::UpdateHostCommandDraft {
            host_id,
            command: "whoami".to_owned(),
        });

        assert!(outcome.changed());
        assert_eq!(state.ui.remote_command_for(host_id), "whoami");
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn sftp_initial_dir_draft_message_updates_ui_state_only() {
        let mut state = AppState::default();
        let host_id = HostId(Uuid::new_v4());

        let outcome = state.apply(Message::UpdateHostSftpInitialDirDraft {
            host_id,
            initial_dir: "/etc".to_owned(),
        });

        assert!(outcome.changed());
        assert_eq!(state.ui.sftp_initial_dir_for(host_id), "/etc");
        assert_eq!(state.sessions.tab_count(), 0);
    }

    #[test]
    fn sftp_action_draft_message_updates_ui_state_only() {
        let mut state = AppState::default();
        let host_id = HostId(Uuid::new_v4());

        let outcome = state.apply(Message::UpdateSftpActionDraft {
            host_id,
            field: SftpActionDraftField::LocalPath,
            value: "C:/tmp/app.tar.gz".to_owned(),
        });

        assert!(outcome.changed());
        assert_eq!(state.ui.sftp_local_path_for(host_id), "C:/tmp/app.tar.gz");
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn terminal_input_draft_message_updates_ui_state_only() {
        let mut state = AppState::default();
        let session_id = SessionId(Uuid::new_v4());

        let outcome = state.apply(Message::UpdateTerminalInputDraft {
            session_id,
            input: "ls".to_owned(),
        });

        assert!(outcome.changed());
        assert_eq!(state.ui.terminal_input_for(session_id), "ls");
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn terminal_key_messages_edit_input_draft_without_backend_side_effects() {
        let mut state = AppState::default();
        let session_id = SessionId(Uuid::new_v4());

        state.apply(Message::AppendTerminalInputDraft {
            session_id,
            text: "ls".to_owned(),
        });
        state.apply(Message::AppendTerminalInputDraft {
            session_id,
            text: "\u{e001}".to_owned(),
        });
        state.apply(Message::BackspaceTerminalInputDraft { session_id });

        assert_eq!(state.ui.terminal_input_for(session_id), "l");
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn local_terminal_input_is_visible_and_queues_on_enter() {
        let mut state = AppState::default();
        let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;

        let text = state.apply(Message::UpdateTerminalInputDraft {
            session_id,
            input: "echo smagicalssh-local".to_owned(),
        });
        assert!(text.changed());
        assert_eq!(
            state.ui.terminal_input_for(session_id),
            "echo smagicalssh-local"
        );

        let enter = state.apply(Message::SendTerminalInput { session_id });

        assert!(enter.changed());
        assert_eq!(state.backend_commands.pending_count(), 1);
        assert_eq!(state.ui.terminal_input_for(session_id), "");
        assert_eq!(
            state.terminal.tabs[0].buffer,
            vec![format!(
                "{} echo smagicalssh-local",
                crate::backend::LocalShellProfile::default_for_platform().prompt
            )]
        );
        assert!(matches!(
            state.backend_commands.front(),
            Some(BackendCommand::SendShellInput { session_id: queued_session_id, input })
                if *queued_session_id == session_id && input == "echo smagicalssh-local\n"
        ));
        assert_eq!(state.storage.command_history_count(), 1);
        assert_eq!(state.storage.command_history[0].host_id, None);
    }

    #[test]
    fn local_terminal_starts_without_help_banner() {
        let mut state = AppState::default();
        let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;

        assert!(ensure_local_terminal_tab(&mut state, session_id));
        assert!(!ensure_local_terminal_tab(&mut state, session_id));

        let tab = state
            .terminal
            .tabs
            .iter()
            .find(|tab| tab.session_id == session_id)
            .expect("local terminal tab should exist");
        assert_eq!(tab.title, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
        assert!(tab.buffer.is_empty());
    }

    #[test]
    fn local_terminal_empty_enter_queues_newline_without_history() {
        let mut state = AppState::default();
        let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;

        ensure_local_terminal_tab(&mut state, session_id);
        state.apply(Message::UpdateTerminalInputDraft {
            session_id,
            input: String::new(),
        });

        let outcome = state.apply(Message::SendTerminalInput { session_id });

        assert!(outcome.changed());
        assert_eq!(state.ui.terminal_input_for(session_id), "");
        assert!(matches!(
            state.backend_commands.front(),
            Some(BackendCommand::SendShellInput { session_id: queued_session_id, input })
                if *queued_session_id == session_id && input == "\n"
        ));
        assert_eq!(state.storage.command_history_count(), 0);
    }

    #[test]
    fn workspace_ui_messages_update_layout_state_only() {
        let mut state = AppState::default();

        state.apply(Message::SetWorkspacePage {
            page: WorkspacePage::Settings,
        });
        state.apply(Message::ToggleHostListMode);
        state.apply(Message::UpdateHostSearchQuery {
            query: "prod".to_owned(),
        });
        state.apply(Message::ResizeHostsPanel { width: 260 });
        state.apply(Message::ResizeActivityPanel { width: 300 });
        state.apply(Message::ResizeToolPanel { width: 360 });
        state.apply(Message::OpenToolPanel {
            mode: ToolPanelMode::History,
        });
        state.apply(Message::ToggleRightSidebar);
        state.apply(Message::OpenCommandPalette {
            query: "prod".to_owned(),
        });

        assert_eq!(state.ui.workspace.active_page, WorkspacePage::Settings);
        assert!(matches!(
            state.ui.workspace.host_list_mode,
            crate::model::HostListMode::Card
        ));
        assert_eq!(state.ui.workspace.host_search_query, "prod");
        assert_eq!(state.ui.workspace.hosts_panel_width, 260);
        assert_eq!(state.ui.workspace.activity_panel_width, 300);
        assert_eq!(state.ui.workspace.tool_panel_width, 360);
        assert_eq!(state.ui.workspace.tool_panel_mode, ToolPanelMode::History);
        assert!(state.ui.workspace.right_sidebar_collapsed);
        assert!(state.ui.workspace.command_palette.open);
        assert_eq!(state.ui.workspace.command_palette.query, "prod");
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn tool_panel_messages_update_layout_state_only() {
        let mut state = AppState::default();

        let open = state.apply(Message::OpenToolPanel {
            mode: ToolPanelMode::History,
        });
        let close = state.apply(Message::CloseToolPanel);

        assert!(open.changed());
        assert!(close.changed());
        assert_eq!(state.ui.workspace.tool_panel_mode, ToolPanelMode::Closed);
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn opening_sftp_tool_panel_returns_to_terminal_workspace() {
        let mut state = AppState::default();
        let host_id = HostId(Uuid::new_v4());
        let session_id = SessionId(Uuid::new_v4());
        state
            .sessions
            .open_shell_tab(session_id, host_id, "production");
        state
            .terminal
            .open_tab(TerminalTabState::new(session_id, "production"));
        state.ui.workspace.active_page = WorkspacePage::Sftp;

        let open = state.apply(Message::OpenToolPanel {
            mode: ToolPanelMode::Sftp,
        });

        assert!(open.changed());
        assert_eq!(state.ui.workspace.active_page, WorkspacePage::Terminal);
        assert_eq!(state.ui.workspace.tool_panel_mode, ToolPanelMode::Sftp);
        assert_eq!(state.sessions.active_tab, Some(session_id));
    }
}
