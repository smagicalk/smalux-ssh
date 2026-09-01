//! 代码片段与多层层级管理中心业务回调处理器。
//!
//! 负责多层树形展开折叠、检索过滤、沉浸式编辑保存、动态参数填报与终端命令注入。

use std::collections::HashMap;

use slint::{ComponentHandle, Model, ModelRc, VecModel};
use smagical_core::domain::snippet::{SnippetGroupRecord, SnippetRecord};
use smagical_core::domain::terminal_context::TerminalAction;
use smagical_core::event::{SnippetDeletedEvent, SnippetExecutedEvent, SnippetGroupDeletedEvent, SnippetGroupSavedEvent, SnippetSavedEvent};

use crate::generated::{AppWindow, QuickCmdData, SnippetParamFieldData};
use crate::handlers::AppContext;
use crate::snippet_tree_model::{
    build_raw_snippet_tree_from_storage, build_search_snippet_tree_nodes,
    build_snippet_group_options, build_visible_snippet_tree_nodes,
    move_and_reorder_raw_snippet_node,
};

/// 辅助函数：向当前激活终端注入执行命令并广播领域事件
fn execute_snippet_in_terminal(ctx: &AppContext, snippet_id: &str, cmd: &str, auto_execute: bool) {
    let active_pid = ctx.active_pane_id.borrow().clone();
    let groups = ctx.pane_groups.borrow();
    let active_sess_id = groups.iter()
        .find(|g| g.pane_id == active_pid)
        .or_else(|| groups.first())
        .and_then(|g| g.get_active_session())
        .map(|s| s.session_id.clone());

    if let Some(ref sess_id) = active_sess_id {
        let mut terminals = ctx.active_terminals.borrow_mut();
        if let Some(instance) = terminals.get_mut(sess_id) {
            let to_send = if auto_execute && !cmd.ends_with('\n') {
                format!("{}\n", cmd)
            } else {
                cmd.to_string()
            };
            let _ = instance.send_input(&to_send);
        }
        let action = if auto_execute {
            TerminalAction::ExecuteCommand(cmd.to_string())
        } else {
            TerminalAction::PasteText(cmd.to_string())
        };
        ctx.core_state.send_terminal_action(sess_id, action);
    }

    ctx.core_state.events().dispatch(&SnippetExecutedEvent {
        snippet_id: snippet_id.to_string(),
        session_id: active_sess_id,
        auto_execute,
    });
}

/// 同步并刷新 UI 代码片段树、父级选项与右侧伴生工具栏
pub(crate) fn sync_ui_snippets(window: &AppWindow, ctx: &AppContext) {
    let master = build_raw_snippet_tree_from_storage(ctx.core_state.storage().as_ref());
    *ctx.master_snippet_tree.borrow_mut() = master.clone();

    let search_q = ctx.snippet_search_query.borrow().clone();
    let visible_nodes = if search_q.trim().is_empty() {
        let expanded = ctx.expanded_snippet_groups.borrow();
        build_visible_snippet_tree_nodes(&master, &expanded)
    } else {
        build_search_snippet_tree_nodes(&master, &search_q)
    };

    window.set_snippet_tree_nodes(ModelRc::new(VecModel::from(visible_nodes)));

    // 分组下拉选项
    let options = build_snippet_group_options(ctx.core_state.storage().as_ref());
    window.set_snippet_parent_options(ModelRc::new(VecModel::from(options)));

    // 右侧伴生工具栏快速列表 (显示所有非分组代码片段)
    let all_snippets = ctx.core_state.storage().snippets().list_all().unwrap_or_default();
    let groups = ctx.core_state.storage().snippets().list_groups().unwrap_or_default();
    let group_map: HashMap<String, String> = groups.into_iter().map(|g| (g.id, g.name)).collect();

    let quick_list: Vec<QuickCmdData> = all_snippets
        .into_iter()
        .map(|s| {
            let cat_name = s.parent_group_id.as_deref()
                .and_then(|gid| group_map.get(gid))
                .cloned()
                .unwrap_or_default();
            QuickCmdData {
                id: s.id.into(),
                title: s.title.into(),
                command: s.content.into(),
                category: cat_name.into(),
                language: s.language.into(),
            }
        })
        .collect();

    window.set_quick_cmds(ModelRc::new(VecModel::from(quick_list)));
}

/// 注册所有代码片段相关 UI 回调
pub(crate) fn register_snippet_handlers(window: &AppWindow, ctx: &AppContext) {
    // 1. 树搜索过滤
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_filter_snippets_tree(move |query| {
            *ctx.snippet_search_query.borrow_mut() = query.to_string();
            if let Some(w) = w_handle.upgrade() {
                sync_ui_snippets(&w, &ctx);
            }
        });
    }

    // 2. 文件夹展开/折叠
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_toggle_snippet_group_expanded(move |group_id| {
            let gid = group_id.to_string();
            let is_now_exp = {
                let mut expanded = ctx.expanded_snippet_groups.borrow_mut();
                if expanded.contains(&gid) {
                    expanded.remove(&gid);
                    false
                } else {
                    expanded.insert(gid.clone());
                    true
                }
            };
            let _ = ctx.core_state.storage().snippets().set_group_expanded(&gid, is_now_exp);
            if let Some(w) = w_handle.upgrade() {
                sync_ui_snippets(&w, &ctx);
            }
        });
    }

    // 3. 选中树节点 (查看详情/编辑)
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_select_snippet_node(move |node_id, is_group| {
            if let Some(w) = w_handle.upgrade() {
                w.set_snippet_selected_id(node_id.clone());
                w.set_snippet_has_selection(true);
                w.set_snippet_is_selected_group(is_group);

                if is_group {
                    if let Ok(Some(g)) = ctx.core_state.storage().snippets().get_group_by_id(&node_id) {
                        w.set_snippet_edit_id(g.id.into());
                        w.set_snippet_edit_title(g.name.into());
                    }
                } else {
                    if let Ok(Some(s)) = ctx.core_state.storage().snippets().get_by_id(&node_id) {
                        w.set_snippet_edit_id(s.id.clone().into());
                        w.set_snippet_edit_title(s.title.clone().into());
                        w.set_snippet_edit_language(s.language.clone().into());
                        w.set_snippet_edit_content(s.content.clone().into());
                        w.set_snippet_edit_auto_execute(s.auto_execute);
                        w.set_snippet_edit_description(s.description.clone().into());
                        w.set_snippet_edit_is_favorite(s.is_favorite);
                        w.set_snippet_edit_updated_at(s.updated_at.clone().into());

                        // 计算所属文件夹名称
                        let cat_name = if let Some(ref gid) = s.parent_group_id {
                            ctx.core_state.storage().snippets().get_group_by_id(gid)
                                .ok()
                                .flatten()
                                .map(|g| g.name)
                                .unwrap_or_else(|| "根目录".to_string())
                        } else {
                            "根目录".to_string()
                        };
                        w.set_snippet_edit_category_name(cat_name.into());

                        // 识别变量占位符
                        let vars = s.extract_variables();
                        if vars.is_empty() {
                            w.set_snippet_detected_variables_text("".into());
                        } else {
                            let var_names: Vec<String> = vars.iter()
                                .map(|v| {
                                    if let Some(ref def) = v.default_value {
                                        format!("{{{{{}:{}}}}}", v.key, def)
                                    } else {
                                        format!("{{{{{}}}}}", v.key)
                                    }
                                })
                                .collect();
                            let text = format!("识别到 {} 个动态模板参数: {}", vars.len(), var_names.join(", "));
                            w.set_snippet_detected_variables_text(text.into());
                        }
                    }
                }
            }
        });
    }

    // 4. 星标置顶切换
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_toggle_snippet_favorite(move |snippet_id| {
            let id = snippet_id.to_string();
            let _ = ctx.core_state.storage().snippets().toggle_favorite(&id);
            if let Some(w) = w_handle.upgrade() {
                sync_ui_snippets(&w, &ctx);
            }
        });
    }

    // 5. 新建片段 (清空右侧表单)
    {
        let w_handle = window.as_weak();
        window.on_open_create_snippet(move || {
            if let Some(w) = w_handle.upgrade() {
                let new_id = format!("snip-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                w.set_snippet_selected_id(new_id.clone().into());
                w.set_snippet_has_selection(true);
                w.set_snippet_is_selected_group(false);
                w.set_snippet_edit_id(new_id.into());
                w.set_snippet_edit_title("未命名代码片段".into());
                w.set_snippet_edit_category_name("根目录".into());
                w.set_snippet_edit_language("bash".into());
                w.set_snippet_edit_content("#!/bin/bash\n\n".into());
                w.set_snippet_edit_auto_execute(true);
                w.set_snippet_edit_description("".into());
                w.set_snippet_edit_is_favorite(false);
                w.set_snippet_detected_variables_text("".into());
                w.set_snippet_edit_updated_at("刚刚".into());
            }
        });
    }

    // 6. 新建分组弹窗控制
    {
        let w_handle = window.as_weak();
        window.on_open_create_snippet_group(move || {
            if let Some(w) = w_handle.upgrade() {
                w.set_new_snippet_group_name("".into());
                w.set_new_snippet_group_parent_id("root".into());
                w.set_is_create_snippet_group_open(true);
            }
        });
    }
    {
        let w_handle = window.as_weak();
        window.on_cancel_create_snippet_group(move || {
            if let Some(w) = w_handle.upgrade() {
                w.set_is_create_snippet_group_open(false);
            }
        });
    }
    {
        let w_handle = window.as_weak();
        window.on_select_snippet_group_parent(move |id| {
            if let Some(w) = w_handle.upgrade() {
                w.set_new_snippet_group_parent_id(id);
            }
        });
    }
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_submit_create_snippet_group(move || {
            if let Some(w) = w_handle.upgrade() {
                let name = w.get_new_snippet_group_name().to_string();
                let parent_id = w.get_new_snippet_group_parent_id().to_string();
                if name.trim().is_empty() {
                    ctx.notify_warning("创建失败", "分组名称不能为空");
                    return;
                }

                let new_id = format!("sgrp-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                let (p_opt, level) = if parent_id == "root" || parent_id.is_empty() {
                    (None, 0)
                } else {
                    let lvl = ctx.core_state.storage().snippets().get_group_by_id(&parent_id)
                        .ok()
                        .flatten()
                        .map(|g| g.level + 1)
                        .unwrap_or(0);
                    (Some(parent_id.clone()), lvl)
                };

                let grp = SnippetGroupRecord {
                    id: new_id.clone(),
                    name: name.clone(),
                    parent_id: p_opt.clone(),
                    level,
                    is_expanded: true,
                    sort_order: 0,
                };

                if let Ok(()) = ctx.core_state.storage().snippets().save_group(&grp) {
                    ctx.expanded_snippet_groups.borrow_mut().insert(new_id.clone());
                    ctx.core_state.events().dispatch(&SnippetGroupSavedEvent {
                        group_id: new_id,
                        name,
                        parent_id: p_opt,
                        is_new: true,
                    });
                    ctx.notify_success("创建成功", "已成功新建代码片段文件夹");
                    w.set_is_create_snippet_group_open(false);
                    sync_ui_snippets(&w, &ctx);
                }
            }
        });
    }

    // 7. 保存当前编辑的代码片段
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_save_snippet_from_center(move || {
            if let Some(w) = w_handle.upgrade() {
                let id = w.get_snippet_edit_id().to_string();
                let title = w.get_snippet_edit_title().to_string();
                let lang = w.get_snippet_edit_language().to_string();
                let content = w.get_snippet_edit_content().to_string();
                let auto = w.get_snippet_edit_auto_execute();
                let desc = w.get_snippet_edit_description().to_string();
                let is_fav = w.get_snippet_edit_is_favorite();

                if title.trim().is_empty() {
                    ctx.notify_warning("保存失败", "代码片段标题不能为空");
                    return;
                }

                // 继承已有 parent_group_id 或保持 None
                let existing_parent = ctx.core_state.storage().snippets().get_by_id(&id)
                    .ok()
                    .flatten()
                    .and_then(|s| s.parent_group_id);

                let is_new = ctx.core_state.storage().snippets().get_by_id(&id).ok().flatten().is_none();

                let record = SnippetRecord {
                    id: id.clone(),
                    parent_group_id: existing_parent.clone(),
                    title: title.clone(),
                    content: content.clone(),
                    language: lang,
                    tags: Vec::new(),
                    auto_execute: auto,
                    description: desc,
                    is_favorite: is_fav,
                    sort_order: 0,
                    updated_at: "刚刚".to_string(),
                };

                if let Ok(()) = ctx.core_state.storage().snippets().save(&record) {
                    ctx.core_state.events().dispatch(&SnippetSavedEvent {
                        snippet_id: id,
                        title,
                        parent_group_id: existing_parent,
                        is_new,
                    });
                    ctx.notify_success("保存成功", "代码片段已更新并保存至库中");
                    sync_ui_snippets(&w, &ctx);
                }
            }
        });
    }

    // 8. 删除代码片段
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_delete_snippet_node(move |id| {
            let id_str = id.to_string();
            if let Ok(true) = ctx.core_state.storage().snippets().delete(&id_str) {
                ctx.core_state.events().dispatch(&SnippetDeletedEvent {
                    snippet_id: id_str,
                });
                ctx.notify_success("删除成功", "已从代码片段库中移除");
                if let Some(w) = w_handle.upgrade() {
                    w.set_snippet_has_selection(false);
                    sync_ui_snippets(&w, &ctx);
                }
            }
        });
    }

    // 9. 删除分组
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_delete_snippet_group_node(move |id| {
            let id_str = id.to_string();
            if let Ok(true) = ctx.core_state.storage().snippets().delete_group(&id_str) {
                ctx.core_state.events().dispatch(&SnippetGroupDeletedEvent {
                    group_id: id_str,
                });
                ctx.notify_success("删除成功", "文件夹已移除，包含的子项已安全回退");
                if let Some(w) = w_handle.upgrade() {
                    w.set_snippet_has_selection(false);
                    sync_ui_snippets(&w, &ctx);
                }
            }
        });
    }

    // 10. 复制片段内容
    {
        let ctx = ctx.clone();
        window.on_copy_snippet_text(move |id| {
            if let Ok(Some(s)) = ctx.core_state.storage().snippets().get_by_id(&id) {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&s.content);
                    ctx.notify_success("复制成功", format!("已复制 '{}' 脚本到剪贴板", s.title));
                }
            }
        });
    }

    // 11. 执行代码片段 (检查是否包含变量参数)
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_execute_snippet_from_center(move |id| {
            if let Ok(Some(s)) = ctx.core_state.storage().snippets().get_by_id(&id) {
                let vars = s.extract_variables();
                if vars.is_empty() {
                    // 无参数 -> 直接注入当前活跃终端
                    execute_snippet_in_terminal(&ctx, &s.id, &s.content, s.auto_execute);
                    ctx.notify_success("注入执行", format!("已向活动终端发送 '{}'", s.title));
                } else {
                    // 有参数 -> 弹出参数填报模态框
                    if let Some(w) = w_handle.upgrade() {
                        let fields: Vec<SnippetParamFieldData> = vars.iter()
                            .map(|v| SnippetParamFieldData {
                                key: v.key.clone().into(),
                                label: v.label.clone().into(),
                                value: v.default_value.clone().unwrap_or_default().into(),
                                default_val: v.default_value.clone().unwrap_or_default().into(),
                            })
                            .collect();

                        let default_params: HashMap<String, String> = vars.iter()
                            .filter_map(|v| v.default_value.as_ref().map(|d| (v.key.clone(), d.clone())))
                            .collect();
                        let rendered = s.render_content(&default_params);

                        w.set_snippet_run_modal_id(s.id.into());
                        w.set_snippet_run_modal_title(s.title.into());
                        w.set_snippet_run_params_list(ModelRc::new(VecModel::from(fields)));
                        w.set_snippet_run_modal_rendered_command(rendered.into());
                        w.set_is_snippet_run_modal_open(true);
                    }
                }
            }
        });
    }

    // 12. 参数输入修改动态渲染合成命令
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_run_snippet_param_changed(move |idx, val| {
            if let Some(w) = w_handle.upgrade() {
                let id = w.get_snippet_run_modal_id().to_string();
                if let Ok(Some(s)) = ctx.core_state.storage().snippets().get_by_id(&id) {
                    let mut params = HashMap::new();
                    let list = w.get_snippet_run_params_list();
                    let mut new_fields = Vec::new();
                    for i in 0..list.row_count() {
                        if let Some(mut item) = list.row_data(i) {
                            if i == idx as usize {
                                item.value = val.clone();
                            }
                            let v_val = if item.value.as_str().is_empty() {
                                item.default_val.to_string()
                            } else {
                                item.value.to_string()
                            };
                            params.insert(item.key.to_string(), v_val);
                            new_fields.push(item);
                        }
                    }
                    let rendered = s.render_content(&params);
                    w.set_snippet_run_params_list(ModelRc::new(VecModel::from(new_fields)));
                    w.set_snippet_run_modal_rendered_command(rendered.into());
                }
            }
        });
    }

    // 13. 取消与提交执行弹窗
    {
        let w_handle = window.as_weak();
        window.on_cancel_snippet_run(move || {
            if let Some(w) = w_handle.upgrade() {
                w.set_is_snippet_run_modal_open(false);
            }
        });
    }
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_submit_snippet_run(move || {
            if let Some(w) = w_handle.upgrade() {
                let id = w.get_snippet_run_modal_id().to_string();
                let cmd = w.get_snippet_run_modal_rendered_command().to_string();
                let auto = ctx.core_state.storage().snippets().get_by_id(&id)
                    .ok()
                    .flatten()
                    .map(|s| s.auto_execute)
                    .unwrap_or(true);

                execute_snippet_in_terminal(&ctx, &id, &cmd, auto);
                ctx.notify_success("注入执行", "参数化命令已注入活动终端");
                w.set_is_snippet_run_modal_open(false);
            }
        });
    }

    // 14. 右侧伴生工具栏搜索过滤
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_filter_quick_cmds(move |query| {
            if let Some(w) = w_handle.upgrade() {
                let q = query.trim().to_lowercase();
                let all_snippets = ctx.core_state.storage().snippets().list_all().unwrap_or_default();
                let groups = ctx.core_state.storage().snippets().list_groups().unwrap_or_default();
                let group_map: HashMap<String, String> = groups.into_iter().map(|g| (g.id, g.name)).collect();

                let filtered: Vec<QuickCmdData> = all_snippets
                    .into_iter()
                    .filter(|s| {
                        if q.is_empty() {
                            true
                        } else {
                            s.title.to_lowercase().contains(&q)
                                || s.content.to_lowercase().contains(&q)
                        }
                    })
                    .map(|s| {
                        let cat_name = s.parent_group_id.as_deref()
                            .and_then(|gid| group_map.get(gid))
                            .cloned()
                            .unwrap_or_default();
                        QuickCmdData {
                            id: s.id.into(),
                            title: s.title.into(),
                            command: s.content.into(),
                            category: cat_name.into(),
                            language: s.language.into(),
                        }
                    })
                    .collect();

                w.set_quick_cmds(ModelRc::new(VecModel::from(filtered)));
            }
        });
    }

    // -------------------------------------------------------------------------
    // 15. 代码片段树拖拽移动 / 调序
    // -------------------------------------------------------------------------
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_move_snippet_tree_node(move |src_id, target_id, drop_position| {
            if let Some(w) = w_handle.upgrade() {
                let src_str = src_id.to_string();
                let target_str = target_id.to_string();
                let pos_str = drop_position.to_string();

                let mut tree = ctx.master_snippet_tree.borrow_mut();
                match move_and_reorder_raw_snippet_node(&mut tree, &src_str, &target_str, &pos_str) {
                    Ok((src_name, target_name)) => {
                        // 如果移动到了具体分组内部，自动将该目标分组加入展开集合
                        if pos_str == "inside" && !target_str.is_empty() {
                            let mut exp = ctx.expanded_snippet_groups.borrow_mut();
                            let mut curr = target_str.clone();
                            while curr != "root" && !curr.is_empty() {
                                exp.insert(curr.clone());
                                if let Some(p) = tree.iter().find(|n| n.id == curr) {
                                    curr = p.parent_id.clone();
                                } else {
                                    break;
                                }
                            }
                        }

                        // 同步移动结果至存储层
                        if let Some(moved_node) = tree.iter().find(|n| n.id == src_str) {
                            if moved_node.is_group {
                                let new_p = if moved_node.parent_id == "root" || moved_node.parent_id.is_empty() {
                                    None
                                } else {
                                    Some(moved_node.parent_id.as_str())
                                };
                                let _ = ctx.core_state.storage().snippets().move_group(&src_str, new_p);
                            } else if let Ok(Some(mut snip)) = ctx.core_state.storage().snippets().get_by_id(&src_str) {
                                snip.parent_group_id = if moved_node.parent_id == "root" || moved_node.parent_id.is_empty() {
                                    None
                                } else {
                                    Some(moved_node.parent_id.clone())
                                };
                                let _ = ctx.core_state.storage().snippets().save(&snip);
                            }
                        }

                        drop(tree);
                        sync_ui_snippets(&w, &ctx);
                        ctx.notify_success("移动成功", format!("已将 [{}] 移动至 [{}]", src_name, target_name));
                    }
                    Err(err_msg) => {
                        tracing::warn!(target: "smagical_ui::snippets", "代码片段拖拽被阻止: {}", err_msg);
                        ctx.notify_warning("无法移动", err_msg);
                    }
                }
            }
        });
    }

    // -------------------------------------------------------------------------
    // 16. 拖拽悬停实时计算回调
    // -------------------------------------------------------------------------
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_request_snippet_drag_hover(move |src_id, target_idx, _offset_in_row| {
            if let Some(w) = w_handle.upgrade() {
                let src_str = src_id.to_string();
                let tree = ctx.master_snippet_tree.borrow();
                let q = ctx.snippet_search_query.borrow().clone();
                let visible_nodes = if q.is_empty() {
                    build_visible_snippet_tree_nodes(&tree, &ctx.expanded_snippet_groups.borrow())
                } else {
                    build_search_snippet_tree_nodes(&tree, &q)
                };

                if target_idx < 0 {
                    w.set_drop_target_id("root".into());
                    w.set_drop_position("root".into());
                    w.set_drop_target_valid(true);
                    w.set_drop_target_index(-1);
                    return;
                }

                let idx = target_idx as usize;
                if idx >= visible_nodes.len() {
                    w.set_drop_target_id("".into());
                    w.set_drop_position("none".into());
                    w.set_drop_target_valid(false);
                    w.set_drop_target_index(-1);
                    return;
                }

                let target_node = &visible_nodes[idx];
                let tgt_id = target_node.id.to_string();

                if tgt_id == src_str {
                    w.set_drop_target_id("".into());
                    w.set_drop_position("none".into());
                    w.set_drop_target_valid(false);
                    w.set_drop_target_index(-1);
                    return;
                }

                // 防止循环引用
                let mut curr = tgt_id.clone();
                let mut is_descendant = false;
                while curr != "root" && !curr.is_empty() {
                    if curr == src_str {
                        is_descendant = true;
                        break;
                    }
                    if let Some(p) = tree.iter().find(|n| n.id == curr) {
                        curr = p.parent_id.clone();
                    } else {
                        break;
                    }
                }

                if is_descendant {
                    w.set_drop_target_id("".into());
                    w.set_drop_position("none".into());
                    w.set_drop_target_valid(false);
                    w.set_drop_target_index(-1);
                    return;
                }

                w.set_drop_target_id(tgt_id.into());
                w.set_drop_target_index(target_idx);
                w.set_drop_target_valid(true);
                if target_node.is_group {
                    w.set_drop_position("inside".into());
                } else {
                    w.set_drop_position("after".into());
                }
            }
        });
    }
}
