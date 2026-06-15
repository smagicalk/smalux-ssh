//! 命令片段和历史命令消息路由。
//!
//! 这里处理命令模板的保存、变量填充、执行和历史命令重放。片段最终会变成
//! 远程命令启动请求，但模板渲染和变量校验留在 snippet 领域内。

use crate::core::CoreState;

use super::super::{AppState, AppUpdateOutcome, Message};

impl CoreState {
    /// 分发命令片段和历史命令消息。
    pub(super) fn dispatch_snippet_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::UpdateSnippetArgument {
                snippet_id,
                name,
                value,
            } => self.update_snippet_argument(snippet_id, name, value),
            Message::CreateSnippet {
                name,
                description,
                command_template,
                scope,
                group_id,
            } => self.create_snippet(name, description, command_template, scope, group_id),
            Message::UpdateSnippet {
                snippet_id,
                name,
                description,
                command_template,
                scope,
                group_id,
            } => self.update_snippet(
                snippet_id,
                name,
                description,
                command_template,
                scope,
                group_id,
            ),
            Message::CreateSnippetTarget {
                snippet_id,
                target_keys,
                display_name,
                command_template,
                share_target_id,
            } => self.create_snippet_targets(
                snippet_id,
                target_keys,
                display_name,
                command_template,
                share_target_id,
            ),
            Message::UpdateSnippetTarget {
                snippet_id,
                target_id,
                target_key,
                display_name,
                command_template,
            } => self.update_snippet_target(
                snippet_id,
                target_id,
                target_key,
                display_name,
                command_template,
            ),
            Message::SyncSnippetTargetImplementationTargets {
                snippet_id,
                target_id,
                target_keys,
                display_name,
                command_template,
            } => self.sync_snippet_target_implementation_targets(
                snippet_id,
                target_id,
                target_keys,
                display_name,
                command_template,
            ),
            Message::RemoveSnippetTarget {
                snippet_id,
                target_id,
            } => self.remove_snippet_target(snippet_id, target_id),
            Message::SplitSnippetTargetImplementation {
                snippet_id,
                target_id,
            } => self.split_snippet_target_implementation(snippet_id, target_id),
            Message::CreateSnippetGroup { name, parent_id } => {
                self.create_snippet_group(name, parent_id)
            }
            Message::RenameSnippetGroup { group_id, name } => {
                self.rename_snippet_group(group_id, name)
            }
            Message::RemoveSnippetGroup { group_id } => self.remove_snippet_group(group_id),
            Message::RemoveSnippetGroupRecursive { group_id } => {
                self.remove_snippet_group_recursive(group_id)
            }
            Message::MoveSnippetGroup {
                group_id,
                parent_id,
            } => self.move_snippet_group(group_id, parent_id),
            Message::MoveSnippet {
                snippet_id,
                group_id,
            } => self.move_snippet(snippet_id, group_id),
            Message::RemoveSnippet { snippet_id } => self.remove_snippet(snippet_id),
            Message::SaveHostCommandSnippet { .. }
            | Message::RunSnippetOnActiveHost { .. }
            | Message::RunSnippetTargetOnActiveHost { .. } => AppUpdateOutcome {
                error: Some("当前片段消息仍依赖运行期上下文，不能只在 CoreState 中运行".to_owned()),
                ..AppUpdateOutcome::default()
            },
            Message::RunSnippet {
                host_id,
                snippet_id,
            } => self.run_snippet_action(host_id, snippet_id),
            Message::RunSnippetWithArguments {
                host_id,
                snippet_id,
                arguments,
            } => self.run_snippet_with_arguments_action(host_id, snippet_id, arguments),
            Message::RunSnippetTargetWithArguments {
                host_id,
                snippet_id,
                target_id,
                arguments,
            } => self.run_snippet_target_with_arguments_action(
                host_id, snippet_id, target_id, arguments,
            ),
            Message::RunCommandHistory { history_id } => self.run_command_history(history_id),
            _ => unreachable!("非命令片段消息不应进入命令片段路由"),
        }
    }
}

impl AppState {
    pub(super) fn dispatch_snippet_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::UpdateSnippetArgument { .. }
            | Message::CreateSnippet { .. }
            | Message::UpdateSnippet { .. }
            | Message::CreateSnippetTarget { .. }
            | Message::UpdateSnippetTarget { .. }
            | Message::SyncSnippetTargetImplementationTargets { .. }
            | Message::RemoveSnippetTarget { .. }
            | Message::SplitSnippetTargetImplementation { .. }
            | Message::CreateSnippetGroup { .. }
            | Message::RenameSnippetGroup { .. }
            | Message::RemoveSnippetGroup { .. }
            | Message::RemoveSnippetGroupRecursive { .. }
            | Message::MoveSnippetGroup { .. }
            | Message::MoveSnippet { .. }
            | Message::RemoveSnippet { .. } => self.core.dispatch_snippet_message(message),
            _ => match message {
                Message::SaveHostCommandSnippet { host_id } => {
                    self.core.save_host_command_snippet_action(
                        host_id,
                        self.ui.remote_command_for(host_id).to_owned(),
                    )
                }
                Message::RunSnippet {
                    host_id,
                    snippet_id,
                } => self.run_snippet(host_id, snippet_id),
                Message::RunSnippetWithArguments {
                    host_id,
                    snippet_id,
                    arguments,
                } => {
                    let outcome = self
                        .core
                        .run_snippet_with_arguments_action(host_id, snippet_id, arguments);
                    if outcome.changed() {
                        self.ui.workspace.active_page = crate::model::WorkspacePage::Terminal;
                    }
                    outcome
                }
                Message::RunSnippetTargetWithArguments {
                    host_id,
                    snippet_id,
                    target_id,
                    arguments,
                } => {
                    let outcome = self.core.run_snippet_target_with_arguments_action(
                        host_id, snippet_id, target_id, arguments,
                    );
                    if outcome.changed() {
                        self.ui.workspace.active_page = crate::model::WorkspacePage::Terminal;
                    }
                    outcome
                }
                Message::RunSnippetOnActiveHost { snippet_id } => {
                    self.run_snippet_on_active_host(snippet_id)
                }
                Message::RunSnippetTargetOnActiveHost {
                    snippet_id,
                    target_id,
                } => self.run_snippet_target_on_active_host(snippet_id, target_id),
                Message::RunCommandHistory { history_id } => {
                    self.core.run_command_history(history_id)
                }
                _ => unreachable!("非命令片段消息不应进入命令片段路由"),
            },
        }
    }
}
