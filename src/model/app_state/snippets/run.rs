//! 执行快捷命令。

use crate::core::CoreState;
use crate::model::{
    HostId, Snippet, SnippetArgument, SnippetId, SnippetImplementation, SnippetRenderError,
    SnippetSupportTargetId,
};

use super::super::AppUpdateOutcome;
use super::outcome::{missing_host, missing_snippet};

impl CoreState {
    /// 使用默认支持目标渲染并运行适用于主机的快捷命令。
    pub(crate) fn run_snippet_action(
        &mut self,
        host_id: HostId,
        snippet_id: SnippetId,
    ) -> AppUpdateOutcome {
        let Some((_, snippet)) = validated_host_snippet(self, host_id, snippet_id) else {
            return missing_snippet_or_host(self, host_id, snippet_id);
        };
        let Some(implementation) = snippet.default_implementation() else {
            return AppUpdateOutcome {
                error: Some("快捷命令没有可执行脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command = match rendered_snippet_command(
            implementation,
            &snippet,
            &implementation.last_arguments,
        ) {
            Ok(command) => command,
            Err(outcome) => return outcome,
        };

        self.run_remote_command(host_id, command, false)
    }

    /// 使用本次输入参数渲染并运行默认支持目标。
    pub(crate) fn run_snippet_with_arguments_action(
        &mut self,
        host_id: HostId,
        snippet_id: SnippetId,
        arguments: Vec<SnippetArgument>,
    ) -> AppUpdateOutcome {
        let Some((_, snippet)) = validated_host_snippet(self, host_id, snippet_id) else {
            return missing_snippet_or_host(self, host_id, snippet_id);
        };
        let valid_arguments = filtered_snippet_arguments(&snippet, arguments);
        let Some(implementation) = snippet.default_implementation() else {
            return AppUpdateOutcome {
                error: Some("快捷命令没有可执行脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command = match rendered_snippet_command(implementation, &snippet, &valid_arguments) {
            Ok(command) => command,
            Err(outcome) => return outcome,
        };

        self.storage
            .record_snippet_arguments(snippet_id, valid_arguments);
        self.run_remote_command(host_id, command, false)
    }

    /// 使用指定支持目标渲染并运行快捷命令。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn run_snippet_target_action(
        &mut self,
        host_id: HostId,
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
    ) -> AppUpdateOutcome {
        let Some((_, snippet)) = validated_host_snippet(self, host_id, snippet_id) else {
            return missing_snippet_or_host(self, host_id, snippet_id);
        };
        let Some(implementation) = snippet_implementation_for_target(&snippet, target_id) else {
            return AppUpdateOutcome {
                error: Some("快捷命令支持目标没有可执行脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command = match rendered_snippet_command(
            implementation,
            &snippet,
            &implementation.last_arguments,
        ) {
            Ok(command) => command,
            Err(outcome) => return outcome,
        };

        self.run_remote_command(host_id, command, false)
    }

    /// 使用本次输入参数渲染并运行指定支持目标。
    pub(crate) fn run_snippet_target_with_arguments_action(
        &mut self,
        host_id: HostId,
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
        arguments: Vec<SnippetArgument>,
    ) -> AppUpdateOutcome {
        let Some((_, snippet)) = validated_host_snippet(self, host_id, snippet_id) else {
            return missing_snippet_or_host(self, host_id, snippet_id);
        };
        let valid_arguments = filtered_snippet_arguments(&snippet, arguments);
        let Some(implementation) = snippet_implementation_for_target(&snippet, target_id) else {
            return AppUpdateOutcome {
                error: Some("快捷命令支持目标没有可执行脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command = match rendered_snippet_command(implementation, &snippet, &valid_arguments) {
            Ok(command) => command,
            Err(outcome) => return outcome,
        };

        self.storage
            .record_snippet_implementation_arguments(implementation.id, valid_arguments);
        self.run_remote_command(host_id, command, false)
    }
}

fn validated_host_snippet(
    state: &CoreState,
    host_id: HostId,
    snippet_id: SnippetId,
) -> Option<(crate::model::Host, Snippet)> {
    let host = state.host_by_id(host_id)?;
    let snippet = state
        .storage
        .snippets
        .iter()
        .find(|snippet| snippet.id == snippet_id)
        .cloned()?;
    if !snippet.scope.applies_to_host(&host) {
        return None;
    }
    Some((host, snippet))
}

fn missing_snippet_or_host(
    state: &CoreState,
    host_id: HostId,
    snippet_id: SnippetId,
) -> AppUpdateOutcome {
    if state.host_by_id(host_id).is_none() {
        return missing_host(host_id);
    }

    let Some(snippet) = state
        .storage
        .snippets
        .iter()
        .find(|snippet| snippet.id == snippet_id)
    else {
        return missing_snippet(snippet_id);
    };

    let host = state
        .host_by_id(host_id)
        .expect("已确认主机存在，读取不应失败");
    if !snippet.scope.applies_to_host(&host) {
        return AppUpdateOutcome {
            error: Some(format!("快捷命令不适用于主机：{}", host.name)),
            ..AppUpdateOutcome::default()
        };
    }

    missing_snippet(snippet_id)
}

fn filtered_snippet_arguments(
    snippet: &Snippet,
    arguments: Vec<SnippetArgument>,
) -> Vec<SnippetArgument> {
    arguments
        .into_iter()
        .filter(|argument| {
            snippet
                .variables
                .iter()
                .any(|variable| variable.name == argument.name)
        })
        .collect()
}

fn rendered_snippet_command(
    implementation: &SnippetImplementation,
    snippet: &Snippet,
    arguments: &[SnippetArgument],
) -> Result<String, AppUpdateOutcome> {
    let command = match implementation.render(&snippet.variables, arguments) {
        Ok(command) => command,
        Err(error) => {
            return Err(AppUpdateOutcome {
                error: Some(snippet_render_error_message(error)),
                ..AppUpdateOutcome::default()
            });
        }
    };
    if command.trim().is_empty() {
        return Err(AppUpdateOutcome {
            error: Some("快捷命令渲染结果不能为空".to_owned()),
            ..AppUpdateOutcome::default()
        });
    }
    Ok(command)
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
