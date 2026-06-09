//! 执行快捷命令。

use crate::model::{
    HostId, Snippet, SnippetArgument, SnippetId, SnippetImplementation, SnippetRenderError,
    SnippetSupportTargetId,
};

use super::super::{AppState, AppUpdateOutcome};
use super::outcome::{missing_host, missing_snippet};

impl AppState {
    /// 在当前活动远程标签关联的主机上执行快捷命令。
    pub(in crate::model::app_state) fn run_snippet_on_active_host(
        &mut self,
        snippet_id: SnippetId,
    ) -> AppUpdateOutcome {
        let Some(host_id) = self.active_remote_host_id() else {
            return AppUpdateOutcome {
                error: Some("请先打开或选中一个远程主机终端".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.run_snippet(host_id, snippet_id)
    }

    /// 在当前活动远程标签关联的主机上执行指定支持目标。
    pub(in crate::model::app_state) fn run_snippet_target_on_active_host(
        &mut self,
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
    ) -> AppUpdateOutcome {
        let Some(host_id) = self.active_remote_host_id() else {
            return AppUpdateOutcome {
                error: Some("请先打开或选中一个远程主机终端".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.run_snippet_target(host_id, snippet_id, target_id)
    }

    /// 使用本次输入参数渲染并运行快捷命令。
    pub(in crate::model::app_state) fn run_snippet_with_arguments(
        &mut self,
        host_id: HostId,
        snippet_id: SnippetId,
        arguments: Vec<SnippetArgument>,
    ) -> AppUpdateOutcome {
        let Some(host) = self
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
        else {
            return missing_host(host_id);
        };

        let Some(snippet) = self
            .storage
            .snippets
            .iter()
            .find(|snippet| snippet.id == snippet_id)
            .cloned()
        else {
            return missing_snippet(snippet_id);
        };
        if !snippet.scope.applies_to_host(&host) {
            return AppUpdateOutcome {
                error: Some(format!("快捷命令不适用于主机：{}", host.name)),
                ..AppUpdateOutcome::default()
            };
        }

        let valid_arguments = arguments
            .into_iter()
            .filter(|argument| {
                snippet
                    .variables
                    .iter()
                    .any(|variable| variable.name == argument.name)
            })
            .collect::<Vec<_>>();
        let Some(implementation) = snippet.default_implementation() else {
            return AppUpdateOutcome {
                error: Some("快捷命令没有可执行脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command = match implementation.render(&snippet.variables, &valid_arguments) {
            Ok(command) => command,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(snippet_render_error_message(error)),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        if command.trim().is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令渲染结果不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.storage
            .record_snippet_arguments(snippet_id, valid_arguments);
        self.run_remote_command(host_id, command, false)
    }

    /// 使用本次输入参数渲染并运行指定支持目标。
    pub(in crate::model::app_state) fn run_snippet_target_with_arguments(
        &mut self,
        host_id: HostId,
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
        arguments: Vec<SnippetArgument>,
    ) -> AppUpdateOutcome {
        let Some(host) = self
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
        else {
            return missing_host(host_id);
        };

        let Some(snippet) = self
            .storage
            .snippets
            .iter()
            .find(|snippet| snippet.id == snippet_id)
            .cloned()
        else {
            return missing_snippet(snippet_id);
        };
        if !snippet.scope.applies_to_host(&host) {
            return AppUpdateOutcome {
                error: Some(format!("快捷命令不适用于主机：{}", host.name)),
                ..AppUpdateOutcome::default()
            };
        }

        let valid_arguments = arguments
            .into_iter()
            .filter(|argument| {
                snippet
                    .variables
                    .iter()
                    .any(|variable| variable.name == argument.name)
            })
            .collect::<Vec<_>>();
        let Some(implementation) = snippet_implementation_for_target(&snippet, target_id) else {
            return AppUpdateOutcome {
                error: Some("快捷命令支持目标没有可执行脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command = match implementation.render(&snippet.variables, &valid_arguments) {
            Ok(command) => command,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(snippet_render_error_message(error)),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        if command.trim().is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令渲染结果不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.storage
            .record_snippet_implementation_arguments(implementation.id, valid_arguments);
        self.run_remote_command(host_id, command, false)
    }

    /// 渲染并执行适用于主机的快捷命令。
    pub(in crate::model::app_state) fn run_snippet(
        &mut self,
        host_id: HostId,
        snippet_id: SnippetId,
    ) -> AppUpdateOutcome {
        let Some(host) = self
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
        else {
            return missing_host(host_id);
        };

        let Some(snippet) = self
            .storage
            .snippets
            .iter()
            .find(|snippet| snippet.id == snippet_id)
            .cloned()
        else {
            return missing_snippet(snippet_id);
        };

        if !snippet.scope.applies_to_host(&host) {
            return AppUpdateOutcome {
                error: Some(format!("快捷命令不适用于主机：{}", host.name)),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(implementation) = snippet.default_implementation() else {
            return AppUpdateOutcome {
                error: Some("快捷命令没有可执行脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command =
            match implementation.render(&snippet.variables, &implementation.last_arguments) {
                Ok(command) => command,
                Err(error) => {
                    return AppUpdateOutcome {
                        error: Some(snippet_render_error_message(error)),
                        ..AppUpdateOutcome::default()
                    };
                }
            };
        if command.trim().is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令渲染结果不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.run_remote_command(host_id, command, false)
    }

    /// 渲染并执行指定支持目标。
    pub(in crate::model::app_state) fn run_snippet_target(
        &mut self,
        host_id: HostId,
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
    ) -> AppUpdateOutcome {
        let Some(host) = self
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
        else {
            return missing_host(host_id);
        };

        let Some(snippet) = self
            .storage
            .snippets
            .iter()
            .find(|snippet| snippet.id == snippet_id)
            .cloned()
        else {
            return missing_snippet(snippet_id);
        };

        if !snippet.scope.applies_to_host(&host) {
            return AppUpdateOutcome {
                error: Some(format!("快捷命令不适用于主机：{}", host.name)),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(implementation) = snippet_implementation_for_target(&snippet, target_id) else {
            return AppUpdateOutcome {
                error: Some("快捷命令支持目标没有可执行脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command =
            match implementation.render(&snippet.variables, &implementation.last_arguments) {
                Ok(command) => command,
                Err(error) => {
                    return AppUpdateOutcome {
                        error: Some(snippet_render_error_message(error)),
                        ..AppUpdateOutcome::default()
                    };
                }
            };
        if command.trim().is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令渲染结果不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.run_remote_command(host_id, command, false)
    }

    fn active_remote_host_id(&self) -> Option<HostId> {
        let active_tab = self.sessions.active_tab?;
        self.sessions
            .tabs
            .iter()
            .find(|tab| tab.id == active_tab)
            .and_then(|tab| tab.host_id)
    }
}

fn snippet_implementation_for_target(
    snippet: &Snippet,
    target_id: SnippetSupportTargetId,
) -> Option<&SnippetImplementation> {
    let target = snippet
        .support_targets
        .iter()
        .find(|target| target.id == target_id)?;
    snippet
        .implementations
        .iter()
        .find(|implementation| implementation.id == target.implementation_id)
}

fn snippet_render_error_message(error: SnippetRenderError) -> String {
    match error {
        SnippetRenderError::MissingVariable(name) => format!("快捷命令缺少变量：{name}"),
        SnippetRenderError::UnknownVariable(name) => format!("快捷命令存在未声明变量：{name}"),
    }
}
