//! 终端多会话管理、Tab 页签操作与快速启动器检索回调绑定。
//!
//! 负责响应前端终端 Tab 切换/关闭、新建会话中心搜索、终端按键输入、滚轮滚动与剪贴板等交互事件。

use std::rc::Rc;
use slint::ComponentHandle;

use crate::debug_ui::sync_ui_debug_logs;
use crate::generated::{AppWindow, HostItemData, LocalShellItemData};
use crate::handlers::AppContext;
use crate::session::sync_active_session_ui;
use crate::terminal::{encode_key_event, SplitNode, SplitOrientation, TerminalInstance};



/// 注册终端会话与启动器相关交互回调。
///
/// 将前端所有终端事件总线（包含多会话 Tab 调度、快速新建弹窗、按键转发、剪贴板交互等）与底层运行时打通。
///
/// # 参数
/// - `window`: Slint 主窗口句柄引用
/// - `ctx`: 全局应用共享上下文对象引用
pub(crate) fn register_session_handlers(window: &AppWindow, ctx: &AppContext) {
    // -------------------------------------------------------------------------
    // 1. 关闭会话 Tab 回调
    // -------------------------------------------------------------------------
    // 当用户点击某个 Tab 上的关闭按钮 (x) 或通过快捷键/右键菜单关闭会话时触发。
    // 算法逻辑：从活跃会话列表中移除目标会话，若关闭的是当前激活 Tab，则智能激活其前一个或后一个邻近 Tab。
    let window_weak = window.as_weak();
    let active_sessions_close = Rc::clone(&ctx.active_sessions);
    let active_terminals_close = Rc::clone(&ctx.active_terminals);
    window.on_close_tab(move |sess_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = sess_id.to_string();
            let mut sessions = active_sessions_close.borrow_mut();
            let cur_active = w.get_active_session_tab().to_string();

            let mut next_active = cur_active.clone();
            if let Some(idx) = sessions.iter().position(|s| s.session_id == id_str) {
                if cur_active == id_str {
                    if idx > 0 {
                        next_active = sessions[idx - 1].session_id.clone();
                    } else if idx + 1 < sessions.len() {
                        next_active = sessions[idx + 1].session_id.clone();
                    } else {
                        next_active = "".to_string();
                    }
                }
                sessions.remove(idx);
            }

            // 终止并清理底层 PTY 主会话与分屏会话实例
            let mut terminals = active_terminals_close.borrow_mut();
            if let Some(mut instance) = terminals.remove(&id_str) {
                let _ = instance.pty.kill();
            }
            let split_key = format!("{}-split", id_str);
            if let Some(mut split_instance) = terminals.remove(&split_key) {
                let _ = split_instance.pty.kill();
            }

            // 同步最新的 Tab 列表与激活会话状态到 UI 视口
            sync_active_session_ui(&w, &sessions, &next_active);
            tracing::info!(target: "smagical_ui::session", "已关闭终端会话: {}", id_str);
            sync_ui_debug_logs(&w);
        }
    });



    // -------------------------------------------------------------------------
    // 2. 切换激活 Tab 会话回调
    // -------------------------------------------------------------------------
    // 当用户在顶部 Tab 栏点击某个会话标签时触发，将视口切换为该会话的连接信息与位图流。
    let window_weak = window.as_weak();
    let active_sessions_select = Rc::clone(&ctx.active_sessions);
    window.on_select_tab(move |sess_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = sess_id.to_string();
            let sessions = active_sessions_select.borrow();
            sync_active_session_ui(&w, &sessions, &id_str);
            tracing::debug!(target: "smagical_ui::session", "切换至终端会话: {}", id_str);
        }
    });

    // -------------------------------------------------------------------------
    // 3. 呼出新建终端会话弹窗回调
    // -------------------------------------------------------------------------
    // 点击 Tab 栏右侧加号 (+) 按钮时触发，重置搜索输入框并装载当前缓存的本地 Shell 列表与全部主机资产。
    let window_weak = window.as_weak();
    let master_tree_reset = Rc::clone(&ctx.master_tree);
    let cached_shells_new_tab = Rc::clone(&ctx.cached_shells);
    window.on_new_tab(move || {
        if let Some(w) = window_weak.upgrade() {
            // 恢复本地终端完整列表 (来自启动时缓存，避免文件系统探测开销)
            w.set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from((*cached_shells_new_tab).clone()))));

            // 从当前内存全量树中提取所有主机节点
            let tree = master_tree_reset.borrow();
            let all_hosts: Vec<HostItemData> = tree
                .iter()
                .filter(|n| !n.is_group)
                .map(|n| HostItemData {
                    id: n.id.clone().into(),
                    name: n.name.clone().into(),
                    address: n.address.clone().into(),
                    port: n.port,
                    group: "".into(),
                    status: n.status.clone().into(),
                    ping_ms: n.ping_ms,
                })
                .collect();
            w.set_launcher_host_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(all_hosts))));

            tracing::info!(target: "smagical_ui::session", "呼出快速新建会话中心");
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 4. 快速新建会话中心实时关键词搜索过滤回调
    // -------------------------------------------------------------------------
    // 在新建会话弹窗输入框中键入关键词时触发，实时对本地 Shell 环境与远程主机进行多字段模糊匹配。
    let window_weak = window.as_weak();
    let master_tree_launcher = Rc::clone(&ctx.master_tree);
    let cached_shells_launcher = Rc::clone(&ctx.cached_shells);
    window.on_filter_launcher(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let q = query.trim().to_lowercase();

            // 过滤本地 Shell 项 (匹配标题、副标题、类型标签、常见别名)
            let filtered_locals: Vec<LocalShellItemData> = if q.is_empty() {
                (*cached_shells_launcher).clone()
            } else {
                cached_shells_launcher
                    .iter()
                    .filter(|s| {
                        let t = s.title.to_lowercase();
                        let sub = s.subtitle.to_lowercase();
                        let id = s.id.to_lowercase();
                        let tag = s.tag.to_lowercase();
                        t.contains(&q) || sub.contains(&q) || id.contains(&q) || tag.contains(&q)
                            || (q.contains("wsl") && (id.contains("wsl") || sub.contains("wsl")))
                            || (q.contains("ps") && (id.contains("powershell") || id.contains("pwsh")))
                            || (q.contains("bash") && (id.contains("bash") || id.contains("wsl")))
                            || (q.contains("zsh") && id.contains("zsh"))
                            || (q.contains("fish") && id.contains("fish"))
                            || (q.contains("cmd") && id.contains("cmd"))
                    })
                    .cloned()
                    .collect()
            };
            w.set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(filtered_locals))));

            // 过滤远程主机项 (匹配主机名称、IP/域名、所属分组)
            let tree = master_tree_launcher.borrow();
            let filtered_hosts: Vec<HostItemData> = tree
                .iter()
                .filter(|n| {
                    if n.is_group {
                        false
                    } else if q.is_empty() {
                        true
                    } else {
                        n.name.to_lowercase().contains(&q)
                            || n.address.to_lowercase().contains(&q)
                            || n.parent_id.to_lowercase().contains(&q)
                    }
                })
                .map(|n| HostItemData {
                    id: n.id.clone().into(),
                    name: n.name.clone().into(),
                    address: n.address.clone().into(),
                    port: n.port,
                    group: "".into(),
                    status: n.status.clone().into(),
                    ping_ms: n.ping_ms,
                })
                .collect();

            w.set_launcher_host_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(filtered_hosts))));
        }
    });

    // -------------------------------------------------------------------------
    // -------------------------------------------------------------------------
    // 5. 快捷指令片段发送回调
    // -------------------------------------------------------------------------
    // 从右侧工具栏快捷指令库中点击某条指令时触发，直接将命令文本写入当前活跃终端。
    let window_weak = window.as_weak();
    let active_terminals_snippet = Rc::clone(&ctx.active_terminals);
    window.on_send_snippet(move |cmd| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }
            let mut terminals = active_terminals_snippet.borrow_mut();
            if let Some(instance) = terminals.get_mut(&active) {
                let cmd_str = format!("{}\n", cmd);
                let _ = instance.send_input(&cmd_str);
            }
            tracing::info!(target: "smagical_ui::cmd", "向终端发送指令片段: {}", cmd);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 6. 终端按键输入捕获回调
    // -------------------------------------------------------------------------
    // 接收来自前端 FocusScope 捕获的键盘输入字符或转义序列，转发至当前激活窗格的 PTY/SSH 输入流。
    let window_weak = window.as_weak();
    let active_terminals_input = Rc::clone(&ctx.active_terminals);
    let active_pane_ids_input = Rc::clone(&ctx.active_pane_ids);
    window.on_terminal_key_input(move |text, is_ctrl, is_shift, is_alt| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }
            let target_id = if w.get_is_split() {
                active_pane_ids_input
                    .borrow()
                    .get(&active)
                    .cloned()
                    .unwrap_or_else(|| active.clone())
            } else {
                active.clone()
            };

            let mut terminals = active_terminals_input.borrow_mut();
            if let Some(instance) = terminals.get_mut(&target_id) {
                // 快捷键: Shift+PageUp 向上翻页 / Shift+PageDown 向下翻页
                if is_shift && (text == "\u{0012}" || text == "PageUp") {
                    instance.scroll_page_up();
                    return;
                }
                if is_shift && (text == "\u{0013}" || text == "PageDown") {
                    instance.scroll_page_down();
                    return;
                }

                // 正常敲击键盘输入时，自动跳回当前最新底端
                instance.scroll_to_bottom();

                let bytes = encode_key_event(text.as_str(), is_ctrl, is_shift, is_alt);
                let _ = instance.send_bytes(&bytes);
            }
            tracing::debug!(target: "smagical_ui::terminal", "终端 [{}] 接收按键: text={:?}, ctrl={}, shift={}, alt={}", target_id, text, is_ctrl, is_shift, is_alt);
        }
    });

    // -------------------------------------------------------------------------
    // 7. 终端滚轮视口滚动回调
    // -------------------------------------------------------------------------
    // 鼠标在终端主视口滑动滚轮时触发，通知终端缓冲区向上或向下翻滚历史行。
    let window_weak = window.as_weak();
    let active_terminals_scroll = Rc::clone(&ctx.active_terminals);
    let active_pane_ids_scroll = Rc::clone(&ctx.active_pane_ids);
    let scroll_accum = Rc::new(std::cell::Cell::new(0.0f32));
    window.on_terminal_scroll(move |delta| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }
            let target_id = if w.get_is_split() {
                active_pane_ids_scroll
                    .borrow()
                    .get(&active)
                    .cloned()
                    .unwrap_or_else(|| active.clone())
            } else {
                active.clone()
            };

            // 使用平滑累加器按标准滚轮刻度 (40px/行，1 个标准刻度 120px 恰好对应 3 行) 平滑消费
            let current = scroll_accum.get() + delta;
            let line_step = 40.0f32;
            let lines = (current / line_step) as i32;

            if lines != 0 {
                scroll_accum.set(current - (lines as f32) * line_step);
                let mut terminals = active_terminals_scroll.borrow_mut();
                if let Some(instance) = terminals.get_mut(&target_id) {
                    instance.scroll_delta(lines);
                }
            } else {
                scroll_accum.set(current);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 8. 终端选区复制到剪贴板回调
    // -------------------------------------------------------------------------
    // 响应快捷键 Ctrl+Shift+C 或右键菜单“复制”操作，提取当前选区纯文本并写入系统剪贴板。
    let window_weak = window.as_weak();
    let active_terminals_copy = Rc::clone(&ctx.active_terminals);
    let active_pane_ids_copy = Rc::clone(&ctx.active_pane_ids);
    window.on_terminal_copy(move || {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }
            let target_id = if w.get_is_split() {
                active_pane_ids_copy
                    .borrow()
                    .get(&active)
                    .cloned()
                    .unwrap_or_else(|| active.clone())
            } else {
                active.clone()
            };

            let mut terminals = active_terminals_copy.borrow_mut();
            if let Some(instance) = terminals.get_mut(&target_id) {
                let text = instance.parser.copy_selection_text();
                if !text.is_empty()
                    && let Ok(mut clipboard) = arboard::Clipboard::new()
                {
                    let _ = clipboard.set_text(text);
                }
            }
            tracing::info!(target: "smagical_ui::terminal", "执行终端 [{}] 选区文本复制到系统剪贴板", target_id);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 8.1 终端鼠标拖拽划选选区变更回调
    // -------------------------------------------------------------------------
    // 响应鼠标在终端视口上的拖拽选区事件，实时更新高亮坐标并在下一次重绘时着色。
    let window_weak = window.as_weak();
    let active_terminals_sel = Rc::clone(&ctx.active_terminals);
    let active_pane_ids_sel = Rc::clone(&ctx.active_pane_ids);
    window.on_terminal_selection_changed(move |sc, sr, ec, er, has_sel| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }
            let target_id = if w.get_is_split() {
                active_pane_ids_sel
                    .borrow()
                    .get(&active)
                    .cloned()
                    .unwrap_or_else(|| active.clone())
            } else {
                active.clone()
            };

            let mut terminals = active_terminals_sel.borrow_mut();
            if let Some(instance) = terminals.get_mut(&target_id) {
                if has_sel && sc >= 0 && sr >= 0 && ec >= 0 && er >= 0 {
                    instance.parser.set_selection((sc as usize, sr as usize), (ec as usize, er as usize));
                } else {
                    instance.parser.clear_selection();
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 9. 剪贴板文本粘贴到终端回调
    // -------------------------------------------------------------------------
    // 响应快捷键 Ctrl+Shift+V 或右键菜单“粘贴”操作，读取系统剪贴板并写入当前激活窗格 PTY。
    let window_weak = window.as_weak();
    let active_terminals_paste = Rc::clone(&ctx.active_terminals);
    let active_pane_ids_paste = Rc::clone(&ctx.active_pane_ids);
    window.on_terminal_paste(move || {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }
            let target_id = if w.get_is_split() {
                active_pane_ids_paste
                    .borrow()
                    .get(&active)
                    .cloned()
                    .unwrap_or_else(|| active.clone())
            } else {
                active.clone()
            };

            if let Ok(text) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                let mut terminals = active_terminals_paste.borrow_mut();
                if let Some(instance) = terminals.get_mut(&target_id) {
                    let _ = instance.send_input(&text);
                }
            }
            tracing::info!(target: "smagical_ui::terminal", "执行剪贴板粘贴到终端 [{}]", target_id);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 10. 终端清屏回调
    // -------------------------------------------------------------------------
    // 响应快捷键 Ctrl+L 或右键菜单“清屏”操作，向当前激活窗格 PTY 发送清屏控制码 (`\x1b[2J\x1b[H`)。
    let window_weak = window.as_weak();
    let active_terminals_clear = Rc::clone(&ctx.active_terminals);
    let active_pane_ids_clear = Rc::clone(&ctx.active_pane_ids);
    window.on_terminal_clear(move || {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }
            let target_id = if w.get_is_split() {
                active_pane_ids_clear
                    .borrow()
                    .get(&active)
                    .cloned()
                    .unwrap_or_else(|| active.clone())
            } else {
                active.clone()
            };

            let mut terminals = active_terminals_clear.borrow_mut();
            if let Some(instance) = terminals.get_mut(&target_id) {
                let _ = instance.clear();
            }
            tracing::info!(target: "smagical_ui::terminal", "执行终端 [{}] 清屏", target_id);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 11. 终端多窗格分屏回调 (支持无限层级任意嵌套分屏)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let active_terminals_split = Rc::clone(&ctx.active_terminals);
    let session_split_trees_split = Rc::clone(&ctx.session_split_trees);
    let active_pane_ids_split = Rc::clone(&ctx.active_pane_ids);
    let next_pane_counter = Rc::new(std::cell::Cell::new(2usize));
    window.on_split_terminal(move |orient| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }

            let split_dir = match orient.as_str() {
                "horizontal" => SplitOrientation::Horizontal,
                _ => SplitOrientation::Vertical,
            };

            let mut trees = session_split_trees_split.borrow_mut();
            let tree = trees.entry(active.clone()).or_insert_with(|| {
                SplitNode::new_single(active.clone(), format!("{} (Pane 1)", w.get_active_session_name()))
            });

            let current_target = active_pane_ids_split
                .borrow()
                .get(&active)
                .cloned()
                .unwrap_or_else(|| active.clone());

            let pane_num = next_pane_counter.get();
            next_pane_counter.set(pane_num + 1);
            let new_pane_id = format!("{}-pane-{}", active, pane_num);
            let new_pane_title = format!("{} (Pane {})", w.get_active_session_name(), pane_num);

            if tree.split_pane(&current_target, new_pane_id.clone(), new_pane_title.clone(), split_dir) {
                let mut terminals = active_terminals_split.borrow_mut();
                if let std::collections::hash_map::Entry::Vacant(e) = terminals.entry(new_pane_id.clone())
                    && let Ok(instance) = TerminalInstance::spawn_local(
                        new_pane_id.clone(),
                        "local-powershell",
                        new_pane_title,
                        80,
                        24,
                    )
                {
                    e.insert(instance);
                }

                active_pane_ids_split.borrow_mut().insert(active.clone(), new_pane_id.clone());
                w.set_is_split(true);
                w.set_split_count(tree.leaf_count() as i32);
                w.set_active_pane_id(new_pane_id.clone().into());

                tracing::info!(target: "smagical_ui::terminal", "在窗格 [{}] 上成功嵌套分屏 -> 新窗格 [{}] (总窗格数: {})", current_target, new_pane_id, tree.leaf_count());
                sync_ui_debug_logs(&w);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 12. 按窗格 ID 关闭指定分屏窗格回调 (支持递归拓扑节点合并)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let active_terminals_close_id = Rc::clone(&ctx.active_terminals);
    let session_split_trees_close_id = Rc::clone(&ctx.session_split_trees);
    let active_pane_ids_close_id = Rc::clone(&ctx.active_pane_ids);
    window.on_close_pane_by_id(move |target_pane_id| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }

            let mut trees = session_split_trees_close_id.borrow_mut();
            if let Some(tree) = trees.get_mut(&active) {
                let mut terminals = active_terminals_close_id.borrow_mut();
                if let Some(mut instance) = terminals.remove(target_pane_id.as_str()) {
                    let _ = instance.pty.kill();
                }

                tree.close_pane(target_pane_id.as_str());
                let remaining_count = tree.leaf_count();

                if remaining_count <= 1 {
                    trees.remove(&active);
                    w.set_is_split(false);
                    w.set_split_count(1);
                    active_pane_ids_close_id.borrow_mut().insert(active.clone(), active.clone());
                    w.set_active_pane_id(active.clone().into());
                } else {
                    let all_remaining = tree.all_pane_ids();
                    let next_active = all_remaining.into_iter().next().unwrap_or_else(|| active.clone());
                    active_pane_ids_close_id.borrow_mut().insert(active.clone(), next_active.clone());
                    w.set_split_count(remaining_count as i32);
                    w.set_active_pane_id(next_active.into());
                }

                tracing::info!(target: "smagical_ui::terminal", "关闭指定窗格 [{}] (剩余窗格数: {})", target_pane_id, remaining_count);
                sync_ui_debug_logs(&w);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 12.1 按窗格索引关闭分屏窗格回调 (向后兼容)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let session_split_trees_close_idx = Rc::clone(&ctx.session_split_trees);
    let active_terminals_close_idx = Rc::clone(&ctx.active_terminals);
    let active_pane_ids_close_idx = Rc::clone(&ctx.active_pane_ids);
    window.on_close_pane_by_index(move |idx| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if active.is_empty() {
                return;
            }

            let mut trees = session_split_trees_close_idx.borrow_mut();
            if let Some(tree) = trees.get_mut(&active) {
                let pane_ids = tree.all_pane_ids();
                if let Some(target_pane_id) = pane_ids.get(idx as usize) {
                    let target_id = target_pane_id.clone();
                    let mut terminals = active_terminals_close_idx.borrow_mut();
                    if let Some(mut instance) = terminals.remove(&target_id) {
                        let _ = instance.pty.kill();
                    }

                    tree.close_pane(&target_id);
                    let remaining_count = tree.leaf_count();

                    if remaining_count <= 1 {
                        trees.remove(&active);
                        w.set_is_split(false);
                        w.set_split_count(1);
                        active_pane_ids_close_idx.borrow_mut().insert(active.clone(), active.clone());
                        w.set_active_pane_id(active.clone().into());
                    } else {
                        let all_remaining = tree.all_pane_ids();
                        let next_active = all_remaining.into_iter().next().unwrap_or_else(|| active.clone());
                        active_pane_ids_close_idx.borrow_mut().insert(active.clone(), next_active.clone());
                        w.set_split_count(remaining_count as i32);
                        w.set_active_pane_id(next_active.into());
                    }
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 12.2 关闭当前终端全部嵌套分屏模式
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let active_terminals_close_split = Rc::clone(&ctx.active_terminals);
    let session_split_trees_close_split = Rc::clone(&ctx.session_split_trees);
    let active_pane_ids_close_split = Rc::clone(&ctx.active_pane_ids);
    window.on_close_split(move || {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            w.set_is_split(false);
            w.set_split_count(1);
            w.set_active_pane_index(0);
            w.set_active_pane_id(active.clone().into());
            active_pane_ids_close_split.borrow_mut().insert(active.clone(), active.clone());

            if !active.is_empty() {
                let mut trees = session_split_trees_close_split.borrow_mut();
                if let Some(tree) = trees.remove(&active) {
                    let pane_ids = tree.all_pane_ids();
                    let mut terminals = active_terminals_close_split.borrow_mut();
                    for pid in pane_ids {
                        if pid != active
                            && let Some(mut instance) = terminals.remove(&pid)
                        {
                            let _ = instance.pty.kill();
                        }
                    }
                }
            }
            tracing::info!(target: "smagical_ui::terminal", "终端 [{}] 全部退出分屏模式", active);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 13. 选择并聚焦指定分屏窗格回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let active_pane_ids_sel_id = Rc::clone(&ctx.active_pane_ids);
    window.on_select_pane_by_id(move |pane_id| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if !active.is_empty() {
                active_pane_ids_sel_id.borrow_mut().insert(active, pane_id.to_string());
                w.set_active_pane_id(pane_id);
            }
        }
    });

    let window_weak = window.as_weak();
    window.on_select_pane(move |idx| {
        if let Some(w) = window_weak.upgrade() {
            w.set_active_pane_index(idx);
        }
    });

    // -------------------------------------------------------------------------
    // 13.1 动态拖拽调节分割条比例回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let session_split_trees_adjust = Rc::clone(&ctx.session_split_trees);
    window.on_adjust_splitter(move |splitter_id, delta_ratio| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if !active.is_empty() {
                let mut trees = session_split_trees_adjust.borrow_mut();
                if let Some(tree) = trees.get_mut(&active) {
                    let res = tree.adjust_splitter(splitter_id.as_str(), delta_ratio);
                    tracing::info!(target: "smagical_ui::terminal", "调整分割条 [{}] delta={:.4} -> 结果: {}", splitter_id, delta_ratio, res);
                }
            }
        }
    });


    // -------------------------------------------------------------------------
    // 14. 终端内文本查找回调
    // -------------------------------------------------------------------------
    // 响应快捷键 Ctrl+F 浮层中输入的关键词，在终端历史回滚缓冲区中检索匹配项。
    let window_weak = window.as_weak();
    window.on_search_terminal(move |query, match_case| {
        if let Some(w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_ui::terminal", "终端查找文本: {:?} (大小写敏感: {})", query, match_case);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 15. 终端动态几何网格尺寸调节回调 (Resize)
    // -------------------------------------------------------------------------
    // 当前端窗口或抽屉宽度拉伸导致终端视口变化时，自适应重置行列数并下发至 PTY。
    let window_weak = window.as_weak();
    let active_terminals_resize = Rc::clone(&ctx.active_terminals);
    window.on_terminal_resize(move |cols, rows| {
        if let Some(w) = window_weak.upgrade() {
            let active = w.get_active_session_tab().to_string();
            if !active.is_empty() {
                let mut terminals = active_terminals_resize.borrow_mut();
                if let Some(instance) = terminals.get_mut(&active) {
                    let _ = instance.resize(cols as u16, rows as u16);
                }
            }
        }
    });
}





