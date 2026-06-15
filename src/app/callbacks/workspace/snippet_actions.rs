//! 工作区片段页回调。
//!
//! 片段页有树形结构、片段本体、目标变体和运行参数几个概念。这里只把 Slint 传来的
//! 字符串 ID 和文本参数解析成 `Message`，片段创建、移动、执行的业务规则仍在核心状态里。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::app::callbacks::{AppWindow, SharedAppState, apply_and_sync, apply_and_sync_success};
use crate::model::{Message, SnippetScope};

use super::super::parse_host_id;
use super::snippet_helpers::{
    parse_optional_snippet_group_node_id, parse_snippet_arguments_text,
    parse_snippet_group_node_id, parse_snippet_row_id, parse_snippet_target_row_id,
};

pub(super) fn bind(window: &AppWindow, state: &SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_update_snippet_search(move |query| {
            // 片段页搜索只影响片段树投影，不改变持久化数据。
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateSnippetSearchQuery {
                    query: query.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_toggle_snippet_tree_node(move |node_id| {
            apply_and_sync(
                &weak,
                &state,
                Message::ToggleSnippetTreeNode {
                    node_id: node_id.to_string(),
                },
            );
        });
    }
    {
        let state = Rc::clone(state);
        window.on_snippet_group_name(move |group_id| {
            let state = state.borrow();
            let Some(group_id) = parse_snippet_group_node_id(group_id.as_str()) else {
                return "根分组".into();
            };
            state
                .core
                .storage
                .snippet_groups
                .iter()
                .find(|group| group.id == group_id)
                .map(|group| group.name.as_str())
                .unwrap_or("根分组")
                .into()
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_create_snippet(
            move |parent_node_id, group_id, name, description, command_template| {
                let target_group_id = if group_id.trim().is_empty() {
                    parse_optional_snippet_group_node_id(parent_node_id.as_str())
                } else {
                    parse_optional_snippet_group_node_id(group_id.as_str())
                };
                apply_and_sync_success(
                    &weak,
                    &state,
                    Message::CreateSnippet {
                        name: name.to_string(),
                        description: description.to_string(),
                        command_template: command_template.to_string(),
                        scope: SnippetScope::Global,
                        group_id: target_group_id,
                    },
                )
            },
        );
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_update_snippet(
            move |snippet_row_id, group_id, name, description, command_template| {
                let Some(snippet_id) = parse_snippet_row_id(snippet_row_id.as_str()) else {
                    return false;
                };
                let scope = {
                    let state = state.borrow();
                    let Some(snippet) = state
                        .core
                        .storage
                        .snippets
                        .iter()
                        .find(|snippet| snippet.id == snippet_id)
                    else {
                        return false;
                    };
                    snippet.scope.clone()
                };
                apply_and_sync_success(
                    &weak,
                    &state,
                    Message::UpdateSnippet {
                        snippet_id,
                        name: name.to_string(),
                        description: description.to_string(),
                        command_template: command_template.to_string(),
                        scope,
                        group_id: parse_optional_snippet_group_node_id(group_id.as_str()),
                    },
                )
            },
        );
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_create_snippet_target(
            move |snippet_row_id,
                  target_key,
                  display_name,
                  command_template,
                  share_target_row_id| {
                let Some(snippet_id) = parse_snippet_row_id(snippet_row_id.as_str()) else {
                    return false;
                };
                let share_target_id = parse_snippet_target_row_id(share_target_row_id.as_str())
                    .map(|(_, target_id)| target_id);
                apply_and_sync_success(
                    &weak,
                    &state,
                    Message::CreateSnippetTarget {
                        snippet_id,
                        target_keys: parse_target_keys(target_key.as_str()),
                        display_name: display_name.to_string(),
                        command_template: command_template.to_string(),
                        share_target_id,
                    },
                )
            },
        );
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_update_snippet_target(
            move |snippet_row_id, target_key, display_name, command_template| {
                let Some((snippet_id, target_id)) =
                    parse_snippet_target_row_id(snippet_row_id.as_str())
                else {
                    return false;
                };
                let target_keys = parse_target_keys(target_key.as_str());
                apply_and_sync_success(
                    &weak,
                    &state,
                    Message::SyncSnippetTargetImplementationTargets {
                        snippet_id,
                        target_id,
                        target_keys,
                        display_name: display_name.to_string(),
                        command_template: command_template.to_string(),
                    },
                )
            },
        );
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_split_snippet_target(move |snippet_row_id| {
            let Some((snippet_id, target_id)) =
                parse_snippet_target_row_id(snippet_row_id.as_str())
            else {
                return false;
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::SplitSnippetTargetImplementation {
                    snippet_id,
                    target_id,
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_create_snippet_group(move |parent_node_id, name| {
            apply_and_sync_success(
                &weak,
                &state,
                Message::CreateSnippetGroup {
                    name: name.to_string(),
                    parent_id: parse_optional_snippet_group_node_id(parent_node_id.as_str()),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_rename_snippet_group(move |group_node_id, name| {
            let Some(group_id) = parse_snippet_group_node_id(group_node_id.as_str()) else {
                return false;
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::RenameSnippetGroup {
                    group_id,
                    name: name.to_string(),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_delete_snippet_node(move |node_id, node_kind| {
            let message =
                match node_kind.as_str() {
                    "Snippet" => parse_snippet_row_id(node_id.as_str())
                        .map(|snippet_id| Message::RemoveSnippet { snippet_id }),
                    "SnippetTarget" => parse_snippet_target_row_id(node_id.as_str()).map(
                        |(snippet_id, target_id)| Message::RemoveSnippetTarget {
                            snippet_id,
                            target_id,
                        },
                    ),
                    "Group" => parse_snippet_group_node_id(node_id.as_str())
                        .map(|group_id| Message::RemoveSnippetGroupRecursive { group_id }),
                    _ => None,
                };
            if let Some(message) = message {
                apply_and_sync(&weak, &state, message);
            }
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_run_snippet_on_active_host(move |snippet_row_id| {
            let message = if let Some((snippet_id, target_id)) =
                parse_snippet_target_row_id(snippet_row_id.as_str())
            {
                Message::RunSnippetTargetOnActiveHost {
                    snippet_id,
                    target_id,
                }
            } else if let Some(snippet_id) = parse_snippet_row_id(snippet_row_id.as_str()) {
                Message::RunSnippetOnActiveHost { snippet_id }
            } else {
                return;
            };
            apply_and_sync(&weak, &state, message);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_run_snippet_with_arguments(move |snippet_row_id, host_id, arguments_text| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return false;
            };
            let arguments = parse_snippet_arguments_text(arguments_text.as_str());
            let message = if let Some((snippet_id, target_id)) =
                parse_snippet_target_row_id(snippet_row_id.as_str())
            {
                Message::RunSnippetTargetWithArguments {
                    host_id,
                    snippet_id,
                    target_id,
                    arguments,
                }
            } else if let Some(snippet_id) = parse_snippet_row_id(snippet_row_id.as_str()) {
                Message::RunSnippetWithArguments {
                    host_id,
                    snippet_id,
                    arguments,
                }
            } else {
                return false;
            };
            apply_and_sync_success(&weak, &state, message)
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_move_snippet_node(move |node_id, node_kind, target_group_id| {
            let target_group_id = parse_optional_snippet_group_node_id(target_group_id.as_str());
            let message = match node_kind.as_str() {
                "Snippet" => {
                    parse_snippet_row_id(node_id.as_str()).map(|snippet_id| Message::MoveSnippet {
                        snippet_id,
                        group_id: target_group_id,
                    })
                }
                "Group" => parse_snippet_group_node_id(node_id.as_str()).map(|group_id| {
                    Message::MoveSnippetGroup {
                        group_id,
                        parent_id: target_group_id,
                    }
                }),
                _ => None,
            };
            if let Some(message) = message {
                apply_and_sync_success(&weak, &state, message)
            } else {
                false
            }
        });
    }
}

fn parse_target_keys(value: &str) -> Vec<String> {
    value
        .split([';', ',', '\n'])
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_target_keys;

    #[test]
    fn parse_target_keys_accepts_multi_select_payload() {
        assert_eq!(
            parse_target_keys("linux;debian-ubuntu;\nwindows-powershell, windows-cmd;"),
            vec![
                "linux",
                "debian-ubuntu",
                "windows-powershell",
                "windows-cmd"
            ]
        );
    }
}
