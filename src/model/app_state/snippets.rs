//! 快捷命令的应用状态行为。

use uuid::Uuid;

use crate::model::{HostId, Snippet, SnippetId, SnippetRenderError, SnippetScope};

use super::{AppState, AppUpdateOutcome};

const HOST_SNIPPET_NAME_LIMIT: usize = 48;

impl AppState {
    /// 将当前主机命令草稿保存为主机级快捷命令。
    pub(super) fn save_host_command_snippet(&mut self, host_id: HostId) -> AppUpdateOutcome {
        if !self.storage.hosts.iter().any(|host| host.id == host_id) {
            return missing_host(host_id);
        }

        let command = self.ui.remote_command_for(host_id).trim().to_owned();
        if command.is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令内容不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.storage.upsert_snippet(Snippet {
            id: SnippetId(Uuid::new_v4()),
            name: snippet_name(&command),
            description: Some("从主机命令草稿保存".to_owned()),
            command_template: command,
            scope: SnippetScope::Host(host_id),
            variables: Vec::new(),
            last_arguments: Vec::new(),
        });

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 渲染并执行适用于主机的快捷命令。
    pub(super) fn run_snippet(
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

        self.run_remote_command(host_id, command, false)
    }

    /// 删除指定快捷命令。
    pub(super) fn remove_snippet(&mut self, snippet_id: SnippetId) -> AppUpdateOutcome {
        if self.storage.remove_snippet(snippet_id) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            missing_snippet(snippet_id)
        }
    }
}

fn snippet_name(command: &str) -> String {
    let mut name: String = command.chars().take(HOST_SNIPPET_NAME_LIMIT).collect();
    if command.chars().count() > HOST_SNIPPET_NAME_LIMIT {
        name.push_str("...");
    }
    name
}

fn missing_host(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

fn missing_snippet(snippet_id: SnippetId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到快捷命令：{}", snippet_id.0)),
        ..AppUpdateOutcome::default()
    }
}

fn snippet_render_error_message(error: SnippetRenderError) -> String {
    match error {
        SnippetRenderError::MissingVariable(name) => format!("快捷命令缺少变量：{name}"),
        SnippetRenderError::UnknownVariable(name) => format!("快捷命令存在未声明变量：{name}"),
    }
}
