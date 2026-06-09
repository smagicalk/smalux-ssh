//! 快捷命令的内存索引操作。

use smagical_core::{
    Snippet, SnippetArgument, SnippetGroup, SnippetGroupId, SnippetId, SnippetImplementationId,
    SnippetScope,
};

use super::StorageManager;

impl StorageManager {
    /// 保存或更新快捷命令分组。
    pub fn upsert_snippet_group(&mut self, group: SnippetGroup) {
        if let Some(existing) = self
            .snippet_groups
            .iter_mut()
            .find(|existing| existing.id == group.id)
        {
            *existing = group;
        } else {
            self.snippet_groups.push(group);
        }
    }

    /// 判断快捷命令分组是否存在。
    pub fn snippet_group_exists(&self, group_id: SnippetGroupId) -> bool {
        self.snippet_groups.iter().any(|group| group.id == group_id)
    }

    /// 判断快捷命令分组是否包含子分组或片段。
    pub fn snippet_group_has_children(&self, group_id: SnippetGroupId) -> bool {
        self.snippet_groups
            .iter()
            .any(|group| group.parent_id == Some(group_id))
            || self
                .snippets
                .iter()
                .any(|snippet| snippet.group_id == Some(group_id))
    }

    /// 修改快捷命令分组名称。
    pub fn rename_snippet_group(&mut self, group_id: SnippetGroupId, name: String) -> bool {
        if let Some(group) = self
            .snippet_groups
            .iter_mut()
            .find(|group| group.id == group_id)
        {
            group.name = name;
            true
        } else {
            false
        }
    }

    /// 移动快捷命令分组到新的父节点。
    pub fn move_snippet_group(
        &mut self,
        group_id: SnippetGroupId,
        parent_id: Option<SnippetGroupId>,
    ) -> bool {
        if parent_id == Some(group_id) {
            return false;
        }
        if parent_id.is_some_and(|parent_id| !self.snippet_group_exists(parent_id)) {
            return false;
        }
        if parent_id.is_some_and(|parent_id| self.snippet_group_is_descendant(parent_id, group_id))
        {
            return false;
        }

        if let Some(group) = self
            .snippet_groups
            .iter_mut()
            .find(|group| group.id == group_id)
        {
            group.parent_id = parent_id;
            true
        } else {
            false
        }
    }

    /// 删除空快捷命令分组。
    pub fn remove_snippet_group(&mut self, group_id: SnippetGroupId) -> bool {
        if self.snippet_group_has_children(group_id) {
            return false;
        }

        let before = self.snippet_groups.len();
        self.snippet_groups.retain(|group| group.id != group_id);
        before != self.snippet_groups.len()
    }

    /// 递归删除快捷命令分组、子分组和分组内片段。
    pub fn remove_snippet_group_recursive(&mut self, group_id: SnippetGroupId) -> bool {
        if !self.snippet_group_exists(group_id) {
            return false;
        }

        let mut group_ids = vec![group_id];
        let mut cursor = 0;
        while cursor < group_ids.len() {
            let parent_id = group_ids[cursor];
            for child in self
                .snippet_groups
                .iter()
                .filter(|group| group.parent_id == Some(parent_id))
            {
                if !group_ids.contains(&child.id) {
                    group_ids.push(child.id);
                }
            }
            cursor += 1;
        }

        self.snippets
            .retain(|snippet| !snippet.group_id.is_some_and(|id| group_ids.contains(&id)));
        self.snippet_groups
            .retain(|group| !group_ids.contains(&group.id));
        true
    }

    /// 移动快捷命令到新的分组；None 表示根目录。
    pub fn move_snippet(
        &mut self,
        snippet_id: SnippetId,
        group_id: Option<SnippetGroupId>,
    ) -> bool {
        if group_id.is_some_and(|group_id| !self.snippet_group_exists(group_id)) {
            return false;
        }
        if let Some(snippet) = self
            .snippets
            .iter_mut()
            .find(|snippet| snippet.id == snippet_id)
        {
            snippet.group_id = group_id;
            true
        } else {
            false
        }
    }

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
            if let Some(implementation) = snippet.implementations.first_mut() {
                implementation.last_arguments = arguments;
            }
            true
        } else {
            false
        }
    }

    /// 记录指定脚本实现最近一次参数。
    pub fn record_snippet_implementation_arguments(
        &mut self,
        id: SnippetImplementationId,
        arguments: Vec<SnippetArgument>,
    ) -> bool {
        for snippet in &mut self.snippets {
            if let Some(implementation) = snippet
                .implementations
                .iter_mut()
                .find(|implementation| implementation.id == id)
            {
                implementation.last_arguments = arguments;
                return true;
            }
        }
        false
    }

    /// 写入单个快捷命令参数，保留其它最近参数。
    pub fn upsert_snippet_argument(&mut self, id: SnippetId, name: &str, value: String) -> bool {
        let Some(snippet) = self.snippets.iter_mut().find(|snippet| snippet.id == id) else {
            return false;
        };
        if !snippet
            .variables
            .iter()
            .any(|variable| variable.name == name)
        {
            return false;
        }

        let Some(implementation) = snippet.implementations.first_mut() else {
            return false;
        };

        if let Some(argument) = implementation
            .last_arguments
            .iter_mut()
            .find(|argument| argument.name == name)
        {
            argument.value = value;
        } else {
            implementation.last_arguments.push(SnippetArgument {
                name: name.to_owned(),
                value,
            });
        }

        true
    }

    fn snippet_group_is_descendant(
        &self,
        group_id: SnippetGroupId,
        ancestor_id: SnippetGroupId,
    ) -> bool {
        let mut current_id = Some(group_id);
        while let Some(id) = current_id {
            let Some(group) = self.snippet_groups.iter().find(|group| group.id == id) else {
                return false;
            };
            if group.parent_id == Some(ancestor_id) {
                return true;
            }
            current_id = group.parent_id;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::HostId;
    use uuid::Uuid;

    #[test]
    fn snippets_can_be_upserted_filtered_removed_and_record_arguments() {
        let mut storage = StorageManager::default();
        let snippet_id = SnippetId(Uuid::new_v4());
        let host_id = HostId(Uuid::new_v4());
        let mut snippet = Snippet::with_default_implementation(
            snippet_id,
            "restart".to_owned(),
            None,
            SnippetScope::Host(host_id),
            None,
            "systemctl restart {{service}}".to_owned(),
        );

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
        assert_eq!(
            storage.snippets[0]
                .default_implementation()
                .expect("默认实现应存在")
                .last_arguments[0]
                .value,
            "sshd"
        );
        assert!(storage.upsert_snippet_argument(snippet_id, "service", "nginx".to_owned()));
        assert_eq!(
            storage.snippets[0]
                .default_implementation()
                .expect("默认实现应存在")
                .last_arguments[0]
                .value,
            "nginx"
        );
        assert!(!storage.upsert_snippet_argument(snippet_id, "unknown", "value".to_owned()));
        assert!(!storage.record_snippet_arguments(SnippetId(Uuid::new_v4()), Vec::new()));
        assert!(storage.remove_snippet(snippet_id));
        assert!(!storage.remove_snippet(snippet_id));
    }

    #[test]
    fn snippet_groups_can_be_upserted_moved_and_removed_when_empty() {
        let mut storage = StorageManager::default();
        let parent_id = SnippetGroupId(Uuid::new_v4());
        let child_id = SnippetGroupId(Uuid::new_v4());
        let snippet_id = SnippetId(Uuid::new_v4());

        storage.upsert_snippet_group(SnippetGroup {
            id: parent_id,
            name: "运维".to_owned(),
            parent_id: None,
            sort_order: 0,
        });
        storage.upsert_snippet_group(SnippetGroup {
            id: child_id,
            name: "服务".to_owned(),
            parent_id: Some(parent_id),
            sort_order: 1,
        });
        storage.upsert_snippet(Snippet::with_default_implementation(
            snippet_id,
            "restart".to_owned(),
            None,
            SnippetScope::Global,
            Some(child_id),
            "systemctl restart nginx".to_owned(),
        ));

        assert!(storage.rename_snippet_group(parent_id, "生产".to_owned()));
        assert_eq!(storage.snippet_groups[0].name, "生产");
        assert!(storage.snippet_group_has_children(parent_id));
        assert!(!storage.remove_snippet_group(parent_id));
        assert!(!storage.move_snippet_group(parent_id, Some(child_id)));
        assert!(storage.move_snippet(snippet_id, Some(parent_id)));
        assert!(storage.move_snippet_group(child_id, None));
        assert!(storage.remove_snippet_group(child_id));
        assert_eq!(storage.snippet_group_count(), 1);
        assert_eq!(storage.snippets[0].group_id, Some(parent_id));
        assert!(!storage.move_snippet(snippet_id, Some(SnippetGroupId(Uuid::new_v4()))));
    }

    #[test]
    fn snippet_groups_can_be_removed_recursively_with_children_and_snippets() {
        let mut storage = StorageManager::default();
        let parent_id = SnippetGroupId(Uuid::new_v4());
        let child_id = SnippetGroupId(Uuid::new_v4());
        let sibling_id = SnippetGroupId(Uuid::new_v4());
        let snippet_id = SnippetId(Uuid::new_v4());

        storage.upsert_snippet_group(SnippetGroup {
            id: parent_id,
            name: "运维".to_owned(),
            parent_id: None,
            sort_order: 0,
        });
        storage.upsert_snippet_group(SnippetGroup {
            id: child_id,
            name: "服务".to_owned(),
            parent_id: Some(parent_id),
            sort_order: 1,
        });
        storage.upsert_snippet_group(SnippetGroup {
            id: sibling_id,
            name: "数据库".to_owned(),
            parent_id: None,
            sort_order: 2,
        });
        storage.upsert_snippet(Snippet::with_default_implementation(
            snippet_id,
            "restart".to_owned(),
            None,
            SnippetScope::Global,
            Some(child_id),
            "systemctl restart nginx".to_owned(),
        ));

        assert!(storage.remove_snippet_group_recursive(parent_id));
        assert_eq!(storage.snippet_group_count(), 1);
        assert_eq!(storage.snippet_count(), 0);
        assert_eq!(storage.snippet_groups[0].id, sibling_id);
        assert!(!storage.remove_snippet_group_recursive(parent_id));
    }
}
