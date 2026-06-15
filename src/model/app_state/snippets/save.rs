//! 保存快捷命令。

use uuid::Uuid;

use crate::core::CoreState;
use crate::model::{HostId, Snippet, SnippetId, SnippetScope};

use super::super::AppUpdateOutcome;
use super::outcome::missing_host;

const HOST_SNIPPET_NAME_LIMIT: usize = 48;

impl CoreState {
    /// 将一条命令保存为主机级快捷命令。
    pub(crate) fn save_host_command_snippet_action(
        &mut self,
        host_id: HostId,
        command: String,
    ) -> AppUpdateOutcome {
        if !self.storage.hosts.iter().any(|host| host.id == host_id) {
            return missing_host(host_id);
        }

        let command = command.trim().to_owned();
        if command.is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令内容不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.storage
            .upsert_snippet(Snippet::with_default_implementation(
                SnippetId(Uuid::new_v4()),
                snippet_name(&command),
                Some("从主机命令草稿保存".to_owned()),
                SnippetScope::Host(host_id),
                None,
                command,
            ));

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
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
