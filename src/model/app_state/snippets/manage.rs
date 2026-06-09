//! 快捷命令维护。

use uuid::Uuid;

use crate::model::{
    Snippet, SnippetArgument, SnippetGroup, SnippetGroupId, SnippetId, SnippetImplementation,
    SnippetImplementationId, SnippetScope, SnippetShell, SnippetSupportTarget,
    SnippetSupportTargetId, SnippetVariable, variables_from_template,
};

use super::super::{AppState, AppUpdateOutcome};
use super::outcome::{missing_host, missing_snippet, missing_snippet_group};

const SNIPPET_NAME_LIMIT: usize = 64;
const SNIPPET_GROUP_NAME_LIMIT: usize = 48;
const SNIPPET_TARGET_KEY_LIMIT: usize = 64;
const SNIPPET_TARGET_NAME_LIMIT: usize = 48;

impl AppState {
    /// 创建通用快捷命令。
    pub(in crate::model::app_state) fn create_snippet(
        &mut self,
        name: String,
        description: String,
        command_template: String,
        scope: SnippetScope,
        group_id: Option<SnippetGroupId>,
    ) -> AppUpdateOutcome {
        let command_template = command_template.trim().to_owned();
        if command_template.is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令内容不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if let Some(error) = self.validate_snippet_location(&scope, group_id) {
            return error;
        }

        let variables = variables_from_template(&command_template);
        let snippet = Snippet::with_default_implementation(
            SnippetId(Uuid::new_v4()),
            normalized_snippet_name(&name, &command_template),
            normalized_optional_text(&description),
            scope,
            group_id,
            command_template,
        );
        debug_assert_eq!(snippet.variables, variables);
        self.storage.upsert_snippet(snippet);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 更新通用快捷命令。
    pub(in crate::model::app_state) fn update_snippet(
        &mut self,
        snippet_id: SnippetId,
        name: String,
        description: String,
        command_template: String,
        scope: SnippetScope,
        group_id: Option<SnippetGroupId>,
    ) -> AppUpdateOutcome {
        let Some(existing) = self
            .storage
            .snippets
            .iter()
            .find(|snippet| snippet.id == snippet_id)
            .cloned()
        else {
            return missing_snippet(snippet_id);
        };

        let command_template = command_template.trim().to_owned();
        if command_template.is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令内容不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if let Some(error) = self.validate_snippet_location(&scope, group_id) {
            return error;
        }

        let variables = variables_from_template(&command_template);
        let mut updated = existing;
        updated.name = normalized_snippet_name(&name, &command_template);
        updated.description = normalized_optional_text(&description);
        updated.scope = scope;
        updated.group_id = group_id;
        updated.variables = variables;
        let variables = updated.variables.clone();
        if let Some(implementation) = updated.default_implementation_mut() {
            implementation.command_template = command_template;
            implementation.last_arguments =
                keep_matching_arguments(implementation.last_arguments.clone(), &variables);
        }
        self.storage.upsert_snippet(updated);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 创建快捷命令分组。
    pub(in crate::model::app_state) fn create_snippet_group(
        &mut self,
        name: String,
        parent_id: Option<SnippetGroupId>,
    ) -> AppUpdateOutcome {
        let name = normalized_snippet_group_name(&name);
        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令分组名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if let Some(parent_id) = parent_id {
            if !self.storage.snippet_group_exists(parent_id) {
                return missing_snippet_group(parent_id);
            }
        }

        let sort_order = self
            .storage
            .snippet_groups
            .iter()
            .filter(|group| group.parent_id == parent_id)
            .map(|group| group.sort_order)
            .max()
            .map_or(0, |value| value + 1);
        self.storage.upsert_snippet_group(SnippetGroup {
            id: SnippetGroupId(Uuid::new_v4()),
            name,
            parent_id,
            sort_order,
        });

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 重命名快捷命令分组。
    pub(in crate::model::app_state) fn rename_snippet_group(
        &mut self,
        group_id: SnippetGroupId,
        name: String,
    ) -> AppUpdateOutcome {
        let name = normalized_snippet_group_name(&name);
        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("快捷命令分组名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if self.storage.rename_snippet_group(group_id, name) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            missing_snippet_group(group_id)
        }
    }

    /// 删除空快捷命令分组。
    pub(in crate::model::app_state) fn remove_snippet_group(
        &mut self,
        group_id: SnippetGroupId,
    ) -> AppUpdateOutcome {
        if self.storage.snippet_group_has_children(group_id) {
            return AppUpdateOutcome {
                error: Some("快捷命令分组非空，不能直接删除".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if self.storage.remove_snippet_group(group_id) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            missing_snippet_group(group_id)
        }
    }

    /// 递归删除快捷命令分组、子分组和分组内快捷命令。
    pub(in crate::model::app_state) fn remove_snippet_group_recursive(
        &mut self,
        group_id: SnippetGroupId,
    ) -> AppUpdateOutcome {
        if self.storage.remove_snippet_group_recursive(group_id) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            missing_snippet_group(group_id)
        }
    }

    /// 移动快捷命令分组。
    pub(in crate::model::app_state) fn move_snippet_group(
        &mut self,
        group_id: SnippetGroupId,
        parent_id: Option<SnippetGroupId>,
    ) -> AppUpdateOutcome {
        if self.storage.move_snippet_group(group_id, parent_id) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some("快捷命令分组移动失败".to_owned()),
                ..AppUpdateOutcome::default()
            }
        }
    }

    /// 移动快捷命令到指定分组。
    pub(in crate::model::app_state) fn move_snippet(
        &mut self,
        snippet_id: SnippetId,
        group_id: Option<SnippetGroupId>,
    ) -> AppUpdateOutcome {
        if self.storage.move_snippet(snippet_id, group_id) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some("快捷命令移动失败".to_owned()),
                ..AppUpdateOutcome::default()
            }
        }
    }

    /// 删除指定快捷命令。
    pub(in crate::model::app_state) fn remove_snippet(
        &mut self,
        snippet_id: SnippetId,
    ) -> AppUpdateOutcome {
        if self.storage.remove_snippet(snippet_id) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            missing_snippet(snippet_id)
        }
    }

    /// 为片段新增支持目标；可以新建独立实现，也可以共享已有目标的实现。
    pub(in crate::model::app_state) fn create_snippet_targets(
        &mut self,
        snippet_id: SnippetId,
        target_keys: Vec<String>,
        display_name: String,
        command_template: String,
        share_target_id: Option<SnippetSupportTargetId>,
    ) -> AppUpdateOutcome {
        let target_keys = normalized_target_keys(target_keys);
        if target_keys.is_empty() {
            return AppUpdateOutcome {
                error: Some("至少需要选择一个支持目标".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        for target_key in target_keys {
            let outcome = self.create_snippet_target(
                snippet_id,
                target_key,
                display_name.clone(),
                command_template.clone(),
                share_target_id,
            );
            if !outcome.changed() {
                return outcome;
            }
        }

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 为片段新增一个支持目标；可以新建独立实现，也可以共享已有目标的实现。
    pub(in crate::model::app_state) fn create_snippet_target(
        &mut self,
        snippet_id: SnippetId,
        target_key: String,
        display_name: String,
        command_template: String,
        share_target_id: Option<SnippetSupportTargetId>,
    ) -> AppUpdateOutcome {
        let Some(mut snippet) = self
            .storage
            .snippets
            .iter()
            .find(|snippet| snippet.id == snippet_id)
            .cloned()
        else {
            return missing_snippet(snippet_id);
        };

        let target_key = normalized_target_key(&target_key);
        if target_key.is_empty() {
            return AppUpdateOutcome {
                error: Some("支持目标标记不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if snippet
            .support_targets
            .iter()
            .any(|target| target.target_key == target_key)
        {
            return AppUpdateOutcome {
                error: Some(format!("支持目标已存在：{target_key}")),
                ..AppUpdateOutcome::default()
            };
        }

        let display_name = normalized_target_name(&display_name, &target_key);
        let implementation_id = if let Some(share_target_id) = share_target_id {
            let Some(shared_target) = snippet
                .support_targets
                .iter()
                .find(|target| target.id == share_target_id)
            else {
                return AppUpdateOutcome {
                    error: Some("找不到要共享的支持目标".to_owned()),
                    ..AppUpdateOutcome::default()
                };
            };
            shared_target.implementation_id
        } else {
            let command_template = command_template.trim().to_owned();
            if command_template.is_empty() {
                return AppUpdateOutcome {
                    error: Some("脚本内容不能为空".to_owned()),
                    ..AppUpdateOutcome::default()
                };
            }
            let implementation_id = SnippetImplementationId(Uuid::new_v4());
            snippet.implementations.push(SnippetImplementation {
                id: implementation_id,
                snippet_id,
                name: format!("{display_name} 脚本"),
                shell: default_shell_for_target(target_key.as_str()),
                command_template,
                notes: None,
                last_arguments: Vec::new(),
                sort_order: next_implementation_sort_order(&snippet),
            });
            implementation_id
        };

        snippet.support_targets.push(SnippetSupportTarget {
            id: SnippetSupportTargetId(Uuid::new_v4()),
            snippet_id,
            target_key,
            display_name,
            implementation_id,
            sort_order: next_target_sort_order(&snippet),
        });
        refresh_snippet_variables(&mut snippet);
        self.storage.upsert_snippet(snippet);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 更新支持目标及其当前指向的脚本实现；共享实现会同步影响其它目标。
    pub(in crate::model::app_state) fn update_snippet_target(
        &mut self,
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
        target_key: String,
        display_name: String,
        command_template: String,
    ) -> AppUpdateOutcome {
        let Some(mut snippet) = self
            .storage
            .snippets
            .iter()
            .find(|snippet| snippet.id == snippet_id)
            .cloned()
        else {
            return missing_snippet(snippet_id);
        };

        let Some(target_index) = snippet
            .support_targets
            .iter()
            .position(|target| target.id == target_id)
        else {
            return AppUpdateOutcome {
                error: Some("找不到支持目标".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        let target_key = normalized_target_key(&target_key);
        if target_key.is_empty() {
            return AppUpdateOutcome {
                error: Some("支持目标标记不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if snippet
            .support_targets
            .iter()
            .enumerate()
            .any(|(index, target)| index != target_index && target.target_key == target_key)
        {
            return AppUpdateOutcome {
                error: Some(format!("支持目标已存在：{target_key}")),
                ..AppUpdateOutcome::default()
            };
        }

        let command_template = command_template.trim().to_owned();
        if command_template.is_empty() {
            return AppUpdateOutcome {
                error: Some("脚本内容不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let display_name = normalized_target_name(&display_name, &target_key);
        let implementation_id = snippet.support_targets[target_index].implementation_id;
        snippet.support_targets[target_index].target_key = target_key.clone();
        snippet.support_targets[target_index].display_name = display_name.clone();
        let Some(implementation) = snippet
            .implementations
            .iter_mut()
            .find(|implementation| implementation.id == implementation_id)
        else {
            return AppUpdateOutcome {
                error: Some("支持目标没有可编辑脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        implementation.name = format!("{display_name} 脚本");
        implementation.shell = default_shell_for_target(target_key.as_str());
        implementation.command_template = command_template;

        refresh_snippet_variables(&mut snippet);
        self.storage.upsert_snippet(snippet);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 删除支持目标；无其它目标引用的脚本实现会一并删除。
    pub(in crate::model::app_state) fn remove_snippet_target(
        &mut self,
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
    ) -> AppUpdateOutcome {
        let Some(mut snippet) = self
            .storage
            .snippets
            .iter()
            .find(|snippet| snippet.id == snippet_id)
            .cloned()
        else {
            return missing_snippet(snippet_id);
        };
        if snippet.support_targets.len() <= 1 {
            return AppUpdateOutcome {
                error: Some("至少需要保留一个支持目标".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(target_index) = snippet
            .support_targets
            .iter()
            .position(|target| target.id == target_id)
        else {
            return AppUpdateOutcome {
                error: Some("找不到支持目标".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let implementation_id = snippet.support_targets[target_index].implementation_id;
        snippet.support_targets.remove(target_index);
        if !snippet
            .support_targets
            .iter()
            .any(|target| target.implementation_id == implementation_id)
        {
            snippet
                .implementations
                .retain(|implementation| implementation.id != implementation_id);
        }
        refresh_snippet_variables(&mut snippet);
        self.storage.upsert_snippet(snippet);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 将共享脚本实现拆分成目标自己的副本，后续编辑不再影响其它目标。
    pub(in crate::model::app_state) fn split_snippet_target_implementation(
        &mut self,
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
    ) -> AppUpdateOutcome {
        let Some(mut snippet) = self
            .storage
            .snippets
            .iter()
            .find(|snippet| snippet.id == snippet_id)
            .cloned()
        else {
            return missing_snippet(snippet_id);
        };
        let Some(target_index) = snippet
            .support_targets
            .iter()
            .position(|target| target.id == target_id)
        else {
            return AppUpdateOutcome {
                error: Some("找不到支持目标".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let implementation_id = snippet.support_targets[target_index].implementation_id;
        let reference_count = snippet
            .support_targets
            .iter()
            .filter(|target| target.implementation_id == implementation_id)
            .count();
        if reference_count <= 1 {
            return AppUpdateOutcome::default();
        }
        let Some(source) = snippet
            .implementations
            .iter()
            .find(|implementation| implementation.id == implementation_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some("支持目标没有可拆分脚本".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        let new_id = SnippetImplementationId(Uuid::new_v4());
        let mut new_implementation = source;
        new_implementation.id = new_id;
        new_implementation.name = format!(
            "{} 脚本",
            snippet.support_targets[target_index].display_name
        );
        new_implementation.last_arguments = Vec::new();
        new_implementation.sort_order = next_implementation_sort_order(&snippet);
        snippet.implementations.push(new_implementation);
        snippet.support_targets[target_index].implementation_id = new_id;
        self.storage.upsert_snippet(snippet);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 更新快捷命令变量最近一次输入值。
    pub(in crate::model::app_state) fn update_snippet_argument(
        &mut self,
        snippet_id: SnippetId,
        name: String,
        value: String,
    ) -> AppUpdateOutcome {
        if self
            .storage
            .upsert_snippet_argument(snippet_id, &name, value)
        {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到快捷命令变量：{name}")),
                ..AppUpdateOutcome::default()
            }
        }
    }

    fn validate_snippet_location(
        &self,
        scope: &SnippetScope,
        group_id: Option<SnippetGroupId>,
    ) -> Option<AppUpdateOutcome> {
        if let Some(group_id) = group_id {
            if !self.storage.snippet_group_exists(group_id) {
                return Some(missing_snippet_group(group_id));
            }
        }

        match scope {
            SnippetScope::Global => None,
            SnippetScope::Host(host_id) => {
                if self.storage.hosts.iter().any(|host| host.id == *host_id) {
                    None
                } else {
                    Some(missing_host(*host_id))
                }
            }
        }
    }
}

fn normalized_snippet_name(name: &str, command_template: &str) -> String {
    let raw_name = if name.trim().is_empty() {
        command_template
    } else {
        name
    };
    raw_name.trim().chars().take(SNIPPET_NAME_LIMIT).collect()
}

fn normalized_snippet_group_name(name: &str) -> String {
    name.trim().chars().take(SNIPPET_GROUP_NAME_LIMIT).collect()
}

fn normalized_target_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .take(SNIPPET_TARGET_KEY_LIMIT)
        .collect()
}

fn normalized_target_keys(values: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        let target_key = normalized_target_key(&value);
        if !target_key.is_empty() && !normalized.contains(&target_key) {
            normalized.push(target_key);
        }
    }
    normalized
}

fn normalized_target_name(name: &str, target_key: &str) -> String {
    let raw_name = if name.trim().is_empty() {
        target_key
    } else {
        name
    };
    raw_name
        .trim()
        .chars()
        .take(SNIPPET_TARGET_NAME_LIMIT)
        .collect()
}

fn normalized_optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn refresh_snippet_variables(snippet: &mut Snippet) {
    let previous = std::mem::take(&mut snippet.variables);
    let mut variables = Vec::new();
    for implementation in &snippet.implementations {
        for variable in variables_from_template(implementation.command_template.as_str()) {
            if !variables
                .iter()
                .any(|existing: &SnippetVariable| existing.name == variable.name)
            {
                let merged = previous
                    .iter()
                    .find(|existing| existing.name == variable.name)
                    .cloned()
                    .unwrap_or(variable);
                variables.push(merged);
            }
        }
    }
    for implementation in &mut snippet.implementations {
        implementation.last_arguments =
            keep_matching_arguments(implementation.last_arguments.clone(), &variables);
    }
    snippet.variables = variables;
}

fn default_shell_for_target(target_key: &str) -> SnippetShell {
    match target_key {
        "windows" | "win" | "powershell" | "pwsh" | "windows-powershell" => {
            SnippetShell::PowerShell
        }
        "cmd" | "windows-cmd" => SnippetShell::Cmd,
        _ => SnippetShell::Bash,
    }
}

fn next_target_sort_order(snippet: &Snippet) -> i32 {
    snippet
        .support_targets
        .iter()
        .map(|target| target.sort_order)
        .max()
        .map_or(0, |value| value + 1)
}

fn next_implementation_sort_order(snippet: &Snippet) -> i32 {
    snippet
        .implementations
        .iter()
        .map(|implementation| implementation.sort_order)
        .max()
        .map_or(0, |value| value + 1)
}

fn keep_matching_arguments(
    arguments: Vec<SnippetArgument>,
    variables: &[crate::model::SnippetVariable],
) -> Vec<SnippetArgument> {
    arguments
        .into_iter()
        .filter(|argument| {
            variables
                .iter()
                .any(|variable| variable.name == argument.name)
        })
        .collect()
}
