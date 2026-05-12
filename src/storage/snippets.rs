//! 快捷命令的内存索引操作。

use crate::model::{Snippet, SnippetArgument, SnippetId, SnippetScope};

use super::StorageManager;

impl StorageManager {
    /// 保存或更新快捷命令。
    pub fn upsert_snippet(&mut self, snippet: Snippet) {
        if let Some(existing) = self
            .snippets
            .iter_mut()
            .find(|existing| existing.id == snippet.id)
        {
            *existing = snippet;
        } else {
            self.snippets.push(snippet);
        }
    }

    /// 删除快捷命令。
    pub fn remove_snippet(&mut self, id: SnippetId) -> bool {
        let before = self.snippets.len();
        self.snippets.retain(|snippet| snippet.id != id);
        before != self.snippets.len()
    }

    /// 按作用域查询快捷命令。
    pub fn snippets_for_scope(&self, scope: &SnippetScope) -> Vec<&Snippet> {
        self.snippets
            .iter()
            .filter(|snippet| &snippet.scope == scope)
            .collect()
    }

    /// 记录快捷命令最近一次参数。
    pub fn record_snippet_arguments(
        &mut self,
        id: SnippetId,
        arguments: Vec<SnippetArgument>,
    ) -> bool {
        if let Some(snippet) = self.snippets.iter_mut().find(|snippet| snippet.id == id) {
            snippet.last_arguments = arguments;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HostId, SnippetVariable};
    use uuid::Uuid;

    #[test]
    fn snippets_can_be_upserted_filtered_removed_and_record_arguments() {
        let mut storage = StorageManager::default();
        let snippet_id = SnippetId(Uuid::new_v4());
        let host_id = HostId(Uuid::new_v4());
        let mut snippet = Snippet {
            id: snippet_id,
            name: "restart".to_owned(),
            description: None,
            command_template: "systemctl restart {{service}}".to_owned(),
            scope: SnippetScope::Host(host_id),
            variables: vec![SnippetVariable {
                name: "service".to_owned(),
                default_value: None,
                required: true,
            }],
            last_arguments: Vec::new(),
        };

        storage.upsert_snippet(snippet.clone());
        snippet.name = "restart service".to_owned();
        storage.upsert_snippet(snippet);

        assert_eq!(storage.snippet_count(), 1);
        assert_eq!(
            storage
                .snippets_for_scope(&SnippetScope::Host(host_id))
                .len(),
            1
        );
        assert!(storage.record_snippet_arguments(
            snippet_id,
            vec![SnippetArgument {
                name: "service".to_owned(),
                value: "sshd".to_owned(),
            }],
        ));
        assert_eq!(storage.snippets[0].last_arguments[0].value, "sshd");
        assert!(!storage.record_snippet_arguments(SnippetId(Uuid::new_v4()), Vec::new()));
        assert!(storage.remove_snippet(snippet_id));
        assert!(!storage.remove_snippet(snippet_id));
    }
}
