//! 执行快捷命令。

use crate::model::{HostId, SnippetId, SnippetRenderError};

use super::super::{AppState, AppUpdateOutcome};
use super::outcome::{missing_host, missing_snippet};

impl AppState {
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

        let command = match snippet.render(&snippet.last_arguments) {
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
}

fn snippet_render_error_message(error: SnippetRenderError) -> String {
    match error {
        SnippetRenderError::MissingVariable(name) => format!("快捷命令缺少变量：{name}"),
        SnippetRenderError::UnknownVariable(name) => format!("快捷命令存在未声明变量：{name}"),
    }
}
