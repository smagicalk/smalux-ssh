//! 双盘文件管理与 SFTP 远程传输 UI 交互处理器。
//!
//! 负责本地与远程文件系统目录遍历、独立双栏 Tab 调度、路径导航与文件上传/下载任务流转。

use std::path::PathBuf;
use std::rc::Rc;
use slint::ComponentHandle;
use smagical_core::event::{
    FileOperationBeforeEvent, FileOperationCompletedEvent, FileTabClosedEvent,
    FileTabFocusChangedEvent, FileTabNavigatedEvent, FileTabOpenedEvent,
    FileTabOpeningEvent, FileTransferStartedEvent,
};
use smagical_core::{
    generate_mock_remote_directory, scan_local_directory, FileItemData,
    LocalFileTabSession, RemoteFileTabSession, TransferDirection, TransferStatus, TransferTask,
};

use crate::generated::{
    AppWindow, FileItemData as SlintFileItemData, FileTabData as SlintFileTabData,
    HostItemData as SlintHostItemData, TransferItemData as SlintTransferItemData,
};
use crate::handlers::AppContext;

/// 将核心层 `TransferTask` 转换为 Slint UI 传输数据项 (支持单文件与文件夹树形展开)
pub(crate) fn map_transfer_task_to_ui(t: &TransferTask) -> SlintTransferItemData {
    SlintTransferItemData {
        id: t.id.clone().into(),
        parent_id: t.parent_id.clone().unwrap_or_default().into(),
        filename: t.filename.clone().into(),
        source_path: t.source_path.clone().into(),
        target_path: t.target_path.clone().into(),
        is_dir: t.is_dir,
        is_expanded: t.is_expanded,
        level: t.level,
        item_count_text: t.item_count_text.clone().into(),
        direction: t.direction.to_string().into(),
        progress: t.progress(),
        speed_text: t.speed_formatted().into(),
        status: t.status.to_string().into(),
        size_text: format!(
            "{} / {}",
            smagical_core::format_file_size(t.transferred_bytes),
            smagical_core::format_file_size(t.total_bytes)
        ).into(),
    }
}




/// 构造文件会话选择弹窗的主机列表 (支持实时过滤)
pub(crate) fn build_file_launcher_hosts(ctx: &AppContext, query: &str) -> Vec<SlintHostItemData> {
    let q = query.trim().to_lowercase();
    let tree = ctx.master_tree.borrow();
    tree.iter()
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
        .map(|n| SlintHostItemData {
            id: n.id.clone().into(),
            name: n.name.clone().into(),
            address: n.address.clone().into(),
            port: n.port,
            group: n.parent_id.clone().into(),
            status: n.status.clone().into(),
            ping_ms: n.ping_ms,
        })
        .collect()
}

/// 将核心层 `FileItemData` 转换为 Slint UI 数据项
pub(crate) fn map_file_item_to_ui(item: &FileItemData) -> SlintFileItemData {
    SlintFileItemData {
        id: item.id.clone().into(),
        name: item.name.clone().into(),
        path: item.path.clone().into(),
        is_dir: item.is_dir,
        size_formatted: item.size_formatted.clone().into(),
        modified_formatted: item.modified_formatted.clone().into(),
        permissions: item.permissions.clone().into(),
        is_expanded: item.is_expanded,
        level: item.level,
    }
}


/// 仅同步左侧本地 Tab 列表 (用于拖拽重排等无需全量扫描的轻量操作)
pub(crate) fn sync_local_tabs_only(window: &AppWindow, ctx: &AppContext) {
    let local_tabs = ctx.local_tabs.borrow();
    let active_local_id = ctx.active_local_tab_id.borrow().clone();
    let ui_local_tabs: Vec<SlintFileTabData> = local_tabs
        .iter()
        .map(|t| SlintFileTabData {
            id: t.tab_id.clone().into(),
            host_id: "local".into(),
            title: t.title.clone().into(),
            subtitle: t.current_path.clone().into(),
            status: "online".into(),
            is_active: t.tab_id == active_local_id,
        })
        .collect();
    window.set_local_tabs(slint::ModelRc::from(Rc::new(slint::VecModel::from(ui_local_tabs))));
    window.set_active_local_tab_id(active_local_id.into());
}

/// 仅同步右侧远程 Tab 列表 (用于拖拽重排等无需全量扫描的轻量操作)
pub(crate) fn sync_remote_tabs_only(window: &AppWindow, ctx: &AppContext) {
    let remote_tabs = ctx.remote_tabs.borrow();
    let active_remote_id = ctx.active_remote_tab_id.borrow().clone();
    let ui_remote_tabs: Vec<SlintFileTabData> = remote_tabs
        .iter()
        .map(|t| SlintFileTabData {
            id: t.tab_id.clone().into(),
            host_id: t.host_id.clone().into(),
            title: t.host_name.clone().into(),
            subtitle: t.host_address.clone().into(),
            status: t.status.clone().into(),
            is_active: t.tab_id == active_remote_id,
        })
        .collect();
    window.set_remote_tabs(slint::ModelRc::from(Rc::new(slint::VecModel::from(ui_remote_tabs))));
    window.set_active_remote_tab_id(active_remote_id.into());
}

/// 同步当前激活文件会话的双盘数据到 Slint UI
pub(crate) fn sync_file_explorer_ui(window: &AppWindow, ctx: &AppContext) {
    // 1. 同步左侧本地 Tab 列表
    sync_local_tabs_only(window, ctx);

    // 2. 同步右侧远程 Tab 列表
    sync_remote_tabs_only(window, ctx);

    // 3. 同步当前路径
    let local_path = ctx.local_current_path.borrow().clone();
    let remote_path = ctx.remote_current_path.borrow().clone();
    window.set_local_current_path(local_path.into());
    window.set_remote_current_path(remote_path.into());

    // 4. 同步文件列表
    let local_items: Vec<SlintFileItemData> = ctx
        .local_file_nodes
        .borrow()
        .iter()
        .map(map_file_item_to_ui)
        .collect();
    let remote_items: Vec<SlintFileItemData> = ctx
        .remote_file_nodes
        .borrow()
        .iter()
        .map(map_file_item_to_ui)
        .collect();

    window.set_local_files(slint::ModelRc::from(Rc::new(slint::VecModel::from(local_items))));
    window.set_remote_files(slint::ModelRc::from(Rc::new(slint::VecModel::from(remote_items))));

    // 5. 同步历史导航前进/后退使能状态
    let local_can_back = {
        let tabs = ctx.local_tabs.borrow();
        let act_id = ctx.active_local_tab_id.borrow();
        tabs.iter().find(|t| t.tab_id == *act_id).map(|t| t.can_go_back()).unwrap_or(false)
    };
    let local_can_fwd = {
        let tabs = ctx.local_tabs.borrow();
        let act_id = ctx.active_local_tab_id.borrow();
        tabs.iter().find(|t| t.tab_id == *act_id).map(|t| t.can_go_forward()).unwrap_or(false)
    };
    let remote_can_back = {
        let tabs = ctx.remote_tabs.borrow();
        let act_id = ctx.active_remote_tab_id.borrow();
        tabs.iter().find(|t| t.tab_id == *act_id).map(|t| t.can_go_back()).unwrap_or(false)
    };
    let remote_can_fwd = {
        let tabs = ctx.remote_tabs.borrow();
        let act_id = ctx.active_remote_tab_id.borrow();
        tabs.iter().find(|t| t.tab_id == *act_id).map(|t| t.can_go_forward()).unwrap_or(false)
    };

    window.set_local_can_go_back(local_can_back);
    window.set_local_can_go_forward(local_can_fwd);
    window.set_remote_can_go_back(remote_can_back);
    window.set_remote_can_go_forward(remote_can_fwd);

    // 6. 同步文件选择弹窗主机列表
    let file_hosts = build_file_launcher_hosts(ctx, "");
    window.set_file_launcher_host_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(file_hosts))));

    // 7. 同步实时传输任务列表 (支持文件夹树折叠过滤)
    let all_tasks = ctx.transfer_tasks.borrow();
    let collapsed_parents: std::collections::HashSet<String> = all_tasks
        .iter()
        .filter(|t| t.is_dir && !t.is_expanded)
        .map(|t| t.id.clone())
        .collect();

    let tasks: Vec<SlintTransferItemData> = all_tasks
        .iter()
        .filter(|t| {
            if let Some(pid) = &t.parent_id {
                !collapsed_parents.contains(pid)
            } else {
                true
            }
        })
        .map(map_transfer_task_to_ui)
        .collect();
    window.set_transfer_tasks(slint::ModelRc::from(Rc::new(slint::VecModel::from(tasks))));
}





/// 扫描并更新本地文件列表 (带错误校验与历史记录)
pub(crate) fn try_refresh_local_path(ctx: &AppContext, new_path: &str, push_history: bool) -> Result<(), String> {
    let target = if new_path == "~" || new_path.is_empty() {
        directories::BaseDirs::new()
            .map(|p| p.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/"))
    } else {
        PathBuf::from(new_path)
    };

    if !target.exists() {
        return Err(format!("本地路径不存在: {}", target.display()));
    }

    if !target.is_dir() {
        return Err(format!("指定路径不是有效文件夹: {}", target.display()));
    }

    match scan_local_directory(&target) {
        Ok(files) => {
            let resolved_path = target.to_string_lossy().to_string();
            *ctx.local_current_path.borrow_mut() = resolved_path.clone();
            *ctx.local_file_nodes.borrow_mut() = files;

            // 同步更新当前激活 Tab 的 current_path 与历史栈
            let act_id = ctx.active_local_tab_id.borrow().clone();
            let mut tabs = ctx.local_tabs.borrow_mut();
            if let Some(tab) = tabs.iter_mut().find(|t| t.tab_id == act_id) {
                if push_history {
                    tab.push_path(resolved_path);
                } else {
                    tab.current_path = resolved_path;
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("无法读取目录 [{}]: {}", target.display(), e)),
    }
}

/// 扫描并更新本地文件列表 (忽略错误并记录历史)
pub(crate) fn refresh_local_path(ctx: &AppContext, new_path: &str) {
    let _ = try_refresh_local_path(ctx, new_path, true);
}

/// 扫描并更新右栏文件列表 (带错误校验与历史记录，自适应本地与远程会话)
pub(crate) fn try_refresh_remote_path(ctx: &AppContext, new_path: &str, push_history: bool) -> Result<(), String> {
    let act_id = ctx.active_remote_tab_id.borrow().clone();
    let is_local_session = {
        let tabs = ctx.remote_tabs.borrow();
        tabs.iter().find(|t| t.tab_id == act_id).map(|t| t.host_id == "local").unwrap_or(false)
    };

    if is_local_session {
        // 右栏当前激活会话为本地目录
        let target = if new_path == "~" || new_path.is_empty() {
            directories::BaseDirs::new()
                .map(|p| p.home_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from("/"))
        } else {
            PathBuf::from(new_path)
        };

        if !target.exists() {
            return Err(format!("本地路径不存在: {}", target.display()));
        }
        if !target.is_dir() {
            return Err(format!("指定路径不是有效文件夹: {}", target.display()));
        }

        match scan_local_directory(&target) {
            Ok(files) => {
                let resolved_path = target.to_string_lossy().to_string();
                *ctx.remote_current_path.borrow_mut() = resolved_path.clone();
                *ctx.remote_file_nodes.borrow_mut() = files;

                let mut tabs = ctx.remote_tabs.borrow_mut();
                if let Some(tab) = tabs.iter_mut().find(|t| t.tab_id == act_id) {
                    if push_history {
                        tab.push_path(resolved_path);
                    } else {
                        tab.current_path = resolved_path;
                    }
                }
                Ok(())
            }
            Err(e) => Err(format!("无法读取目录 [{}]: {}", target.display(), e)),
        }
    } else {
        // 右栏当前激活会话为远程 SFTP
        let clean_path = if new_path == "~" || new_path.is_empty() { "/root" } else { new_path };
        if !clean_path.starts_with('/') {
            return Err(format!("远程路径必须以绝对路径 '/' 开头: {}", clean_path));
        }

        *ctx.remote_current_path.borrow_mut() = clean_path.to_string();
        let files = generate_mock_remote_directory(clean_path);
        *ctx.remote_file_nodes.borrow_mut() = files;

        let mut tabs = ctx.remote_tabs.borrow_mut();
        if let Some(tab) = tabs.iter_mut().find(|t| t.tab_id == act_id) {
            if push_history {
                tab.push_path(clean_path.to_string());
            } else {
                tab.current_path = clean_path.to_string();
            }
        }
        Ok(())
    }
}

/// 扫描并更新右栏文件列表 (忽略错误并记录历史)
pub(crate) fn refresh_remote_path(ctx: &AppContext, new_path: &str) {
    let _ = try_refresh_remote_path(ctx, new_path, true);
}




/// 注册双盘文件管理与 SFTP 视图回调
pub(crate) fn register_file_handlers(window: &AppWindow, ctx: &AppContext) {
    // -------------------------------------------------------------------------
    // 1. 左侧本地 Tab 栏交互回调
    // -------------------------------------------------------------------------
    // 1.1 选择本地 Tab
    let window_weak = window.as_weak();
    let ctx_select_loc = ctx.clone();
    window.on_select_local_tab(move |tab_id| {
        if let Some(w) = window_weak.upgrade() {
            let tid = tab_id.to_string();
            *ctx_select_loc.active_local_tab_id.borrow_mut() = tid.clone();

            let tabs = ctx_select_loc.local_tabs.borrow();
            let loc = if let Some(tab) = tabs.iter().find(|t| t.tab_id == tid) {
                tab.current_path.clone()
            } else {
                String::new()
            };
            drop(tabs);
            if !loc.is_empty() {
                let _ = try_refresh_local_path(&ctx_select_loc, &loc, false);
            }

            ctx_select_loc.core_state.events().dispatch(&FileTabFocusChangedEvent {
                tab_id: Some(tid.clone()),
                is_remote: false,
                current_path: loc.clone(),
            });
            sync_file_explorer_ui(&w, &ctx_select_loc);
            tracing::info!(target: "smagical_ui::files", "切换至本地文件 Tab: {}", tid);
        }
    });

    // 1.2 关闭本地 Tab (若全部关闭则自动新建 1 个默认 Tab 保底)
    let window_weak = window.as_weak();
    let ctx_close_loc = ctx.clone();
    window.on_close_local_tab(move |tab_id| {
        if let Some(w) = window_weak.upgrade() {
            let tid = tab_id.to_string();
            let mut tabs = ctx_close_loc.local_tabs.borrow_mut();
            let mut act_id = ctx_close_loc.active_local_tab_id.borrow_mut();

            if let Some(pos) = tabs.iter().position(|t| t.tab_id == tid) {
                tabs.remove(pos);
                if *act_id == tid {
                    if !tabs.is_empty() {
                        let next_pos = if pos > 0 { pos - 1 } else { 0 };
                        *act_id = tabs[next_pos.min(tabs.len() - 1)].tab_id.clone();
                    } else {
                        // 保底创建一个默认主目录 Tab
                        let home_dir = directories::BaseDirs::new()
                            .map(|p| p.home_dir().to_string_lossy().to_string())
                            .unwrap_or_else(|| "/".to_string());
                        let fallback = LocalFileTabSession::new("ltab-1", "本地 (主目录)", home_dir);
                        tabs.push(fallback);
                        *act_id = "ltab-1".to_string();
                    }
                }
            }

            let next_act_id = act_id.clone();
            let next_tab = tabs.iter().find(|t| t.tab_id == next_act_id).cloned();
            drop(tabs);
            drop(act_id);

            ctx_close_loc.core_state.events().dispatch(&FileTabClosedEvent {
                tab_id: tid.clone(),
            });

            if let Some(t) = next_tab {
                refresh_local_path(&ctx_close_loc, &t.current_path);
            }

            sync_file_explorer_ui(&w, &ctx_close_loc);
            tracing::info!(target: "smagical_ui::files", "关闭本地文件 Tab: {}", tid);
        }
    });

    // 1.3 左侧 + 按钮：新建本地目录 Tab
    let window_weak = window.as_weak();
    let ctx_new_loc = ctx.clone();
    window.on_new_local_tab(move || {
        if let Some(w) = window_weak.upgrade() {
            let mut tabs = ctx_new_loc.local_tabs.borrow_mut();
            let new_idx = tabs.len() + 1;
            let tab_id = format!("ltab-{}", new_idx);
            let home_dir = directories::BaseDirs::new()
                .map(|p| p.home_dir().to_string_lossy().to_string())
                .unwrap_or_else(|| "/".to_string());
            let session = LocalFileTabSession::new(
                tab_id.clone(),
                format!("本地 #{}", new_idx),
                home_dir.clone(),
            );
            tabs.push(session);
            *ctx_new_loc.active_local_tab_id.borrow_mut() = tab_id.clone();
            drop(tabs);

            ctx_new_loc.core_state.events().dispatch(&FileTabOpenedEvent {
                tab_id: tab_id.clone(),
                host_id: "local".into(),
                path: home_dir.clone(),
            });
            refresh_local_path(&ctx_new_loc, &home_dir);
            sync_file_explorer_ui(&w, &ctx_new_loc);
            tracing::info!(target: "smagical_ui::files", "新建本地文件 Tab: {}", tab_id);
        }
    });

    // -------------------------------------------------------------------------
    // 2. 右侧远程 Tab 栏交互回调
    // -------------------------------------------------------------------------
    // 2.1 选择远程 Tab
    let window_weak = window.as_weak();
    let ctx_select_rem = ctx.clone();
    window.on_select_remote_tab(move |tab_id| {
        if let Some(w) = window_weak.upgrade() {
            let tid = tab_id.to_string();
            *ctx_select_rem.active_remote_tab_id.borrow_mut() = tid.clone();

            let tabs = ctx_select_rem.remote_tabs.borrow();
            let rem = if let Some(tab) = tabs.iter().find(|t| t.tab_id == tid) {
                tab.current_path.clone()
            } else {
                String::new()
            };
            drop(tabs);
            if !rem.is_empty() {
                let _ = try_refresh_remote_path(&ctx_select_rem, &rem, false);
            }

            ctx_select_rem.core_state.events().dispatch(&FileTabFocusChangedEvent {
                tab_id: Some(tid.clone()),
                is_remote: true,
                current_path: rem.clone(),
            });
            sync_file_explorer_ui(&w, &ctx_select_rem);
            tracing::info!(target: "smagical_ui::files", "切换至远程 SFTP Tab: {}", tid);
        }
    });

    // 2.2 关闭远程 Tab (若全部关闭则进入优雅空状态)
    let window_weak = window.as_weak();
    let ctx_close_rem = ctx.clone();
    window.on_close_remote_tab(move |tab_id| {
        if let Some(w) = window_weak.upgrade() {
            let tid = tab_id.to_string();
            let mut tabs = ctx_close_rem.remote_tabs.borrow_mut();
            let mut act_id = ctx_close_rem.active_remote_tab_id.borrow_mut();

            if let Some(pos) = tabs.iter().position(|t| t.tab_id == tid) {
                tabs.remove(pos);
                if *act_id == tid {
                    if !tabs.is_empty() {
                        let next_pos = if pos > 0 { pos - 1 } else { 0 };
                        *act_id = tabs[next_pos.min(tabs.len() - 1)].tab_id.clone();
                    } else {
                        *act_id = String::new();
                    }
                }
            }

            let next_act_id = act_id.clone();
            let next_tab = tabs.iter().find(|t| t.tab_id == next_act_id).cloned();
            drop(tabs);
            drop(act_id);

            ctx_close_rem.core_state.events().dispatch(&FileTabClosedEvent {
                tab_id: tid.clone(),
            });

            if let Some(t) = next_tab {
                refresh_remote_path(&ctx_close_rem, &t.current_path);
            } else {
                ctx_close_rem.remote_file_nodes.borrow_mut().clear();
                *ctx_close_rem.remote_current_path.borrow_mut() = String::new();
            }

            sync_file_explorer_ui(&w, &ctx_close_rem);
            tracing::info!(target: "smagical_ui::files", "关闭远程 SFTP Tab: {}", tid);
        }
    });

    // 2.3 右侧 + 按钮 / 空白页连接按钮：打开文件会话选择弹窗 (FileHostModal)
    let window_weak = window.as_weak();
    let ctx_new_rem = ctx.clone();
    window.on_new_remote_tab(move || {
        if let Some(w) = window_weak.upgrade() {
            let hosts = build_file_launcher_hosts(&ctx_new_rem, "");
            w.set_file_launcher_host_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(hosts))));
            w.set_is_file_host_modal_open(true);
            tracing::info!(target: "smagical_ui::files", "打开文件会话选择与 SFTP 连接弹窗");
        }
    });

    // -------------------------------------------------------------------------
    // 3. 从左侧主机栏双击主机：只会创建到右侧远程 Tab 栏
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let ctx_open_host = ctx.clone();
    window.on_open_host_files(move |host_id| {
        if let Some(w) = window_weak.upgrade() {
            let h_id = host_id.to_string();

            // 事件总线前置安全审查
            let open_event = FileTabOpeningEvent::new(&h_id, "/root");
            ctx_open_host.core_state.events().dispatch(&open_event);
            if open_event.is_aborted() {
                ctx_open_host.notify_warning("连接已拦截", open_event.abort_reason().unwrap_or_default());
                return;
            }

            let mut tabs = ctx_open_host.remote_tabs.borrow_mut();

            // 如果该主机已有打开的远程 Tab，则直接激活它
            if let Some(existing) = tabs.iter().find(|t| t.host_id == h_id) {
                let tid = existing.tab_id.clone();
                let rem = existing.current_path.clone();
                *ctx_open_host.active_remote_tab_id.borrow_mut() = tid.clone();
                drop(tabs);

                ctx_open_host.core_state.events().dispatch(&FileTabFocusChangedEvent {
                    tab_id: Some(tid.clone()),
                    is_remote: true,
                    current_path: rem.clone(),
                });
                refresh_remote_path(&ctx_open_host, &rem);
                sync_file_explorer_ui(&w, &ctx_open_host);
                tracing::info!(target: "smagical_ui::files", "激活已存在的右侧远程 Tab: {}", tid);
                return;
            }

            // 查询主机元数据并新建右侧远程 Tab
            let (h_name, h_addr) = if let Ok(Some(h)) = ctx_open_host.core_state.storage().hosts().get_by_id(&h_id) {
                (h.name, format!("{}:{}", h.address, h.port))
            } else {
                (format!("Host ({})", h_id), "127.0.0.1:22".into())
            };

            let tab_id = format!("rtab-{}", tabs.len() + 1);
            let session = RemoteFileTabSession::new(tab_id.clone(), h_id.clone(), h_name, h_addr, "/root");
            tabs.push(session);
            *ctx_open_host.active_remote_tab_id.borrow_mut() = tab_id.clone();
            drop(tabs);

            ctx_open_host.core_state.events().dispatch(&FileTabOpenedEvent {
                tab_id: tab_id.clone(),
                host_id: h_id.clone(),
                path: "/root".into(),
            });
            refresh_remote_path(&ctx_open_host, "/root");
            sync_file_explorer_ui(&w, &ctx_open_host);
            tracing::info!(target: "smagical_ui::files", "双击主机创建右侧远程 SFTP Tab: {}", tab_id);
        }
    });

    // -------------------------------------------------------------------------
    // 4. 路径导航与目录文件交互
    // -------------------------------------------------------------------------
    // 4.1 本地路径导航 (支持回车直达与不存在气泡通知提示)
    let window_weak = window.as_weak();
    let ctx_nav_local = ctx.clone();
    window.on_navigate_local_path(move |path| {
        if let Some(w) = window_weak.upgrade() {
            let p_str = path.to_string();
            let old_p = ctx_nav_local.local_current_path.borrow().clone();
            match try_refresh_local_path(&ctx_nav_local, &p_str, true) {
                Ok(_) => {
                    let act_id = ctx_nav_local.active_local_tab_id.borrow().clone();
                    ctx_nav_local.core_state.events().dispatch(&FileTabNavigatedEvent {
                        tab_id: act_id.clone(),
                        is_remote: false,
                        old_path: old_p.clone(),
                        new_path: p_str.clone(),
                    });
                    sync_file_explorer_ui(&w, &ctx_nav_local);
                    tracing::info!(target: "smagical_ui::files", "成功跳转本地路径: {}", p_str);
                }
                Err(err) => {
                    ctx_nav_local.notify_error("路径不存在", err.clone());
                    let current_valid = ctx_nav_local.local_current_path.borrow().clone();
                    w.set_local_current_path(current_valid.into());
                    tracing::warn!(target: "smagical_ui::files", "本地路径跳转失败: {}", p_str);
                }
            }
        }
    });

    // 4.2 本地历史后退
    let window_weak = window.as_weak();
    let ctx_back_loc = ctx.clone();
    window.on_navigate_local_back(move || {
        if let Some(w) = window_weak.upgrade() {
            let act_id = ctx_back_loc.active_local_tab_id.borrow().clone();
            let prev_path = {
                let mut tabs = ctx_back_loc.local_tabs.borrow_mut();
                tabs.iter_mut().find(|t| t.tab_id == act_id).and_then(|t| t.go_back())
            };
            if let Some(prev) = prev_path {
                let old_p = ctx_back_loc.local_current_path.borrow().clone();
                let _ = try_refresh_local_path(&ctx_back_loc, &prev, false);
                ctx_back_loc.core_state.events().dispatch(&FileTabNavigatedEvent {
                    tab_id: act_id.clone(),
                    is_remote: false,
                    old_path: old_p.clone(),
                    new_path: prev.clone(),
                });
                sync_file_explorer_ui(&w, &ctx_back_loc);
                tracing::info!(target: "smagical_ui::files", "本地后退至路径: {}", prev);
            }
        }
    });

    // 4.3 本地历史前进
    let window_weak = window.as_weak();
    let ctx_fwd_loc = ctx.clone();
    window.on_navigate_local_forward(move || {
        if let Some(w) = window_weak.upgrade() {
            let act_id = ctx_fwd_loc.active_local_tab_id.borrow().clone();
            let next_path = {
                let mut tabs = ctx_fwd_loc.local_tabs.borrow_mut();
                tabs.iter_mut().find(|t| t.tab_id == act_id).and_then(|t| t.go_forward())
            };
            if let Some(next) = next_path {
                let old_p = ctx_fwd_loc.local_current_path.borrow().clone();
                let _ = try_refresh_local_path(&ctx_fwd_loc, &next, false);
                ctx_fwd_loc.core_state.events().dispatch(&FileTabNavigatedEvent {
                    tab_id: act_id.clone(),
                    is_remote: false,
                    old_path: old_p.clone(),
                    new_path: next.clone(),
                });
                sync_file_explorer_ui(&w, &ctx_fwd_loc);
                tracing::info!(target: "smagical_ui::files", "本地前进至路径: {}", next);
            }
        }
    });

    // 4.4 本地返回上一级
    let window_weak = window.as_weak();
    let ctx_up_loc = ctx.clone();
    window.on_navigate_local_up(move || {
        if let Some(w) = window_weak.upgrade() {
            let current = ctx_up_loc.local_current_path.borrow().clone();
            let p = std::path::PathBuf::from(&current);
            if let Some(parent) = p.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                if !parent_str.is_empty() {
                    let act_id = ctx_up_loc.active_local_tab_id.borrow().clone();
                    let _ = try_refresh_local_path(&ctx_up_loc, &parent_str, true);
                    ctx_up_loc.core_state.events().dispatch(&FileTabNavigatedEvent {
                        tab_id: act_id.clone(),
                        is_remote: false,
                        old_path: current.clone(),
                        new_path: parent_str.clone(),
                    });
                    sync_file_explorer_ui(&w, &ctx_up_loc);
                    tracing::info!(target: "smagical_ui::files", "本地向上进入目录: {}", parent_str);
                }
            }
        }
    });

    // 4.5 远程路径导航 (支持回车直达与格式/路径气泡通知校验)
    let window_weak = window.as_weak();
    let ctx_nav_remote = ctx.clone();
    window.on_navigate_remote_path(move |path| {
        if let Some(w) = window_weak.upgrade() {
            let p_str = path.to_string();
            let old_p = ctx_nav_remote.remote_current_path.borrow().clone();
            match try_refresh_remote_path(&ctx_nav_remote, &p_str, true) {
                Ok(_) => {
                    let act_id = ctx_nav_remote.active_remote_tab_id.borrow().clone();
                    ctx_nav_remote.core_state.events().dispatch(&FileTabNavigatedEvent {
                        tab_id: act_id.clone(),
                        is_remote: true,
                        old_path: old_p.clone(),
                        new_path: p_str.clone(),
                    });
                    sync_file_explorer_ui(&w, &ctx_nav_remote);
                    tracing::info!(target: "smagical_ui::files", "成功跳转远程路径: {}", p_str);
                }
                Err(err) => {
                    ctx_nav_remote.notify_error("路径不存在", err.clone());
                    let current_valid = ctx_nav_remote.remote_current_path.borrow().clone();
                    w.set_remote_current_path(current_valid.into());
                    tracing::warn!(target: "smagical_ui::files", "远程路径跳转失败: {}", p_str);
                }
            }
        }
    });

    // 4.6 远程历史后退
    let window_weak = window.as_weak();
    let ctx_back_rem = ctx.clone();
    window.on_navigate_remote_back(move || {
        if let Some(w) = window_weak.upgrade() {
            let act_id = ctx_back_rem.active_remote_tab_id.borrow().clone();
            let target_path = {
                let mut tabs = ctx_back_rem.remote_tabs.borrow_mut();
                tabs.iter_mut().find(|t| t.tab_id == act_id).and_then(|t| t.go_back())
            };
            if let Some(path) = target_path {
                let old_p = ctx_back_rem.remote_current_path.borrow().clone();
                let _ = try_refresh_remote_path(&ctx_back_rem, &path, false);
                ctx_back_rem.core_state.events().dispatch(&FileTabNavigatedEvent {
                    tab_id: act_id.clone(),
                    is_remote: true,
                    old_path: old_p.clone(),
                    new_path: path.clone(),
                });
                sync_file_explorer_ui(&w, &ctx_back_rem);
                tracing::info!(target: "smagical_ui::files", "远程文件历史后退至: {}", path);
            }
        }
    });

    // 4.7 远程历史前进
    let window_weak = window.as_weak();
    let ctx_fwd_rem = ctx.clone();
    window.on_navigate_remote_forward(move || {
        if let Some(w) = window_weak.upgrade() {
            let act_id = ctx_fwd_rem.active_remote_tab_id.borrow().clone();
            let target_path = {
                let mut tabs = ctx_fwd_rem.remote_tabs.borrow_mut();
                tabs.iter_mut().find(|t| t.tab_id == act_id).and_then(|t| t.go_forward())
            };
            if let Some(path) = target_path {
                let old_p = ctx_fwd_rem.remote_current_path.borrow().clone();
                let _ = try_refresh_remote_path(&ctx_fwd_rem, &path, false);
                ctx_fwd_rem.core_state.events().dispatch(&FileTabNavigatedEvent {
                    tab_id: act_id.clone(),
                    is_remote: true,
                    old_path: old_p.clone(),
                    new_path: path.clone(),
                });
                sync_file_explorer_ui(&w, &ctx_fwd_rem);
                tracing::info!(target: "smagical_ui::files", "远程文件历史前进至: {}", path);
            }
        }
    });

    // 4.8 远程/右栏上级目录导航 (兼容本地与远程路径)
    let window_weak = window.as_weak();
    let ctx_up_remote = ctx.clone();
    window.on_navigate_remote_up(move || {
        if let Some(w) = window_weak.upgrade() {
            let act_id = ctx_up_remote.active_remote_tab_id.borrow().clone();
            let is_local_session = {
                let tabs = ctx_up_remote.remote_tabs.borrow();
                tabs.iter().find(|t| t.tab_id == act_id).map(|t| t.host_id == "local").unwrap_or(false)
            };
            let current = ctx_up_remote.remote_current_path.borrow().clone();

            if is_local_session {
                let p = std::path::Path::new(&current);
                if let Some(parent) = p.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if !parent_str.is_empty() {
                        let _ = try_refresh_remote_path(&ctx_up_remote, &parent_str, true);
                        ctx_up_remote.core_state.events().dispatch(&FileTabNavigatedEvent {
                            tab_id: act_id.clone(),
                            is_remote: true,
                            old_path: current.clone(),
                            new_path: parent_str.clone(),
                        });
                        sync_file_explorer_ui(&w, &ctx_up_remote);
                    }
                }
            } else if let Some(pos) = current.rfind('/') {
                let parent_str = if pos == 0 { "/" } else { &current[..pos] };
                let _ = try_refresh_remote_path(&ctx_up_remote, parent_str, true);
                ctx_up_remote.core_state.events().dispatch(&FileTabNavigatedEvent {
                    tab_id: act_id.clone(),
                    is_remote: true,
                    old_path: current.clone(),
                    new_path: parent_str.to_string(),
                });
                sync_file_explorer_ui(&w, &ctx_up_remote);
            }
        }
    });



    // 4.5 打开本地文件/文件夹 (双击)
    let window_weak = window.as_weak();
    let ctx_open_local = ctx.clone();
    window.on_open_local_item(move |path, is_dir| {

        if let Some(w) = window_weak.upgrade() {
            let p_str = path.to_string();
            if is_dir {
                refresh_local_path(&ctx_open_local, &p_str);
                sync_file_explorer_ui(&w, &ctx_open_local);
                tracing::info!(target: "smagical_ui::files", "进入本地目录: {}", p_str);
            } else {
                tracing::info!(target: "smagical_ui::files", "双击本地文件: {}", p_str);
            }
        }
    });

    // 4.4 打开远程文件/文件夹 (双击)
    let window_weak = window.as_weak();
    let ctx_open_remote = ctx.clone();
    window.on_open_remote_item(move |path, is_dir| {
        if let Some(w) = window_weak.upgrade() {
            let p_str = path.to_string();
            if is_dir {
                refresh_remote_path(&ctx_open_remote, &p_str);
                sync_file_explorer_ui(&w, &ctx_open_remote);
                tracing::info!(target: "smagical_ui::files", "进入远程目录: {}", p_str);
            } else {
                tracing::info!(target: "smagical_ui::files", "双击远程文件: {}", p_str);
            }
        }
    });

    // 4.5 刷新本地文件列表
    let window_weak = window.as_weak();
    let ctx_ref_local = ctx.clone();
    window.on_refresh_local_files(move || {
        if let Some(w) = window_weak.upgrade() {
            let p = ctx_ref_local.local_current_path.borrow().clone();
            refresh_local_path(&ctx_ref_local, &p);
            sync_file_explorer_ui(&w, &ctx_ref_local);
            tracing::info!(target: "smagical_ui::files", "刷新本地文件目录: {}", p);
        }
    });

    // 4.6 刷新远程文件列表
    let window_weak = window.as_weak();
    let ctx_ref_remote = ctx.clone();
    window.on_refresh_remote_files(move || {
        if let Some(w) = window_weak.upgrade() {
            let p = ctx_ref_remote.remote_current_path.borrow().clone();
            refresh_remote_path(&ctx_ref_remote, &p);
            sync_file_explorer_ui(&w, &ctx_ref_remote);
            tracing::info!(target: "smagical_ui::files", "刷新远程文件目录: {}", p);
        }
    });

    // 4.7 上传选中文件 (Local -> Remote)
    let window_weak = window.as_weak();
    let ctx_upload = ctx.clone();
    window.on_upload_file(move || {
        if let Some(w) = window_weak.upgrade() {
            let loc_path = ctx_upload.local_current_path.borrow().clone();
            let rem_path = ctx_upload.remote_current_path.borrow().clone();
            let filename = std::path::Path::new(&loc_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "uploaded_file.bin".to_string());

            let task_id = format!("task-up-{}", ctx_upload.transfer_tasks.borrow().len() + 1);
            let task = TransferTask {
                id: task_id.clone(),
                parent_id: None,
                session_id: "session-active".into(),
                filename,
                is_dir: false,
                is_expanded: false,
                level: 0,
                item_count_text: "".into(),
                source_path: loc_path.clone(),
                target_path: rem_path.clone(),
                direction: TransferDirection::Upload,
                total_bytes: 45_200_000,
                transferred_bytes: 31_640_000,
                speed_bytes_per_sec: 14_800_000,
                status: TransferStatus::Transferring,
                error_message: None,
            };
            ctx_upload.transfer_tasks.borrow_mut().push(task);
            sync_file_explorer_ui(&w, &ctx_upload);
            tracing::info!(
                target: "smagical_ui::files",
                "创建文件上传任务: 本地目录 [{}] -> 远程目录 [{}]",
                loc_path, rem_path
            );
        }
    });

    // 4.8 下载选中文件 (Remote -> Local)
    let window_weak = window.as_weak();
    let ctx_download = ctx.clone();
    window.on_download_file(move || {
        if let Some(w) = window_weak.upgrade() {
            let loc_path = ctx_download.local_current_path.borrow().clone();
            let rem_path = ctx_download.remote_current_path.borrow().clone();
            let filename = rem_path.split('/').next_back().filter(|s| !s.is_empty()).unwrap_or("downloaded_file.tar.gz").to_string();

            let task_id = format!("task-down-{}", ctx_download.transfer_tasks.borrow().len() + 1);
            let task = TransferTask {
                id: task_id.clone(),
                parent_id: None,
                session_id: "session-active".into(),
                filename,
                is_dir: false,
                is_expanded: false,
                level: 0,
                item_count_text: "".into(),
                source_path: rem_path.clone(),
                target_path: loc_path.clone(),
                direction: TransferDirection::Download,
                total_bytes: 128_000_000,
                transferred_bytes: 128_000_000,
                speed_bytes_per_sec: 0,
                status: TransferStatus::Completed,
                error_message: None,
            };

            ctx_download.transfer_tasks.borrow_mut().push(task);
            sync_file_explorer_ui(&w, &ctx_download);
            tracing::info!(
                target: "smagical_ui::files",
                "创建文件下载任务: 远程目录 [{}] -> 本地目录 [{}]",
                rem_path, loc_path
            );
        }
    });

    // 4.9 清空已完成或失败的传输任务
    let window_weak = window.as_weak();
    let ctx_clear_trans = ctx.clone();
    window.on_clear_completed_transfers(move || {
        if let Some(w) = window_weak.upgrade() {
            let mut tasks = ctx_clear_trans.transfer_tasks.borrow_mut();
            tasks.retain(|t| t.status == TransferStatus::Transferring || t.status == TransferStatus::Pending);
            drop(tasks);
            sync_file_explorer_ui(&w, &ctx_clear_trans);
            tracing::info!(target: "smagical_ui::files", "清空已完成的传输任务");
        }
    });

    // 4.10 触发拖拽文件/文件夹传输任务 (支持单文件与多文件夹嵌套树)
    let window_weak = window.as_weak();
    let ctx_start_trans = ctx.clone();
    window.on_start_transfer_task(move |dir, source_path, filename, is_dir, target_dir| {
        if let Some(w) = window_weak.upgrade() {
            let dir_enum = if dir == "download" { TransferDirection::Download } else { TransferDirection::Upload };
            let src_str = source_path.to_string();
            let name_str = filename.to_string();
            let tgt_str = target_dir.to_string();
            let mut tasks = ctx_start_trans.transfer_tasks.borrow_mut();

            let target_combined = if tgt_str.ends_with('/') || tgt_str.ends_with('\\') {
                format!("{}{}", tgt_str, name_str)
            } else {
                format!("{}/{}", tgt_str, name_str)
            };

            if is_dir {
                let folder_id = format!("task-folder-{}", tasks.len() + 1);
                // 模拟文件夹内部包含的子文件
                let child1 = TransferTask {
                    id: format!("{}-f1", folder_id),
                    parent_id: Some(folder_id.clone()),
                    session_id: "active-session".into(),
                    filename: "main.rs".into(),
                    is_dir: false,
                    is_expanded: false,
                    level: 1,
                    item_count_text: "".into(),
                    source_path: format!("{}/main.rs", src_str.trim_end_matches(['/', '\\'])),
                    target_path: format!("{}/main.rs", target_combined.trim_end_matches(['/', '\\'])),
                    direction: dir_enum,
                    total_bytes: 8_500_000,
                    transferred_bytes: 8_500_000,
                    speed_bytes_per_sec: 0,
                    status: TransferStatus::Completed,
                    error_message: None,
                };
                let child2 = TransferTask {
                    id: format!("{}-f2", folder_id),
                    parent_id: Some(folder_id.clone()),
                    session_id: "active-session".into(),
                    filename: "Cargo.toml".into(),
                    is_dir: false,
                    is_expanded: false,
                    level: 1,
                    item_count_text: "".into(),
                    source_path: format!("{}/Cargo.toml", src_str.trim_end_matches(['/', '\\'])),
                    target_path: format!("{}/Cargo.toml", target_combined.trim_end_matches(['/', '\\'])),
                    direction: dir_enum,
                    total_bytes: 1_200_000,
                    transferred_bytes: 1_200_000,
                    speed_bytes_per_sec: 0,
                    status: TransferStatus::Completed,
                    error_message: None,
                };
                let child3 = TransferTask {
                    id: format!("{}-f3", folder_id),
                    parent_id: Some(folder_id.clone()),
                    session_id: "active-session".into(),
                    filename: "bundle.js".into(),
                    is_dir: false,
                    is_expanded: false,
                    level: 1,
                    item_count_text: "".into(),
                    source_path: format!("{}/bundle.js", src_str.trim_end_matches(['/', '\\'])),
                    target_path: format!("{}/bundle.js", target_combined.trim_end_matches(['/', '\\'])),
                    direction: dir_enum,
                    total_bytes: 35_000_000,
                    transferred_bytes: 24_500_000,
                    speed_bytes_per_sec: 8_400_000,
                    status: TransferStatus::Transferring,
                    error_message: None,
                };
                let child4 = TransferTask {
                    id: format!("{}-f4", folder_id),
                    parent_id: Some(folder_id.clone()),
                    session_id: "active-session".into(),
                    filename: "assets.tar".into(),
                    is_dir: false,
                    is_expanded: false,
                    level: 1,
                    item_count_text: "".into(),
                    source_path: format!("{}/assets.tar", src_str.trim_end_matches(['/', '\\'])),
                    target_path: format!("{}/assets.tar", target_combined.trim_end_matches(['/', '\\'])),
                    direction: dir_enum,
                    total_bytes: 55_300_000,
                    transferred_bytes: 16_800_000,
                    speed_bytes_per_sec: 9_200_000,
                    status: TransferStatus::Transferring,
                    error_message: None,
                };

                let folder_total = child1.total_bytes + child2.total_bytes + child3.total_bytes + child4.total_bytes;
                let folder_trans = child1.transferred_bytes + child2.transferred_bytes + child3.transferred_bytes + child4.transferred_bytes;
                let folder_speed = child1.speed_bytes_per_sec + child2.speed_bytes_per_sec + child3.speed_bytes_per_sec + child4.speed_bytes_per_sec;

                let parent_task = TransferTask {
                    id: folder_id.clone(),
                    parent_id: None,
                    session_id: "active-session".into(),
                    filename: name_str.clone(),
                    is_dir: true,
                    is_expanded: false,
                    level: 0,
                    item_count_text: "4 项".into(),
                    source_path: src_str.clone(),
                    target_path: target_combined.clone(),
                    direction: dir_enum,
                    total_bytes: folder_total,
                    transferred_bytes: folder_trans,
                    speed_bytes_per_sec: folder_speed,
                    status: TransferStatus::Transferring,
                    error_message: None,
                };

                ctx_start_trans.core_state.events().dispatch(&FileTransferStartedEvent {
                    task_id: parent_task.id.clone(),
                });
                tasks.push(parent_task);
                tasks.push(child1);
                tasks.push(child2);
                tasks.push(child3);
                tasks.push(child4);
                tracing::info!(
                    target: "smagical_ui::files",
                    "拖拽创建文件夹传输任务: {} [{}] -> [{}], 共 4 个子文件, 总进度: {}%",
                    dir, src_str, target_combined, (folder_trans * 100 / folder_total)
                );
            } else {
                let task_id = format!("task-file-{}", tasks.len() + 1);
                let single_task = TransferTask {
                    id: task_id.clone(),
                    parent_id: None,
                    session_id: "active-session".into(),
                    filename: name_str.clone(),
                    is_dir: false,
                    is_expanded: false,
                    level: 0,
                    item_count_text: "".into(),
                    source_path: src_str.clone(),
                    target_path: target_combined.clone(),
                    direction: dir_enum,
                    total_bytes: 42_500_000,
                    transferred_bytes: 28_000_000,
                    speed_bytes_per_sec: 14_200_000,
                    status: TransferStatus::Transferring,
                    error_message: None,
                };
                ctx_start_trans.core_state.events().dispatch(&FileTransferStartedEvent {
                    task_id: single_task.id.clone(),
                });
                tasks.push(single_task);
                tracing::info!(
                    target: "smagical_ui::files",
                    "拖拽创建单文件传输任务: {} [{}] -> [{}]",
                    dir, src_str, target_combined
                );
            }

            drop(tasks);
            sync_file_explorer_ui(&w, &ctx_start_trans);
        }
    });

    // 4.11 折叠/展开传输队列中的文件夹任务
    let window_weak = window.as_weak();
    let ctx_toggle_trans = ctx.clone();
    window.on_toggle_transfer_expand(move |task_id| {
        if let Some(w) = window_weak.upgrade() {
            let tid = task_id.to_string();
            let mut tasks = ctx_toggle_trans.transfer_tasks.borrow_mut();
            let is_expanded = if let Some(folder) = tasks.iter_mut().find(|t| t.id == tid && t.is_dir) {
                folder.is_expanded = !folder.is_expanded;
                folder.is_expanded
            } else {
                false
            };
            drop(tasks);
            sync_file_explorer_ui(&w, &ctx_toggle_trans);
            tracing::debug!(target: "smagical_ui::files", "传输任务折叠/展开: task_id={}, is_expanded={}", tid, is_expanded);
        }
    });

    // 4.12 传输队列右键快捷操作 (暂停/继续/停止/重新传输/移除)
    let window_weak = window.as_weak();
    let ctx_trans_act = ctx.clone();
    window.on_transfer_action(move |action, task_id| {
        if let Some(w) = window_weak.upgrade() {
            let act = action.as_str();
            let tid = task_id.to_string();
            let mut tasks = ctx_trans_act.transfer_tasks.borrow_mut();

            match act {
                "pause" => {
                    for t in tasks.iter_mut() {
                        if (t.id == tid || t.parent_id.as_deref() == Some(&tid))
                            && t.status == TransferStatus::Transferring
                        {
                            t.status = TransferStatus::Paused;
                            t.speed_bytes_per_sec = 0;
                        }
                    }
                    tracing::info!(target: "smagical_ui::files", "暂停传输任务: {}", tid);
                }
                "resume" => {
                    for t in tasks.iter_mut() {
                        if (t.id == tid || t.parent_id.as_deref() == Some(&tid))
                            && t.status == TransferStatus::Paused
                        {
                            t.status = TransferStatus::Transferring;
                            t.speed_bytes_per_sec = 12_500_000;
                        }
                    }
                    tracing::info!(target: "smagical_ui::files", "恢复传输任务: {}", tid);
                }
                "stop" => {
                    for t in tasks.iter_mut() {
                        if t.id == tid || t.parent_id.as_deref() == Some(&tid) {
                            t.status = TransferStatus::Failed;
                            t.speed_bytes_per_sec = 0;
                            t.error_message = Some("用户手动终止传输".into());
                        }
                    }
                    tracing::info!(target: "smagical_ui::files", "停止传输任务: {}", tid);
                }
                "retry" => {
                    for t in tasks.iter_mut() {
                        if t.id == tid || t.parent_id.as_deref() == Some(&tid) {
                            t.status = TransferStatus::Transferring;
                            t.transferred_bytes = 0;
                            t.speed_bytes_per_sec = 14_000_000;
                            t.error_message = None;
                        }
                    }
                    tracing::info!(target: "smagical_ui::files", "重新传输任务: {}", tid);
                }
                "remove" => {
                    tasks.retain(|t| t.id != tid && t.parent_id.as_deref() != Some(&tid));
                    tracing::info!(target: "smagical_ui::files", "移除传输任务记录: {}", tid);
                }
                _ => {}
            }

            drop(tasks);
            sync_file_explorer_ui(&w, &ctx_trans_act);
        }
    });

    // 4.13 文件/目录右键快捷操作 (打开/传输/新建文件夹/新建文件/刷新/删除)
    let window_weak = window.as_weak();
    let ctx_file_act = ctx.clone();
    window.on_file_action(move |action, is_remote, path, name, is_dir| {
        if let Some(w) = window_weak.upgrade() {
            let act = action.as_str();
            let p_str = path.to_string();
            let n_str = name.to_string();

            match act {
                "open" => {
                    if is_remote {
                        if is_dir {
                            refresh_remote_path(&ctx_file_act, &p_str);
                            sync_file_explorer_ui(&w, &ctx_file_act);
                        }
                    } else if is_dir {
                        refresh_local_path(&ctx_file_act, &p_str);
                        sync_file_explorer_ui(&w, &ctx_file_act);
                    } else {
                        #[cfg(target_os = "windows")]
                        let _ = std::process::Command::new("explorer").arg(&p_str).spawn();
                        #[cfg(not(target_os = "windows"))]
                        let _ = std::process::Command::new("xdg-open").arg(&p_str).spawn();
                    }
                }
                "transfer" => {
                    let dir_enum = if is_remote { TransferDirection::Download } else { TransferDirection::Upload };
                    let dir_str = if is_remote { "download" } else { "upload" };
                    let target_dir = if is_remote {
                        ctx_file_act.local_current_path.borrow().clone()
                    } else {
                        ctx_file_act.remote_current_path.borrow().clone()
                    };

                    let target_combined = if target_dir.ends_with('/') || target_dir.ends_with('\\') {
                        format!("{}{}", target_dir, n_str)
                    } else {
                        format!("{}/{}", target_dir, n_str)
                    };

                    let mut tasks = ctx_file_act.transfer_tasks.borrow_mut();
                    let task_id = format!("task-menu-{}", tasks.len() + 1);
                    let task = TransferTask {
                        id: task_id,
                        parent_id: None,
                        session_id: "active-session".into(),
                        filename: n_str,
                        is_dir,
                        is_expanded: false,
                        level: 0,
                        item_count_text: if is_dir { "1 项".into() } else { "".into() },
                        source_path: p_str,
                        target_path: target_combined,
                        direction: dir_enum,
                        total_bytes: 35_000_000,
                        transferred_bytes: 10_500_000,
                        speed_bytes_per_sec: 11_200_000,
                        status: TransferStatus::Transferring,
                        error_message: None,
                    };
                    tasks.push(task);
                    drop(tasks);
                    w.set_is_transfer_queue_expanded(true);
                    sync_file_explorer_ui(&w, &ctx_file_act);
                    tracing::info!(target: "smagical_ui::files", "通过右键菜单创建传输任务: {}", dir_str);
                }
                "create_folder" => {
                    if !is_remote {
                        let parent = if is_dir { std::path::PathBuf::from(&p_str) } else { std::path::PathBuf::from(&p_str).parent().unwrap_or(std::path::Path::new(".")).to_path_buf() };
                        let mut target = parent.join("新建文件夹");
                        let mut counter = 1;
                        while target.exists() {
                            target = parent.join(format!("新建文件夹 ({})", counter));
                            counter += 1;
                        }
                        let ok = std::fs::create_dir_all(&target).is_ok();
                        ctx_file_act.core_state.events().dispatch(&FileOperationCompletedEvent {
                            action: "create_folder".into(),
                            is_remote: false,
                            path: target.to_string_lossy().to_string(),
                            success: ok,
                        });
                        let cur_p = ctx_file_act.local_current_path.borrow().clone();
                        refresh_local_path(&ctx_file_act, &cur_p);
                        sync_file_explorer_ui(&w, &ctx_file_act);
                    }
                }
                "create_file" => {
                    if !is_remote {
                        let parent = if is_dir { std::path::PathBuf::from(&p_str) } else { std::path::PathBuf::from(&p_str).parent().unwrap_or(std::path::Path::new(".")).to_path_buf() };
                        let mut target = parent.join("新建文本文档.txt");
                        let mut counter = 1;
                        while target.exists() {
                            target = parent.join(format!("新建文本文档 ({}).txt", counter));
                            counter += 1;
                        }
                        let ok = std::fs::File::create(&target).is_ok();
                        ctx_file_act.core_state.events().dispatch(&FileOperationCompletedEvent {
                            action: "create_file".into(),
                            is_remote: false,
                            path: target.to_string_lossy().to_string(),
                            success: ok,
                        });
                        let cur_p = ctx_file_act.local_current_path.borrow().clone();
                        refresh_local_path(&ctx_file_act, &cur_p);
                        sync_file_explorer_ui(&w, &ctx_file_act);
                    }
                }
                "delete" => {
                    // 1. 高危操作事件总线前置安全审查
                    let del_event = FileOperationBeforeEvent::new("delete", is_remote, &p_str);
                    ctx_file_act.core_state.events().dispatch(&del_event);
                    if del_event.is_aborted() {
                        ctx_file_act.notify_warning("高危操作拦截", del_event.abort_reason().unwrap_or_default());
                        return;
                    }

                    if !is_remote {
                        let ok = if is_dir {
                            std::fs::remove_dir_all(&p_str).is_ok()
                        } else {
                            std::fs::remove_file(&p_str).is_ok()
                        };
                        ctx_file_act.core_state.events().dispatch(&FileOperationCompletedEvent {
                            action: "delete".into(),
                            is_remote: false,
                            path: p_str.clone(),
                            success: ok,
                        });
                        if ok {
                            ctx_file_act.notify_info("已删除", format!("已删除: {}", n_str));
                        }
                        let cur_p = ctx_file_act.local_current_path.borrow().clone();
                        refresh_local_path(&ctx_file_act, &cur_p);
                        sync_file_explorer_ui(&w, &ctx_file_act);
                    }
                }
                "refresh" => {
                    if is_remote {
                        let cur_p = ctx_file_act.remote_current_path.borrow().clone();
                        refresh_remote_path(&ctx_file_act, &cur_p);
                    } else {
                        let cur_p = ctx_file_act.local_current_path.borrow().clone();
                        refresh_local_path(&ctx_file_act, &cur_p);
                    }
                    sync_file_explorer_ui(&w, &ctx_file_act);
                }
                _ => {}
            }
        }
    });

    // 4.14 复制路径到剪贴板
    window.on_copy_to_clipboard(move |text| {
        let t_str = text.to_string();
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(t_str.clone());
        }
        tracing::info!(target: "smagical_ui::files", "已复制到剪贴板: {}", t_str);
    });



    // -------------------------------------------------------------------------
    // 5. 文件会话选择弹窗 (FileHostModal) 交互
    // -------------------------------------------------------------------------
    // 5.1 实时搜索过滤
    let window_weak = window.as_weak();
    let ctx_filter_file = ctx.clone();
    window.on_filter_file_launcher(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let hosts = build_file_launcher_hosts(&ctx_filter_file, query.as_str());
            w.set_file_launcher_host_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(hosts))));
        }
    });

    // 5.2 选取主机并自动连接 SFTP 会话 (或在右栏打开本地目录会话)
    let window_weak = window.as_weak();
    let ctx_open_fhost = ctx.clone();
    window.on_open_file_host(move |host_id| {
        if let Some(w) = window_weak.upgrade() {
            let hid = host_id.to_string();

            // 若在右栏打开本地文件系统
            if hid == "local" {
                let home_path = directories::BaseDirs::new()
                    .map(|p| p.home_dir().to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string());
                let mut tabs = ctx_open_fhost.remote_tabs.borrow_mut();
                let new_idx = tabs.len() + 1;
                let tab_id = format!("rtab-{}", new_idx);
                let session = RemoteFileTabSession::new(
                    tab_id.clone(),
                    "local",
                    format!("本地 (目录 #{})", new_idx),
                    "Local Filesystem",
                    home_path.clone(),
                );
                tabs.push(session);
                *ctx_open_fhost.active_remote_tab_id.borrow_mut() = tab_id.clone();
                drop(tabs);

                ctx_open_fhost.core_state.events().dispatch(&FileTabOpenedEvent {
                    tab_id: tab_id.clone(),
                    host_id: "local".into(),
                    path: home_path.clone(),
                });
                refresh_remote_path(&ctx_open_fhost, &home_path);
                sync_file_explorer_ui(&w, &ctx_open_fhost);
                tracing::info!(target: "smagical_ui::files", "在右栏新建并打开本地文件目录 Tab: tab_id={}, path={}", tab_id, home_path);
                return;
            }

            // 事件总线前置安全审查
            let open_event = FileTabOpeningEvent::new(&hid, "/root");
            ctx_open_fhost.core_state.events().dispatch(&open_event);
            if open_event.is_aborted() {
                ctx_open_fhost.notify_warning("连接已拦截", open_event.abort_reason().unwrap_or_default());
                return;
            }

            let mut tabs = ctx_open_fhost.remote_tabs.borrow_mut();

            // 若已有打开的该主机 Tab，则直接切换过去
            if let Some(existing) = tabs.iter().find(|t| t.host_id == hid) {
                let tid = existing.tab_id.clone();
                let rem = existing.current_path.clone();
                *ctx_open_fhost.active_remote_tab_id.borrow_mut() = tid.clone();
                drop(tabs);

                ctx_open_fhost.core_state.events().dispatch(&FileTabFocusChangedEvent {
                    tab_id: Some(tid.clone()),
                    is_remote: true,
                    current_path: rem.clone(),
                });
                refresh_remote_path(&ctx_open_fhost, &rem);
                sync_file_explorer_ui(&w, &ctx_open_fhost);
                tracing::info!(target: "smagical_ui::files", "文件弹窗切换至已有远程 SFTP Tab: {}", tid);
                return;
            }

            // 查询主机元数据
            let tree = ctx_open_fhost.master_tree.borrow();
            let target_node = tree.iter().find(|n| n.id == hid && !n.is_group).cloned();
            drop(tree);

            let (h_name, h_addr) = if let Some(n) = target_node {
                (n.name, if n.port > 0 { format!("{}:{}", n.address, n.port) } else { n.address })
            } else if let Ok(Some(h)) = ctx_open_fhost.core_state.storage().hosts().get_by_id(&hid) {
                (h.name, format!("{}:{}", h.address, h.port))
            } else {
                (format!("Host ({})", hid), "127.0.0.1:22".into())
            };

            let tab_id = format!("rtab-{}", tabs.len() + 1);
            let session = RemoteFileTabSession::new(
                tab_id.clone(),
                hid.clone(),
                h_name,
                h_addr,
                "/root",
            );
            tabs.push(session);
            *ctx_open_fhost.active_remote_tab_id.borrow_mut() = tab_id.clone();
            drop(tabs);

            ctx_open_fhost.core_state.events().dispatch(&FileTabOpenedEvent {
                tab_id: tab_id.clone(),
                host_id: hid.clone(),
                path: "/root".into(),
            });
            refresh_remote_path(&ctx_open_fhost, "/root");
            sync_file_explorer_ui(&w, &ctx_open_fhost);
            tracing::info!(target: "smagical_ui::files", "文件弹窗自动连接并创建远程 SFTP Tab: host_id={}, tab_id={}", hid, tab_id);
        }
    });

    // -------------------------------------------------------------------------
    // 16. 本地 Tab 拖拽调整顺序 (仅在同栏内生效)
    // -------------------------------------------------------------------------
    let ctx_reorder_loc = ctx.clone();
    let window_weak = window.as_weak();
    window.on_reorder_local_tab(move |from_idx: i32, to_idx: i32| {
        if from_idx == to_idx || from_idx < 0 || to_idx < 0 {
            return;
        }
        let from = from_idx as usize;
        let to = to_idx as usize;
        let mut tabs = ctx_reorder_loc.local_tabs.borrow_mut();
        if from < tabs.len() && to < tabs.len() {
            let item = tabs.remove(from);
            tabs.insert(to, item);
            tracing::info!(target: "smagical_ui::files", "本地 Tab 拖拽重排: {} -> {}", from, to);
        }
        drop(tabs);
        if let Some(w) = window_weak.upgrade() {
            sync_local_tabs_only(&w, &ctx_reorder_loc);
        }
    });

    // -------------------------------------------------------------------------
    // 17. 远程 Tab 拖拽调整顺序 (仅在同栏内生效)
    // -------------------------------------------------------------------------
    let ctx_reorder_rem = ctx.clone();
    let window_weak = window.as_weak();
    window.on_reorder_remote_tab(move |from_idx: i32, to_idx: i32| {
        if from_idx == to_idx || from_idx < 0 || to_idx < 0 {
            return;
        }
        let from = from_idx as usize;
        let to = to_idx as usize;
        let mut tabs = ctx_reorder_rem.remote_tabs.borrow_mut();
        if from < tabs.len() && to < tabs.len() {
            let item = tabs.remove(from);
            tabs.insert(to, item);
            tracing::info!(target: "smagical_ui::files", "远程 Tab 拖拽重排: {} -> {}", from, to);
        }
        drop(tabs);
        if let Some(w) = window_weak.upgrade() {
            sync_remote_tabs_only(&w, &ctx_reorder_rem);
        }
    });
}


