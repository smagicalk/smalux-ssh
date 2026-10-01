//! 双盘文件管理与 SFTP 远程传输 UI 交互处理器。
//!
//! 负责本地与远程文件系统目录遍历、独立双栏 Tab 调度、路径导航与文件上传/下载任务流转。

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use slint::ComponentHandle;
use smagical_core::event::{
    FileOperationBeforeEvent, FileOperationCompletedEvent, FileTabClosedEvent,
    FileTabFocusChangedEvent, FileTabNavigatedEvent, FileTabOpenedEvent,
    FileTabOpeningEvent, FileTransferStartedEvent,
};
use smagical_core::{
    scan_local_directory, CredentialType, FileItemData,
    LocalFileTabSession, RemoteFileTabSession, SftpService, TransferDirection, TransferStatus, TransferTask,
};

use crate::generated::{
    AppWindow, FileItemData as SlintFileItemData, FileTabData as SlintFileTabData,
    FilesBridge, HostItemData as SlintHostItemData, TransferItemData as SlintTransferItemData,
    WindowBridge,
};
use crate::handlers::AppContext;
use crate::terminal::SshLaunchConfig;

thread_local! {
    static FILE_APP_CTX: RefCell<Option<AppContext>> = const { RefCell::new(None) };
    static FILE_WINDOW_WEAK: RefCell<Option<slint::Weak<AppWindow>>> = const { RefCell::new(None) };
}

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
    let tree = ctx.master_tree.read().unwrap();
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

/// 递归扫描指定本地文件夹，统计文件总数、总大小与子文件层级信息
fn scan_folder_recursive(dir: &std::path::Path) -> (usize, u64, Vec<(String, PathBuf, u64, String)>) {
    let mut file_count = 0;
    let mut total_bytes = 0;
    let mut sub_items = Vec::new();

    fn walk(
        base: &std::path::Path,
        current: &std::path::Path,
        file_count: &mut usize,
        total_bytes: &mut u64,
        sub_items: &mut Vec<(String, PathBuf, u64, String)>,
    ) {
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk(base, &p, file_count, total_bytes, sub_items);
                } else if p.is_file() {
                    *file_count += 1;
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    *total_bytes += size;
                    let rel = p.strip_prefix(base).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                    let name = entry.file_name().to_string_lossy().to_string();
                    sub_items.push((name, p, size, rel));
                }
            }
        }
    }

    walk(dir, dir, &mut file_count, &mut total_bytes, &mut sub_items);
    (file_count, total_bytes, sub_items)
}

/// 异步扫描本地目录，将耗时文件系统 IO 派发至 Tokio 阻塞线程池执行 (杜绝大目录冻结 UI)
#[allow(dead_code)]
pub async fn scan_local_directory_async(dir_path: PathBuf) -> std::io::Result<Vec<FileItemData>> {
    tokio::task::spawn_blocking(move || {
        scan_local_directory(&dir_path)
    })
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
}

/// 仅同步左侧本地 Tab 列表 (用于拖拽重排等无需全量扫描的轻量操作)
pub(crate) fn sync_local_tabs_only(window: &AppWindow, ctx: &AppContext) {
    let is_en = window.global::<WindowBridge>().get_current_language() == "en-US";
    let local_tabs = ctx.local_tabs.borrow();
    let active_local_id = ctx.active_local_tab_id.borrow().clone();
    let ui_local_tabs: Vec<SlintFileTabData> = local_tabs
        .iter()
        .map(|t| {
            let display_title = if is_en {
                if t.title == "本地 (主目录)" {
                    "Local (Home)".to_string()
                } else if t.title.starts_with("本地 #") {
                    t.title.replace("本地 #", "Local #")
                } else if t.title == "本地" || t.title == "本地目录" {
                    "Local Directory".to_string()
                } else {
                    t.title.clone()
                }
            } else {
                if t.title == "Local (Home)" {
                    "本地 (主目录)".to_string()
                } else if t.title.starts_with("Local #") {
                    t.title.replace("Local #", "本地 #")
                } else if t.title == "Local Directory" {
                    "本地目录".to_string()
                } else {
                    t.title.clone()
                }
            };
            SlintFileTabData {
                id: t.tab_id.clone().into(),
                host_id: "local".into(),
                title: display_title.into(),
                subtitle: t.current_path.clone().into(),
                status: "online".into(),
                is_active: t.tab_id == active_local_id,
            }
        })
        .collect();
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(ui_local_tabs)));
    let fb = window.global::<FilesBridge>();
    fb.set_local_tabs(model);
    fb.set_active_local_tab_id(active_local_id.into());
}

/// 仅同步右侧远程 Tab 列表 (用于拖拽重排等无需全量扫描的轻量操作)
pub(crate) fn sync_remote_tabs_only(window: &AppWindow, ctx: &AppContext) {
    let remote_tabs = ctx.remote_tabs.borrow();
    let active_remote_id = ctx.active_remote_tab_id.borrow().clone();
    let active_tab = remote_tabs.iter().find(|t| t.tab_id == active_remote_id);
    let is_connecting = active_tab.map(|t| t.status == "connecting").unwrap_or(false);
    let is_error = active_tab.map(|t| t.status == "error").unwrap_or(false);
    let error_msg = active_tab.and_then(|t| t.error_msg.clone()).unwrap_or_default();
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
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(ui_remote_tabs)));
    let fb = window.global::<FilesBridge>();
    fb.set_remote_tabs(model);
    fb.set_active_remote_tab_id(active_remote_id.into());
    fb.set_is_remote_connecting(is_connecting);
    fb.set_is_remote_error(is_error);
    if is_error {
        fb.set_remote_error_msg(error_msg.into());
    } else if is_connecting {
        fb.set_remote_error_msg("".into());
    }
}

/// 同步当前激活文件会话的双盘数据到 Slint UI。
///
/// 聚合执行完整的双盘状态水合流水线：
/// 1. 同步左侧本地 Tab 列表 (`sync_local_tabs_only`)；
/// 2. 同步右侧远程 Tab 列表 (`sync_remote_tabs_only`)；
/// 3. 同步本地与远程的双向当前面包屑工作路径；
/// 4. 同步文件列表并执行两级智能过滤规则：
///    - **隐藏文件过滤**：若设置项 `setting_sftp_show_hidden` 为 `false`，自动剔除所有以 `.` 开头或属性为隐藏的文件；
///    - **排除黑名单通配符过滤**：根据 `setting_sftp_exclude_patterns`（逗号分隔，如 `*.log, node_modules, *.tmp`）
///      执行通配符后缀匹配与包含匹配，动态屏蔽无需展示的文件；
/// 5. 同步两侧历史导航栈的前进/后退按钮使能状态 (`can_go_back` / `can_go_forward`)；
/// 6. 刷新连接弹窗中的备选主机清单 (`file_launcher_host_items`)；
/// 7. 同步传输任务队列，支持文件夹折叠时过滤隐藏其下属子任务。
///
/// # 参数
/// - `window`: Slint 主窗口实例引用；
/// - `ctx`: 全局应用共享上下文对象引用。
/// 判断指定本地路径是否可以向上返回上一级目录 (到达根目录如 C:\ 或 / 时返回 false)
pub(crate) fn can_navigate_local_up(path: &str) -> bool {
    let p = std::path::Path::new(path);
    if let Some(parent) = p.parent() {
        !parent.as_os_str().is_empty() && parent != p
    } else {
        false
    }
}

/// 判断指定远程/SFTP 路径是否可以向上返回上一级目录 (到达根目录 / 时返回 false)
pub(crate) fn can_navigate_remote_up(path: &str, is_local_session: bool) -> bool {
    if is_local_session {
        can_navigate_local_up(path)
    } else {
        let trimmed = path.trim().trim_end_matches('/');
        !trimmed.is_empty() && trimmed != "/"
    }
}

pub(crate) fn sync_file_explorer_ui(window: &AppWindow, ctx: &AppContext) {
    // 1. 同步左侧本地 Tab 列表
    sync_local_tabs_only(window, ctx);

    // 2. 同步右侧远程 Tab 列表
    sync_remote_tabs_only(window, ctx);

    // 3. 同步当前路径
    let local_path = ctx.local_current_path.borrow().clone();
    let remote_path = ctx.remote_current_path.borrow().clone();
    let fb = window.global::<FilesBridge>();
    fb.set_local_current_path(local_path.as_str().into());
    fb.set_remote_current_path(remote_path.as_str().into());

    // 4. 同步文件列表 (支持显示隐藏文件与排除过滤黑名单)
    let sb = window.global::<crate::generated::SettingsBridge>();
    let show_hidden = sb.get_setting_sftp_show_hidden();
    let excludes_str = sb.get_setting_sftp_exclude_patterns().to_string();
    let exclude_list: Vec<String> = excludes_str
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let is_excluded = |name: &str, is_hidden: bool| -> bool {
        if !show_hidden && (is_hidden || name.starts_with('.')) {
            return true;
        }
        if !exclude_list.is_empty() {
            let lower_name = name.to_lowercase();
            for pattern in &exclude_list {
                if pattern.starts_with('*') && pattern.len() > 1 {
                    if lower_name.ends_with(&pattern[1..]) {
                        return true;
                    }
                } else if lower_name == *pattern || lower_name.contains(pattern.as_str()) {
                    return true;
                }
            }
        }
        false
    };

    let local_items: Vec<SlintFileItemData> = ctx
        .local_file_nodes
        .borrow()
        .iter()
        .filter(|item| !is_excluded(&item.name, item.is_hidden))
        .map(map_file_item_to_ui)
        .collect();
    let remote_items: Vec<SlintFileItemData> = ctx
        .remote_file_nodes
        .borrow()
        .iter()
        .filter(|item| !is_excluded(&item.name, item.is_hidden))
        .map(map_file_item_to_ui)
        .collect();

    let local_model = slint::ModelRc::from(Rc::new(slint::VecModel::from(local_items)));
    let remote_model = slint::ModelRc::from(Rc::new(slint::VecModel::from(remote_items)));
    fb.set_local_files(local_model);
    fb.set_remote_files(remote_model);

    // 5. 同步历史导航前进/后退/上级使能状态
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

    let is_rem_local = is_remote_tab_local(ctx);
    let local_can_up = can_navigate_local_up(&local_path);
    let remote_can_up = can_navigate_remote_up(&remote_path, is_rem_local);

    fb.set_local_can_go_back(local_can_back);
    fb.set_local_can_go_forward(local_can_fwd);
    fb.set_remote_can_go_back(remote_can_back);
    fb.set_remote_can_go_forward(remote_can_fwd);
    fb.set_local_can_go_up(local_can_up);
    fb.set_remote_can_go_up(remote_can_up);

    // 6. 同步文件选择弹窗主机列表
    let file_hosts = build_file_launcher_hosts(ctx, "");
    fb.set_file_launcher_host_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(file_hosts))));

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
    let task_model = slint::ModelRc::from(Rc::new(slint::VecModel::from(tasks)));
    fb.set_transfer_tasks(task_model);
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

/// 递归复制本地目录全部子项
fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

/// 解析目标主机 SSH / SFTP 启动配置与关联凭据 (支持私钥、密码、跳板机与网络代理)
pub(crate) fn resolve_host_launch_config(ctx: &AppContext, host_id: &str) -> Option<SshLaunchConfig> {
    if host_id.is_empty() || host_id == "local" || host_id.starts_with("local-") {
        return None;
    }

    let tree = ctx.master_tree.read().unwrap();
    let host_node = tree.iter().find(|n| n.id == host_id && !n.is_group).cloned();
    drop(tree);

    let storage = ctx.core_state.storage();
    let host_rec_opt = crate::async_util::block_on(storage.hosts().get_by_id(host_id)).ok().flatten();

    let (host_addr, host_port) = if let Some(ref n) = host_node {
        (n.address.clone(), n.port as u16)
    } else if let Some(ref h) = host_rec_opt {
        (h.address.clone(), h.port)
    } else {
        return None;
    };

    let mut username_opt = host_node.as_ref().and_then(|n| n.effective_username.clone());
    let mut private_key_pem: Option<String> = None;
    let mut password_plaintext: Option<String> = None;
    let mut jump_host: Option<String> = None;
    let mut proxy_type: Option<String> = None;
    let mut proxy_host: Option<String> = None;
    let mut proxy_port: Option<u16> = None;
    let mut keepalive_interval = 30;
    let mut connect_timeout = 15;

    if let Some(ref h_rec) = host_rec_opt {
        // 1. 若绑定凭据，优先从凭据库提取解密后的密钥/密码与用户名
        if let Some(ref cred_id) = h_rec.credential_id {
            if !cred_id.is_empty() {
                if let Ok(Some(c_rec)) = crate::async_util::block_on(storage.credentials().get_by_id(cred_id)) {
                    if let Some(ref u) = c_rec.username {
                        if !u.trim().is_empty() {
                            username_opt = Some(u.trim().to_string());
                        }
                    }
                    match c_rec.cred_type {
                        CredentialType::Key => {
                            if !c_rec.secret_data.trim().is_empty() {
                                private_key_pem = Some(c_rec.secret_data.clone());
                            }
                        }
                        CredentialType::Password => {
                            if !c_rec.secret_data.is_empty() {
                                password_plaintext = Some(c_rec.secret_data.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // 2. 主机自身直录配置回退
        if username_opt.is_none() {
            if let Some(ref u) = h_rec.username {
                if !u.trim().is_empty() {
                    username_opt = Some(u.trim().to_string());
                }
            }
        }
        if private_key_pem.is_none() {
            if let Some(ref k) = h_rec.key_data {
                if !k.trim().is_empty() {
                    private_key_pem = Some(k.clone());
                }
            }
        }
        if password_plaintext.is_none() {
            if let Some(ref p) = h_rec.password {
                if !p.is_empty() {
                    password_plaintext = Some(p.clone());
                }
            }
        }

        // 3. 跳板机链路
        if let Some(ref jsummary) = h_rec.jump_chain_summary {
            if jsummary.starts_with("preset:") {
                let pid = jsummary.trim_start_matches("preset:");
                if let Ok(Some(tun)) = crate::async_util::block_on(storage.tunnels().get_by_id(pid)) {
                    let hops: Vec<String> = tun.jump_chain.iter()
                        .filter(|h| h.enabled)
                        .map(|h| {
                            let addr = if h.host_address.is_empty() { h.host_name.as_str() } else { h.host_address.as_str() };
                            if h.host_port == 22 || h.host_port == 0 {
                                addr.to_string()
                            } else {
                                format!("{}:{}", addr, h.host_port)
                            }
                        })
                        .collect();
                    if !hops.is_empty() {
                        jump_host = Some(hops.join(","));
                    } else if !tun.remote_host.is_empty() {
                        jump_host = Some(tun.remote_host);
                    }
                }
            } else if !jsummary.trim().is_empty() && !jsummary.starts_with("jump_hops:0") {
                jump_host = Some(jsummary.clone());
            }
        }

        // 4. 网络代理 (若主机未单独配置，则跟随全局出站代理策略)
        let explicit_proxy = h_rec.proxy_type.as_deref().unwrap_or("direct");
        if explicit_proxy != "direct" && !explicit_proxy.is_empty() {
            proxy_type = h_rec.proxy_type.clone();
            proxy_host = h_rec.proxy_host.clone();
            proxy_port = h_rec.proxy_port;
        } else if let Ok(cfg) = crate::async_util::block_on(storage.config().get()) {
            if cfg.global_proxy_mode == "custom" && !cfg.global_proxy_server.is_empty() {
                if let Some((proto, h, p)) = crate::terminal::ssh_config::parse_proxy_url(&cfg.global_proxy_server) {
                    proxy_type = Some(proto);
                    proxy_host = Some(h);
                    proxy_port = Some(p);
                }
            } else if cfg.global_proxy_mode == "system" {
                if let Some((proto, h, p)) = crate::terminal::ssh_config::get_system_proxy() {
                    proxy_type = Some(proto);
                    proxy_host = Some(h);
                    proxy_port = Some(p);
                }
            }
        }

        // 5. 保活与超时
        if h_rec.keepalive_interval > 0 {
            keepalive_interval = h_rec.keepalive_interval;
        }
        if h_rec.connect_timeout > 0 {
            connect_timeout = h_rec.connect_timeout;
        }
    }

    Some(SshLaunchConfig {
        host: host_addr,
        port: if host_port == 0 { 22 } else { host_port },
        username: username_opt,
        private_key_pem,
        password: password_plaintext,
        jump_host,
        proxy_type,
        proxy_host,
        proxy_port,
        keepalive_interval,
        connect_timeout,
        host_key_policy: Some("accept-new".to_string()),
    })
}

/// 获取当前右栏激活会话的目标主机 ID
pub(crate) fn get_active_remote_host_id(ctx: &AppContext) -> String {
    let act_id = ctx.active_remote_tab_id.borrow().clone();
    let tabs = ctx.remote_tabs.borrow();
    if let Some(t) = tabs.iter().find(|t| t.tab_id == act_id) {
        return t.host_id.clone();
    }
    // 备选：中央活跃终端会话
    let active_pane_id = ctx.active_pane_id.borrow().clone();
    let pane_groups = ctx.pane_groups.borrow();
    if let Some(group) = pane_groups.iter().find(|g| g.pane_id == active_pane_id).or_else(|| pane_groups.first()) {
        if let Some(s) = group.get_active_session() {
            return s.host_id.clone();
        }
    }
    String::new()
}

/// 判断当前右栏 SFTP / 远程文件上下文是否为本地会话
pub(crate) fn is_remote_tab_local(ctx: &AppContext) -> bool {
    let hid = get_active_remote_host_id(ctx);
    hid == "local" || hid.starts_with("local-") || hid.is_empty()
}

/// 扫描并更新右栏文件列表 (带错误校验与历史记录，自适应本地与远程会话)
pub(crate) fn try_refresh_remote_path(ctx: &AppContext, new_path: &str, push_history: bool) -> Result<(), String> {
    let act_id = ctx.active_remote_tab_id.borrow().clone();
    let is_local_session = is_remote_tab_local(ctx);

    if is_local_session {
        // 右栏当前激活会话为本地目录
        let target = if new_path == "~" || new_path.is_empty() {
            directories::BaseDirs::new()
                .map(|p| p.home_dir().to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        } else if new_path.starts_with("~/") || new_path.starts_with("~\\") {
            if let Some(home) = directories::BaseDirs::new().map(|p| p.home_dir().to_path_buf()) {
                home.join(&new_path[2..])
            } else {
                PathBuf::from(new_path)
            }
        } else {
            let p = PathBuf::from(new_path);
            if p.is_relative() {
                let curr = ctx.remote_current_path.borrow().clone();
                let base = if curr.is_empty() { PathBuf::from(".") } else { PathBuf::from(curr) };
                base.join(p)
            } else {
                p
            }
        };

        if !target.exists() {
            return Err(format!("路径不存在: {}", target.display()));
        }
        if !target.is_dir() {
            return Err(format!("不是一个有效的目录: {}", target.display()));
        }

        match scan_local_directory(&target) {
            Ok(nodes) => {
                let resolved_path = target.canonicalize()
                    .unwrap_or(target.clone())
                    .to_string_lossy()
                    .replace('\\', "/");
                *ctx.remote_current_path.borrow_mut() = resolved_path.clone();
                *ctx.remote_file_nodes.borrow_mut() = nodes;

                let mut tabs = ctx.remote_tabs.borrow_mut();
                if let Some(tab) = tabs.iter_mut().find(|t| t.tab_id == act_id) {
                    if push_history {
                        tab.push_path(resolved_path);
                    } else {
                        tab.current_path = resolved_path;
                    }
                } else {
                    let tid = if act_id.is_empty() { "rtab-local".to_string() } else { act_id.clone() };
                    let session = RemoteFileTabSession::new(
                        tid.clone(),
                        "local",
                        "本地终端",
                        "Local Filesystem",
                        resolved_path,
                    );
                    tabs.push(session);
                    drop(tabs);
                    *ctx.active_remote_tab_id.borrow_mut() = tid;
                }
                Ok(())
            }
            Err(e) => Err(format!("无法读取目录 [{}]: {}", target.display(), e)),
        }
    } else {
        // 远程会话模式：调用真实 SFTP 异步遍历目录
        let clean_path = if new_path == "~" || new_path.is_empty() { "/root" } else { new_path };
        *ctx.remote_current_path.borrow_mut() = clean_path.to_string();

        let (was_online, prev_path) = {
            let mut tabs = ctx.remote_tabs.borrow_mut();
            if let Some(tab) = tabs.iter_mut().find(|t| t.tab_id == act_id) {
                let online = tab.status == "online";
                let old_p = tab.current_path.clone();
                if push_history {
                    tab.push_path(clean_path.to_string());
                } else {
                    tab.current_path = clean_path.to_string();
                }
                // 仅当此前并非 online 状态时（如首次建连或重连），才置为 connecting 态
                if !online {
                    tab.status = "connecting".to_string();
                }
                tab.error_msg = None;
                (online, old_p)
            } else {
                (false, "/root".to_string())
            }
        };

        // 如果当前列表中包含 Windows 本地反斜杠/盘符路径，立即清空防止远程污染
        if ctx.remote_file_nodes.borrow().iter().any(|n| n.path.contains('\\') || n.path.contains(':')) {
            ctx.remote_file_nodes.borrow_mut().clear();
        }

        FILE_WINDOW_WEAK.with(|w_opt| {
            if let Some(w) = w_opt.borrow().as_ref().and_then(|w| w.upgrade()) {
                let fb = w.global::<FilesBridge>();
                if was_online {
                    // 已建立在线连接：绝不触发 is_remote_connecting 全屏遮罩，仅置 is_remote_loading 驱动轻量指示器
                    fb.set_is_remote_connecting(false);
                    fb.set_is_remote_loading(true);
                } else {
                    // 首次连接/重连：置 is_remote_connecting 展现初始化卡片，清空旧数据防止残留闪现
                    fb.set_is_remote_connecting(true);
                    fb.set_is_remote_loading(false);
                    ctx.remote_file_nodes.borrow_mut().clear();
                }
                fb.set_is_remote_error(false);
                fb.set_remote_error_msg("".into());
                sync_remote_tabs_only(&w, ctx);
            }
        });

        let host_id = get_active_remote_host_id(ctx);
        if let Some(launch_cfg) = resolve_host_launch_config(ctx, &host_id) {
            let path_clone = clean_path.to_string();
            let sftp_driver_opt = ctx.sftp_driver.clone();
            let sftp_svc = ctx.core_state.sftp();
            let hid_clone = host_id.clone();
            let tab_id_clone = act_id.clone();
            let cfg_clone = launch_cfg.clone();
            crate::async_util::spawn_async(async move {
                let res = if let Some(driver) = sftp_driver_opt {
                    // 1. 如果已有长连接，直接极速复用 (20ms~50ms)
                    let fast_res = driver.list_dir(&hid_clone, &path_clone).await;
                    match fast_res {
                        Ok(nodes) => {
                            tracing::info!(target: "smagical_ui::files", "⚡ [SFTP 长连接极速复用] 读取远程目录成功: {}", path_clone);
                            Ok(nodes)
                        }
                        Err(_) => {
                            // 2. 尝试建立并注册长连接到连接池
                            if let Ok(()) = driver.connect_and_register_with_config(&hid_clone, &cfg_clone).await {
                                match driver.list_dir(&hid_clone, &path_clone).await {
                                    Ok(nodes) => {
                                        tracing::info!(target: "smagical_ui::files", "✅ [SFTP 新长连接建立成功] 读取远程目录成功: {}", path_clone);
                                        Ok(nodes)
                                    }
                                    Err(e) => {
                                        tracing::warn!(target: "smagical_ui::files", "RusshSftpDriver list_dir 失败: {:?}，回退至命令行", e);
                                        crate::sftp::list_remote_directory(cfg_clone, path_clone.clone()).await
                                    }
                                }
                            } else {
                                // 3. 握手失败，平滑降级到基于命令行子进程的方式
                                tracing::warn!(target: "smagical_ui::files", "RusshSftpDriver 建连失败，平滑降级至 OpenSSH 进程: {}", path_clone);
                                crate::sftp::list_remote_directory(cfg_clone, path_clone.clone()).await
                            }
                        }
                    }
                } else {
                    let fallback_res = sftp_svc.list_dir(&hid_clone, &path_clone).await;
                    match fallback_res {
                        Ok(nodes) => Ok(nodes),
                        Err(_) => crate::sftp::list_remote_directory(cfg_clone, path_clone.clone()).await,
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    FILE_APP_CTX.with(|cell| {
                        if let Some(ctx) = cell.borrow().as_ref() {
                            match res {
                                Ok(nodes) => {
                                    let mut tabs = ctx.remote_tabs.borrow_mut();
                                    if let Some(tab) = tabs.iter_mut().find(|t| t.tab_id == tab_id_clone) {
                                        tab.status = "online".to_string();
                                        tab.error_msg = None;
                                    }
                                    drop(tabs);
                                    *ctx.remote_file_nodes.borrow_mut() = nodes;
                                    FILE_WINDOW_WEAK.with(|w_opt| {
                                        if let Some(w) = w_opt.borrow().as_ref().and_then(|w| w.upgrade()) {
                                            let fb = w.global::<FilesBridge>();
                                            fb.set_is_remote_connecting(false);
                                            fb.set_is_remote_loading(false);
                                            fb.set_is_remote_error(false);
                                            sync_file_explorer_ui(&w, ctx);
                                        }
                                    });
                                }
                                Err(e) => {
                                    let err_msg = e.to_string();
                                    FILE_WINDOW_WEAK.with(|w_opt| {
                                        if let Some(w) = w_opt.borrow().as_ref().and_then(|w| w.upgrade()) {
                                            let fb = w.global::<FilesBridge>();
                                            fb.set_is_remote_connecting(false);
                                            fb.set_is_remote_loading(false);

                                            if was_online {
                                                // 之前会话已在线，单次列目录失败（如权限拒绝或目录不存在）不破坏整个会话
                                                let mut tabs = ctx.remote_tabs.borrow_mut();
                                                if let Some(tab) = tabs.iter_mut().find(|t| t.tab_id == tab_id_clone) {
                                                    tab.status = "online".to_string();
                                                    tab.current_path = prev_path.clone();
                                                }
                                                drop(tabs);
                                                *ctx.remote_current_path.borrow_mut() = prev_path;
                                                sync_file_explorer_ui(&w, ctx);
                                                ctx.notify_error("SFTP 目录无法访问", err_msg);
                                            } else {
                                                // 首次建连失败或非在线重连失败，转为 error 状态展示重试
                                                let mut tabs = ctx.remote_tabs.borrow_mut();
                                                if let Some(tab) = tabs.iter_mut().find(|t| t.tab_id == tab_id_clone) {
                                                    tab.status = "error".to_string();
                                                    tab.error_msg = Some(err_msg.clone());
                                                }
                                                drop(tabs);
                                                fb.set_is_remote_error(true);
                                                fb.set_remote_error_msg(err_msg.clone().into());
                                                sync_file_explorer_ui(&w, ctx);
                                                ctx.notify_error("SFTP 连接失败", err_msg);
                                            }
                                        }
                                    });
                                    tracing::error!(target: "smagical_ui::files", "远程列目录失败 [{}]: {:?}", path_clone, e);
                                }
                            }
                        }
                    });
                });
            });
        } else {
            let err_msg = format!("未找到主机 [{}] 的连接配置", host_id);
            let mut tabs = ctx.remote_tabs.borrow_mut();
            if let Some(tab) = tabs.iter_mut().find(|t| t.tab_id == act_id) {
                tab.status = "error".to_string();
                tab.error_msg = Some(err_msg.clone());
            }
            drop(tabs);
            FILE_WINDOW_WEAK.with(|w_opt| {
                if let Some(w) = w_opt.borrow().as_ref().and_then(|w| w.upgrade()) {
                    let fb = w.global::<FilesBridge>();
                    fb.set_is_remote_connecting(false);
                    fb.set_is_remote_loading(false);
                    fb.set_is_remote_error(true);
                    fb.set_remote_error_msg(err_msg.clone().into());
                    sync_file_explorer_ui(&w, ctx);
                }
            });
            ctx.notify_error("SFTP 连接失败", err_msg);
        }
        Ok(())
    }
}

/// 执行真实文件/文件夹传输任务 (统一调度本地对拷与 SFTP 远程上传/下载)。
///
/// 封装完整的端到端传输执行管线：
/// 1. **同名冲突仲裁 (Conflict Policy)**：
///    - 读取用户的 `setting_sftp_conflict_policy`（`"skip"` 跳过 / `"rename"` 自动重命名 / `"overwrite"` 覆盖）；
///    - 根据传输流向（下载校验本地磁盘路径，上传校验当前远程节点名录）判定目标是否存在；
///    - 触发 `skip` 时中止任务并弹出通知；触发 `rename` 时自动按 `文件名 (新).扩展名` 重建目标路径。
/// 2. **目录递归扁平化展开**：
///    - 针对文件夹上传，调用 `scan_folder_recursive` 递归探测全部层级子项与字节大小，
///      自动构造具有父子级联折叠属性的树形任务组。
/// 3. **传输驱动调度**：
///    - 本地双盘模式（双侧均为本地目录）：调度多线程并发执行 `copy_dir_all` 或 `std::fs::copy`；
///    - SFTP 远程模式：调度底层 SSH 启动配置并拉起异步 SFTP 流式分块传输流水线。
///
/// # 参数
/// - `w_weak`: Slint UI 主窗口弱引用，用于跨线程更新传输队列并读取全局冲突策略；
/// - `ctx`: 全局应用共享上下文引用；
/// - `dir`: 传输流向（[`TransferDirection::Upload`] 为上传至远程，[`TransferDirection::Download`] 为下载至本地）；
/// - `source_path`: 待传输的源物理路径（本地绝对路径或远程绝对路径）；
/// - `filename`: 待传输项的文件或目录名称；
/// - `is_dir`: 是否为目录递归传输；
/// - `target_dir`: 目标对端的目标存放根目录路径。
pub(crate) fn execute_transfer_task(
    w_weak: slint::Weak<AppWindow>,
    ctx: &AppContext,
    dir: TransferDirection,
    source_path: String,
    filename: String,
    is_dir: bool,
    target_dir: String,
) {
    let (target_combined, filename) = {
        let raw_target = if target_dir.ends_with('/') || target_dir.ends_with('\\') {
            format!("{}{}", target_dir, filename)
        } else if target_dir.contains('\\') {
            format!("{}\\{}", target_dir, filename)
        } else {
            format!("{}/{}", target_dir, filename)
        };

        let conflict_policy = w_weak
            .upgrade()
            .map(|w| w.global::<crate::generated::SettingsBridge>().get_setting_sftp_conflict_policy().to_string())
            .unwrap_or_else(|| "ask".to_string());

        let target_exists = if dir == TransferDirection::Download {
            let p = std::path::PathBuf::from(&raw_target);
            p.exists()
        } else {
            ctx.remote_file_nodes.borrow().iter().any(|n| n.name == filename)
        };

        if target_exists {
            match conflict_policy.as_str() {
                "skip" => {
                    ctx.notify_info("跳过同名文件", format!("检测到「{}」已存在，根据冲突策略跳过传输", filename));
                    tracing::info!(target: "smagical_ui::files", "同名冲突策略触发 [skip]，跳过传输: {}", filename);
                    return;
                }
                "rename" => {
                    let (base, ext) = if let Some((b, e)) = filename.rsplit_once('.') {
                        (b, format!(".{}", e))
                    } else {
                        (filename.as_str(), String::new())
                    };
                    let new_name = format!("{} (新){}", base, ext);
                    let new_combined = if target_dir.ends_with('/') || target_dir.ends_with('\\') {
                        format!("{}{}", target_dir, new_name)
                    } else if target_dir.contains('\\') {
                        format!("{}\\{}", target_dir, new_name)
                    } else {
                        format!("{}/{}", target_dir, new_name)
                    };
                    tracing::info!(target: "smagical_ui::files", "同名冲突策略触发 [rename]，重命名为: {}", new_name);
                    (new_combined, new_name)
                }
                _ => (raw_target, filename),
            }
        } else {
            (raw_target, filename)
        }
    };

    let is_remote_local = is_remote_tab_local(ctx);
    let host_id = get_active_remote_host_id(ctx);

    if is_dir && dir == TransferDirection::Upload {
        let (file_count, total_bytes, sub_items) = scan_folder_recursive(&PathBuf::from(&source_path));
        let parent_task_id = format!("task-dir-{}", uuid::Uuid::new_v4().simple());
        let parent_task = TransferTask {
            id: parent_task_id.clone(),
            parent_id: None,
            session_id: "session-active".into(),
            filename: filename.clone(),
            is_dir: true,
            is_expanded: true,
            level: 0,
            item_count_text: format!("{} 项", file_count),
            source_path: source_path.clone(),
            target_path: target_combined.clone(),
            direction: dir,
            total_bytes,
            transferred_bytes: 0,
            speed_bytes_per_sec: 14_500_000,
            status: TransferStatus::Transferring,
            error_message: None,
        };

        let mut child_tasks = Vec::new();
        for (sub_name, sub_p, sub_sz, sub_rel) in sub_items {
            let child_task_id = format!("task-sub-{}", uuid::Uuid::new_v4().simple());
            let child_target = format!("{}/{}", target_combined.trim_end_matches(['/', '\\']), sub_rel);
            child_tasks.push(TransferTask {
                id: child_task_id,
                parent_id: Some(parent_task_id.clone()),
                session_id: "session-active".into(),
                filename: sub_name,
                is_dir: false,
                is_expanded: false,
                level: 1,
                item_count_text: "".into(),
                source_path: sub_p.to_string_lossy().to_string(),
                target_path: child_target,
                direction: dir,
                total_bytes: sub_sz,
                transferred_bytes: 0,
                speed_bytes_per_sec: 14_500_000,
                status: TransferStatus::Transferring,
                error_message: None,
            });
        }

        ctx.core_state.events().dispatch(&FileTransferStartedEvent {
            task_id: parent_task_id.clone(),
        });

        {
            let mut tasks = ctx.transfer_tasks.borrow_mut();
            tasks.push(parent_task);
            tasks.extend(child_tasks);
        }

        if let Some(w) = w_weak.upgrade() {
            w.global::<FilesBridge>().set_is_transfer_queue_expanded(true);
            sync_file_explorer_ui(&w, ctx);
        }

        if is_remote_local {
            let pid_clone = parent_task_id.clone();
            let src_clone = source_path.clone();
            let tgt_clone = target_combined.clone();

            crate::async_util::spawn_async(async move {
                let ok = tokio::task::spawn_blocking(move || {
                    copy_dir_all(&PathBuf::from(&src_clone), &PathBuf::from(&tgt_clone)).is_ok()
                }).await.unwrap_or(false);

                let _ = slint::invoke_from_event_loop(move || {
                    FILE_APP_CTX.with(|cell| {
                        if let Some(ctx) = cell.borrow().as_ref() {
                            let mut tasks = ctx.transfer_tasks.borrow_mut();
                            for t in tasks.iter_mut() {
                                if t.id == pid_clone || t.parent_id.as_deref() == Some(&pid_clone) {
                                    if ok {
                                        t.status = TransferStatus::Completed;
                                        t.transferred_bytes = t.total_bytes;
                                        t.speed_bytes_per_sec = 0;
                                    } else {
                                        t.status = TransferStatus::Failed;
                                        t.speed_bytes_per_sec = 0;
                                        t.error_message = Some("文件夹复制失败".into());
                                    }
                                }
                            }
                            drop(tasks);

                            let cur_rem = ctx.remote_current_path.borrow().clone();
                            let _ = try_refresh_remote_path(ctx, &cur_rem, false);
                            FILE_WINDOW_WEAK.with(|w_opt| {
                                if let Some(w) = w_opt.borrow().as_ref().and_then(|w| w.upgrade()) {
                                    sync_file_explorer_ui(&w, ctx);
                                }
                            });
                        }
                    });
                });
            });
        } else if let Some(launch_cfg) = resolve_host_launch_config(ctx, &host_id) {
            let pid_clone = parent_task_id.clone();
            let tgt_dir_clone = target_dir.clone();
            let filename_clone = filename.clone();
            let sftp_svc = ctx.core_state.sftp();
            let hid_clone = host_id.clone();

            crate::async_util::spawn_async(async move {
                let local_p = PathBuf::from(&source_path);
                let result = match sftp_svc.upload_file(&hid_clone, &source_path, &tgt_dir_clone, None).await {
                    Ok(()) => Ok(()),
                    Err(_) => crate::sftp::upload_path(launch_cfg, local_p, tgt_dir_clone.clone()).await,
                };

                let _ = slint::invoke_from_event_loop(move || {
                    FILE_APP_CTX.with(|cell| {
                        if let Some(ctx) = cell.borrow().as_ref() {
                            let mut tasks = ctx.transfer_tasks.borrow_mut();
                            for t in tasks.iter_mut() {
                                if t.id == pid_clone || t.parent_id.as_deref() == Some(&pid_clone) {
                                    match result {
                                        Ok(()) => {
                                            t.status = TransferStatus::Completed;
                                            t.transferred_bytes = t.total_bytes;
                                            t.speed_bytes_per_sec = 0;
                                        }
                                        Err(ref e) => {
                                            t.status = TransferStatus::Failed;
                                            t.speed_bytes_per_sec = 0;
                                            t.error_message = Some(e.to_string());
                                        }
                                    }
                                }
                            }
                            drop(tasks);

                            if result.is_ok() {
                                ctx.notify_info("上传完成", format!("文件夹已上传: {}", filename_clone));
                            } else {
                                ctx.notify_error("上传失败", "文件夹上传失败");
                            }

                            let _ = try_refresh_remote_path(ctx, &tgt_dir_clone, false);
                            FILE_WINDOW_WEAK.with(|w_opt| {
                                if let Some(w) = w_opt.borrow().as_ref().and_then(|w| w.upgrade()) {
                                    sync_file_explorer_ui(&w, ctx);
                                }
                            });
                        }
                    });
                });
            });
        } else {
            let mut tasks = ctx.transfer_tasks.borrow_mut();
            for t in tasks.iter_mut() {
                if t.id == parent_task_id || t.parent_id.as_deref() == Some(&parent_task_id) {
                    t.status = TransferStatus::Failed;
                    t.speed_bytes_per_sec = 0;
                    t.error_message = Some("无法解析目标主机 SSH 凭据".into());
                }
            }
            drop(tasks);
            ctx.notify_error("传输失败", "无法解析目标主机 SSH 启动配置与凭据");
            if let Some(w) = w_weak.upgrade() {
                sync_file_explorer_ui(&w, ctx);
            }
        }
        return;
    }

    // 单文件传输或文件夹下载
    let total_bytes = if dir == TransferDirection::Upload {
        std::fs::metadata(&source_path).map(|m| m.len()).unwrap_or(1024 * 1024)
    } else {
        ctx.remote_file_nodes.borrow().iter()
            .find(|n| n.path == source_path || n.name == filename)
            .map(|n| n.size)
            .unwrap_or(2 * 1024 * 1024)
    };

    let task_id = format!("task-{}", uuid::Uuid::new_v4().simple());
    let task = TransferTask {
        id: task_id.clone(),
        parent_id: None,
        session_id: "session-active".into(),
        filename: filename.clone(),
        is_dir,
        is_expanded: false,
        level: 0,
        item_count_text: if is_dir { "1 项".into() } else { "".into() },
        source_path: source_path.clone(),
        target_path: target_combined.clone(),
        direction: dir,
        total_bytes: if total_bytes == 0 { 1024 } else { total_bytes },
        transferred_bytes: 0,
        speed_bytes_per_sec: 12_500_000,
        status: TransferStatus::Transferring,
        error_message: None,
    };

    ctx.core_state.events().dispatch(&FileTransferStartedEvent {
        task_id: task_id.clone(),
    });

    ctx.transfer_tasks.borrow_mut().push(task);
    if let Some(w) = w_weak.upgrade() {
        w.global::<FilesBridge>().set_is_transfer_queue_expanded(true);
        sync_file_explorer_ui(&w, ctx);
    }

    if is_remote_local {
        let tid_clone = task_id.clone();
        let src_clone = source_path.clone();
        let tgt_clone = target_combined.clone();

        crate::async_util::spawn_async(async move {
            let ok = tokio::task::spawn_blocking(move || {
                if is_dir {
                    copy_dir_all(&PathBuf::from(&src_clone), &PathBuf::from(&tgt_clone)).is_ok()
                } else {
                    std::fs::copy(&src_clone, &tgt_clone).is_ok()
                }
            }).await.unwrap_or(false);

            let _ = slint::invoke_from_event_loop(move || {
                FILE_APP_CTX.with(|cell| {
                    if let Some(ctx) = cell.borrow().as_ref() {
                        let mut tasks = ctx.transfer_tasks.borrow_mut();
                        if let Some(t) = tasks.iter_mut().find(|t| t.id == tid_clone) {
                            if ok {
                                t.status = TransferStatus::Completed;
                                t.transferred_bytes = t.total_bytes;
                                t.speed_bytes_per_sec = 0;
                            } else {
                                t.status = TransferStatus::Failed;
                                t.speed_bytes_per_sec = 0;
                                t.error_message = Some("本地文件复制失败".into());
                            }
                        }
                        drop(tasks);

                        let cur_loc = ctx.local_current_path.borrow().clone();
                        refresh_local_path(ctx, &cur_loc);
                        let cur_rem = ctx.remote_current_path.borrow().clone();
                        let _ = try_refresh_remote_path(ctx, &cur_rem, false);

                        FILE_WINDOW_WEAK.with(|w_opt| {
                            if let Some(w) = w_opt.borrow().as_ref().and_then(|w| w.upgrade()) {
                                sync_file_explorer_ui(&w, ctx);
                            }
                        });
                    }
                });
            });
        });
    } else if let Some(launch_cfg) = resolve_host_launch_config(ctx, &host_id) {
        let tid_clone = task_id.clone();
        let tgt_dir_clone = target_dir.clone();
        let filename_clone = filename.clone();
        let sftp_svc = ctx.core_state.sftp();
        let hid_clone = host_id.clone();

        crate::async_util::spawn_async(async move {
            let result = match dir {
                TransferDirection::Upload => {
                    let local_p = PathBuf::from(&source_path);
                    match sftp_svc.upload_file(&hid_clone, &source_path, &tgt_dir_clone, None).await {
                        Ok(()) => Ok(()),
                        Err(_) => crate::sftp::upload_path(launch_cfg, local_p, tgt_dir_clone.clone()).await,
                    }
                }
                TransferDirection::Download => {
                    let local_p = PathBuf::from(&tgt_dir_clone);
                    match sftp_svc.download_file(&hid_clone, &source_path, &tgt_dir_clone, None).await {
                        Ok(()) => Ok(()),
                        Err(_) => crate::sftp::download_path(launch_cfg, source_path.clone(), local_p).await,
                    }
                }
            };

            let _ = slint::invoke_from_event_loop(move || {
                FILE_APP_CTX.with(|cell| {
                    if let Some(ctx) = cell.borrow().as_ref() {
                        let mut tasks = ctx.transfer_tasks.borrow_mut();
                        if let Some(t) = tasks.iter_mut().find(|t| t.id == tid_clone) {
                            match result {
                                Ok(()) => {
                                    t.status = TransferStatus::Completed;
                                    t.transferred_bytes = t.total_bytes;
                                    t.speed_bytes_per_sec = 0;
                                    ctx.notify_info(
                                        if dir == TransferDirection::Upload { "上传完成" } else { "下载完成" },
                                        format!("{}: {}", if dir == TransferDirection::Upload { "已上传" } else { "已下载" }, filename_clone),
                                    );
                                }
                                Err(ref e) => {
                                    t.status = TransferStatus::Failed;
                                    t.speed_bytes_per_sec = 0;
                                    t.error_message = Some(e.to_string());
                                    ctx.notify_error(
                                        if dir == TransferDirection::Upload { "上传失败" } else { "下载失败" },
                                        format!("传输失败: {:?}", e),
                                    );
                                }
                            }
                        }
                        drop(tasks);

                        match dir {
                            TransferDirection::Upload => {
                                let _ = try_refresh_remote_path(ctx, &tgt_dir_clone, false);
                            }
                            TransferDirection::Download => {
                                refresh_local_path(ctx, &tgt_dir_clone);
                            }
                        }

                        FILE_WINDOW_WEAK.with(|w_opt| {
                            if let Some(w) = w_opt.borrow().as_ref().and_then(|w| w.upgrade()) {
                                sync_file_explorer_ui(&w, ctx);
                            }
                        });
                    }
                });
            });
        });
    } else {
        let mut tasks = ctx.transfer_tasks.borrow_mut();
        if let Some(t) = tasks.iter_mut().find(|t| t.id == task_id) {
            t.status = TransferStatus::Failed;
            t.speed_bytes_per_sec = 0;
            t.error_message = Some("无法解析目标主机 SSH 凭据".into());
        }
        drop(tasks);
        ctx.notify_error("传输失败", "无法解析目标主机 SSH 启动配置与凭据");
        if let Some(w) = w_weak.upgrade() {
            sync_file_explorer_ui(&w, ctx);
        }
    }
}

/// 扫描并更新右栏文件列表 (忽略错误并记录历史)
pub(crate) fn refresh_remote_path(ctx: &AppContext, new_path: &str) {
    let _ = try_refresh_remote_path(ctx, new_path, true);
}

/// 当右栏伴生抽屉切换至 SFTP 或活跃终端主机发生变更时，同步刷新 SFTP 抽屉真实数据
pub(crate) fn sync_sftp_drawer_for_host(
    window: &AppWindow,
    ctx: &AppContext,
    host_id: &str,
    host_name: &str,
) {
    let is_local = host_id == "local" || host_id.starts_with("local-") || host_id.is_empty();

    if is_local {
        // 1. 确保在 remote_tabs 中有对应的本地文件会话
        let act_id = ctx.active_remote_tab_id.borrow().clone();
        let mut tabs = ctx.remote_tabs.borrow_mut();
        let has_matching_tab = tabs.iter().any(|t| t.tab_id == act_id && (t.host_id == "local" || t.host_id.starts_with("local-")));

        if !has_matching_tab {
            if let Some(existing) = tabs.iter().find(|t| t.host_id == "local" || t.host_id.starts_with("local-")) {
                *ctx.active_remote_tab_id.borrow_mut() = existing.tab_id.clone();
            } else {
                let tid = format!("rtab-{}", if host_id.is_empty() { "local" } else { host_id });
                let home_path = directories::BaseDirs::new()
                    .map(|p| p.home_dir().to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string());
                let session = RemoteFileTabSession::new(
                    tid.clone(),
                    if host_id.is_empty() { "local" } else { host_id },
                    if host_name.is_empty() { "本地终端" } else { host_name },
                    "Local Filesystem",
                    home_path,
                );
                tabs.push(session);
                *ctx.active_remote_tab_id.borrow_mut() = tid;
            }
        }
        drop(tabs);

        // 2. 获取当前路径或默认主目录 (过滤远程 Unix 路径)
        let current_p = ctx.remote_current_path.borrow().clone();
        let target_p = if current_p.is_empty() || !std::path::Path::new(&current_p).exists() || current_p.starts_with('/') {
            directories::BaseDirs::new()
                .map(|p| p.home_dir().to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        } else {
            current_p
        };

        // 3. 扫描真实本地文件并同步 UI
        let _ = try_refresh_remote_path(ctx, &target_p, false);
        sync_file_explorer_ui(window, ctx);
        tracing::info!(target: "smagical_ui::files", "右栏 SFTP 伴生抽屉成功同步本地终端真实目录: {}", target_p);
    } else {
        // 远程主机模式
        let act_id = ctx.active_remote_tab_id.borrow().clone();
        let mut tabs = ctx.remote_tabs.borrow_mut();
        let has_matching_tab = tabs.iter().any(|t| t.tab_id == act_id && t.host_id == host_id);

        if !has_matching_tab {
            if let Some(existing) = tabs.iter().find(|t| t.host_id == host_id) {
                *ctx.active_remote_tab_id.borrow_mut() = existing.tab_id.clone();
            } else {
                let tid = format!("rtab-{}", host_id);
                let mut session = RemoteFileTabSession::new(
                    tid.clone(),
                    host_id,
                    host_name,
                    "Remote Host",
                    "/root",
                );
                session.status = "connecting".into();
                tabs.push(session);
                *ctx.active_remote_tab_id.borrow_mut() = tid;
            }
        }
        drop(tabs);

        // 远程模式：严禁读取或残留本地 Windows 路径，且清除残留的本地文件列表防止闪现
        let current_p = ctx.remote_current_path.borrow().clone();
        let target_p = if current_p.is_empty() || current_p.contains(':') || !current_p.starts_with('/') {
            "/root".to_string()
        } else {
            current_p
        };
        if ctx.remote_file_nodes.borrow().iter().any(|n| n.path.contains('\\') || n.path.contains(':')) {
            ctx.remote_file_nodes.borrow_mut().clear();
        }
        let _ = try_refresh_remote_path(ctx, &target_p, false);
        sync_file_explorer_ui(window, ctx);
        tracing::info!(target: "smagical_ui::files", "右栏 SFTP 伴生抽屉切换至远程主机: {}", host_id);
    }
}




/// 注册双盘文件管理与 SFTP 视图回调
pub(crate) fn register_file_handlers(window: &AppWindow, ctx: &AppContext) {
    FILE_APP_CTX.with(|cell| {
        *cell.borrow_mut() = Some(ctx.clone());
    });
    FILE_WINDOW_WEAK.with(|cell| {
        *cell.borrow_mut() = Some(window.as_weak());
    });

    let fb = window.global::<FilesBridge>();

    // -------------------------------------------------------------------------
    // 1. 左侧本地 Tab 栏交互回调
    // -------------------------------------------------------------------------
    // 1.1 选择本地 Tab
    let window_weak = window.as_weak();
    let ctx_select_loc = ctx.clone();
    fb.on_select_local_tab(move |tab_id| {
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
    fb.on_close_local_tab(move |tab_id| {
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
    fb.on_new_local_tab(move || {
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
    fb.on_select_remote_tab(move |tab_id| {
        if let Some(w) = window_weak.upgrade() {
            let tid = tab_id.to_string();
            *ctx_select_rem.active_remote_tab_id.borrow_mut() = tid.clone();

            let tabs = ctx_select_rem.remote_tabs.borrow();
            let (rem, status) = if let Some(tab) = tabs.iter().find(|t| t.tab_id == tid) {
                (tab.current_path.clone(), tab.status.clone())
            } else {
                (String::new(), "online".to_string())
            };
            drop(tabs);
            if !rem.is_empty() && status != "error" {
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
    fb.on_close_remote_tab(move |tab_id| {
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
    fb.on_new_remote_tab(move || {
        if let Some(w) = window_weak.upgrade() {
            let hosts = build_file_launcher_hosts(&ctx_new_rem, "");
            w.global::<FilesBridge>().set_file_launcher_host_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(hosts))));
            w.global::<FilesBridge>().set_is_file_host_modal_open(true);
            tracing::info!(target: "smagical_ui::files", "打开文件会话选择与 SFTP 连接弹窗");
        }
    });

    // -------------------------------------------------------------------------
    // 3. 从左侧主机栏双击主机：只会创建到右侧远程 Tab 栏
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let ctx_open_host = ctx.clone();
    fb.on_open_host_files(move |host_id| {
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

            // 查询主机元数据并新建右侧远程 Tab (优先从内存 master_tree 中快速读取)
            let tree = ctx_open_host.master_tree.read().unwrap();
            let target_node = tree.iter().find(|n| n.id == h_id && !n.is_group).cloned();
            drop(tree);

            let (h_name, h_addr) = if let Some(n) = target_node {
                (n.name, if n.port > 0 { format!("{}:{}", n.address, n.port) } else { n.address })
            } else {
                (format!("Host ({})", h_id), "127.0.0.1:22".into())
            };

            let tab_id = format!("rtab-{}", tabs.len() + 1);
            let mut session = RemoteFileTabSession::new(tab_id.clone(), h_id.clone(), h_name, h_addr, "/root");
            session.status = "connecting".into();
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
    fb.on_navigate_local_path(move |path| {
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
                    w.global::<FilesBridge>().set_local_current_path(current_valid.into());
                    tracing::warn!(target: "smagical_ui::files", "本地路径跳转失败: {}", p_str);
                }
            }
        }
    });

    // 4.2 本地历史后退
    let window_weak = window.as_weak();
    let ctx_back_loc = ctx.clone();
    fb.on_navigate_local_back(move || {
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
    fb.on_navigate_local_forward(move || {
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
    fb.on_navigate_local_up(move || {
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
    fb.on_navigate_remote_path(move |path| {
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
                    w.global::<FilesBridge>().set_remote_current_path(current_valid.into());
                    tracing::warn!(target: "smagical_ui::files", "远程路径跳转失败: {}", p_str);
                }
            }
        }
    });

    // 4.6 远程历史后退
    let window_weak = window.as_weak();
    let ctx_back_rem = ctx.clone();
    fb.on_navigate_remote_back(move || {
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
    fb.on_navigate_remote_forward(move || {
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
    fb.on_navigate_remote_up(move || {
        if let Some(w) = window_weak.upgrade() {
            let act_id = ctx_up_remote.active_remote_tab_id.borrow().clone();
            let is_local_session = is_remote_tab_local(&ctx_up_remote);
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
            } else {
                let trimmed = current.trim_end_matches('/');
                let parent_str = if trimmed.is_empty() || trimmed == "/" {
                    "/"
                } else if let Some(pos) = trimmed.rfind('/') {
                    if pos == 0 { "/" } else { &trimmed[..pos] }
                } else {
                    "/"
                };
                if parent_str != current {
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
        }
    });



    // 4.5 打开本地文件/文件夹 (双击)
    let window_weak = window.as_weak();
    let ctx_open_local = ctx.clone();
    fb.on_open_local_item(move |path, is_dir| {

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
    fb.on_open_remote_item(move |path, is_dir| {
        if let Some(w) = window_weak.upgrade() {
            let p_str = path.to_string();
            if is_dir {
                refresh_remote_path(&ctx_open_remote, &p_str);
                sync_file_explorer_ui(&w, &ctx_open_remote);
                tracing::info!(target: "smagical_ui::files", "进入远程目录: {}", p_str);
            } else {
                if is_remote_tab_local(&ctx_open_remote) {
                    #[cfg(target_os = "windows")]
                    let _ = std::process::Command::new("explorer").arg(&p_str).spawn();
                    #[cfg(not(target_os = "windows"))]
                    let _ = std::process::Command::new("xdg-open").arg(&p_str).spawn();
                } else {
                    let host_id = get_active_remote_host_id(&ctx_open_remote);
                    if let Some(launch_cfg) = resolve_host_launch_config(&ctx_open_remote, &host_id) {
                        let p_clone = p_str.clone();
                        let storage_edit = ctx_open_remote.core_state.storage().clone();
                        crate::async_util::spawn_async(async move {
                            let temp_dir = std::env::temp_dir().join("smalux_sftp_cache");
                            let _ = std::fs::create_dir_all(&temp_dir);
                            let fname = std::path::Path::new(&p_clone)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("remote_file");
                            let local_cache = temp_dir.join(fname);

                            if let Ok(()) = crate::sftp::download_path(launch_cfg, p_clone.clone(), temp_dir.clone()).await {
                                if let Ok(cfg) = storage_edit.config().get().await {
                                    match cfg.sftp_editor_mode.as_str() {
                                        "custom" => {
                                            let custom_cmd = cfg.sftp_custom_editor.trim();
                                            if !custom_cmd.is_empty() {
                                                let _ = std::process::Command::new(custom_cmd).arg(&local_cache).spawn();
                                            } else {
                                                #[cfg(target_os = "windows")]
                                                let _ = std::process::Command::new("explorer").arg(&local_cache).spawn();
                                                #[cfg(not(target_os = "windows"))]
                                                let _ = std::process::Command::new("xdg-open").arg(&local_cache).spawn();
                                            }
                                        }
                                        _ => {
                                            #[cfg(target_os = "windows")]
                                            let _ = std::process::Command::new("explorer").arg(&local_cache).spawn();
                                            #[cfg(not(target_os = "windows"))]
                                            let _ = std::process::Command::new("xdg-open").arg(&local_cache).spawn();
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }
        }
    });

    // 4.5 刷新本地文件列表
    let window_weak = window.as_weak();
    let ctx_ref_local = ctx.clone();
    fb.on_refresh_local(move || {
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
    fb.on_refresh_remote(move || {
        if let Some(w) = window_weak.upgrade() {
            let p = ctx_ref_remote.remote_current_path.borrow().clone();
            refresh_remote_path(&ctx_ref_remote, &p);
            sync_file_explorer_ui(&w, &ctx_ref_remote);
            tracing::info!(target: "smagical_ui::files", "刷新远程文件目录: {}", p);
        }
    });

    // 4.7 上传系统文件 (弹窗系统文件选择，支持多选并真正加入传输任务队列)
    let window_weak = window.as_weak();
    fb.on_upload_file(move || {
        let is_en = window_weak.upgrade().map(|w| w.global::<WindowBridge>().get_current_language() == "en-US").unwrap_or(false);
        let w_weak = window_weak.clone();
        crate::async_util::spawn_async(async move {
            let dialog_title = if is_en { "Select files to upload (multiple selection supported)" } else { "选择要上传的文件 (支持多选)" };
            let picked = tokio::task::spawn_blocking(move || {
                rfd::FileDialog::new()
                    .set_title(dialog_title)
                    .pick_files()
            }).await.unwrap_or(None);
            if let Some(files) = picked {
                if files.is_empty() { return; }
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = w_weak.upgrade() {
                        FILE_APP_CTX.with(|cell| {
                            if let Some(ctx_up) = cell.borrow().as_ref() {
                                let rem_path = ctx_up.remote_current_path.borrow().clone();
                                for path in files {
                                    let filename = path.file_name()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "uploaded_file.bin".to_string());
                                    execute_transfer_task(
                                        w.as_weak(),
                                        ctx_up,
                                        TransferDirection::Upload,
                                        path.to_string_lossy().to_string(),
                                        filename,
                                        false,
                                        rem_path.clone(),
                                    );
                                }
                            }
                        });
                    }
                });
            }
        });
    });

    // 4.7b 上传系统文件夹 (弹窗系统文件夹选择，递归扫描并加入真实传输任务队列)
    let window_weak_folder = window.as_weak();
    fb.on_upload_folder(move || {
        let is_en = window_weak_folder.upgrade().map(|w| w.global::<WindowBridge>().get_current_language() == "en-US").unwrap_or(false);
        let w_weak = window_weak_folder.clone();
        crate::async_util::spawn_async(async move {
            let dialog_title = if is_en { "Select folder to upload" } else { "选择要上传的文件夹" };
            let picked = tokio::task::spawn_blocking(move || {
                rfd::FileDialog::new()
                    .set_title(dialog_title)
                    .pick_folder()
            }).await.unwrap_or(None);
            if let Some(dir_path) = picked {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = w_weak.upgrade() {
                        FILE_APP_CTX.with(|cell| {
                            if let Some(ctx_up) = cell.borrow().as_ref() {
                                let rem_path = ctx_up.remote_current_path.borrow().clone();
                                let dir_name = dir_path.file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "folder".to_string());
                                execute_transfer_task(
                                    w.as_weak(),
                                    ctx_up,
                                    TransferDirection::Upload,
                                    dir_path.to_string_lossy().to_string(),
                                    dir_name,
                                    true,
                                    rem_path,
                                );
                            }
                        });
                    }
                });
            }
        });
    });

    // 4.8 下载选中文件 (Remote -> Local)
    let window_weak = window.as_weak();
    let ctx_download = ctx.clone();
    fb.on_download_file(move || {
        let loc_path = ctx_download.local_current_path.borrow().clone();
        let rem_path = ctx_download.remote_current_path.borrow().clone();
        let filename = rem_path.split('/').next_back().filter(|s| !s.is_empty()).unwrap_or("downloaded_file.bin").to_string();
        execute_transfer_task(
            window_weak.clone(),
            &ctx_download,
            TransferDirection::Download,
            rem_path,
            filename,
            false,
            loc_path,
        );
    });

    // 4.9 清空已完成或失败的传输任务
    let window_weak = window.as_weak();
    let ctx_clear_trans = ctx.clone();
    fb.on_clear_completed_transfers(move || {
        if let Some(w) = window_weak.upgrade() {
            let mut tasks = ctx_clear_trans.transfer_tasks.borrow_mut();
            tasks.retain(|t| t.status == TransferStatus::Transferring || t.status == TransferStatus::Pending);
            drop(tasks);
            sync_file_explorer_ui(&w, &ctx_clear_trans);
            tracing::info!(target: "smagical_ui::files", "清空已完成的传输任务");
        }
    });

    // 4.10 触发拖拽文件/文件夹传输任务 (支持真实单文件与多文件夹嵌套树)
    let window_weak = window.as_weak();
    let ctx_start_trans = ctx.clone();
    fb.on_start_transfer_task(move |dir, source_path, filename, is_dir, target_dir| {
        let dir_enum = if dir == "download" { TransferDirection::Download } else { TransferDirection::Upload };
        execute_transfer_task(
            window_weak.clone(),
            &ctx_start_trans,
            dir_enum,
            source_path.to_string(),
            filename.to_string(),
            is_dir,
            target_dir.to_string(),
        );
    });

    // 4.11 折叠/展开传输队列中的文件夹任务
    let window_weak = window.as_weak();
    let ctx_toggle_trans = ctx.clone();
    fb.on_toggle_transfer_expand(move |task_id| {
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
    fb.on_transfer_action(move |action, task_id| {
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
    fb.on_file_action(move |action, is_remote, path, name, is_dir| {
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
                    let target_dir = if is_remote {
                        ctx_file_act.local_current_path.borrow().clone()
                    } else {
                        ctx_file_act.remote_current_path.borrow().clone()
                    };
                    execute_transfer_task(
                        window_weak.clone(),
                        &ctx_file_act,
                        dir_enum,
                        p_str,
                        n_str,
                        is_dir,
                        target_dir,
                    );
                }
                "create_folder" => {
                    if !is_remote {
                        let parent = if is_dir { std::path::PathBuf::from(&p_str) } else { std::path::PathBuf::from(&p_str).parent().unwrap_or(std::path::Path::new(".")).to_path_buf() };
                        let events = ctx_file_act.core_state.events().clone();
                        let w_weak = w.as_weak();
                        crate::async_util::spawn_async(async move {
                            let (target, ok) = tokio::task::spawn_blocking(move || {
                                let mut target = parent.join("新建文件夹");
                                let mut counter = 1;
                                while target.exists() {
                                    target = parent.join(format!("新建文件夹 ({})", counter));
                                    counter += 1;
                                }
                                let ok = std::fs::create_dir_all(&target).is_ok();
                                (target, ok)
                            }).await.unwrap_or_else(|_| (std::path::PathBuf::new(), false));

                            events.dispatch(&FileOperationCompletedEvent {
                                action: "create_folder".into(),
                                is_remote: false,
                                path: target.to_string_lossy().to_string(),
                                success: ok,
                            });

                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = w_weak.upgrade() {
                                    FILE_APP_CTX.with(|cell| {
                                        if let Some(ctx_ref) = cell.borrow().as_ref() {
                                            let cur_p = ctx_ref.local_current_path.borrow().clone();
                                            refresh_local_path(ctx_ref, &cur_p);
                                            sync_file_explorer_ui(&w, ctx_ref);
                                        }
                                    });
                                }
                            });
                        });
                    } else {
                        let parent = if is_dir {
                            p_str.clone()
                        } else if let Some(pos) = p_str.rfind('/') {
                            if pos == 0 { "/".to_string() } else { p_str[..pos].to_string() }
                        } else {
                            ctx_file_act.remote_current_path.borrow().clone()
                        };
                        let target_dir = format!("{}/新建文件夹", parent.trim_end_matches('/'));
                        let host_id = get_active_remote_host_id(&ctx_file_act);
                        if let Some(launch_cfg) = resolve_host_launch_config(&ctx_file_act, &host_id) {
                            let events = ctx_file_act.core_state.events().clone();
                            let w_weak = w.as_weak();
                            let target_clone = target_dir.clone();
                            crate::async_util::spawn_async(async move {
                                let ok = crate::sftp::create_remote_dir(launch_cfg, target_clone.clone()).await.is_ok();
                                events.dispatch(&FileOperationCompletedEvent {
                                    action: "create_folder".into(),
                                    is_remote: true,
                                    path: target_clone,
                                    success: ok,
                                });

                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(w) = w_weak.upgrade() {
                                        FILE_APP_CTX.with(|cell| {
                                            if let Some(ctx_ref) = cell.borrow().as_ref() {
                                                if ok {
                                                    ctx_ref.notify_info("已创建", "远程文件夹创建成功");
                                                } else {
                                                    ctx_ref.notify_error("创建失败", "远程文件夹创建失败");
                                                }
                                                let cur_p = ctx_ref.remote_current_path.borrow().clone();
                                                let _ = try_refresh_remote_path(ctx_ref, &cur_p, false);
                                                sync_file_explorer_ui(&w, ctx_ref);
                                            }
                                        });
                                    }
                                });
                            });
                        }
                    }
                }
                "create_file" => {
                    if !is_remote {
                        let parent = if is_dir { std::path::PathBuf::from(&p_str) } else { std::path::PathBuf::from(&p_str).parent().unwrap_or(std::path::Path::new(".")).to_path_buf() };
                        let events = ctx_file_act.core_state.events().clone();
                        let w_weak = w.as_weak();
                        crate::async_util::spawn_async(async move {
                            let (target, ok) = tokio::task::spawn_blocking(move || {
                                let mut target = parent.join("新建文本文档.txt");
                                let mut counter = 1;
                                while target.exists() {
                                    target = parent.join(format!("新建文本文档 ({}).txt", counter));
                                    counter += 1;
                                }
                                let ok = std::fs::File::create(&target).is_ok();
                                (target, ok)
                            }).await.unwrap_or_else(|_| (std::path::PathBuf::new(), false));

                            events.dispatch(&FileOperationCompletedEvent {
                                action: "create_file".into(),
                                is_remote: false,
                                path: target.to_string_lossy().to_string(),
                                success: ok,
                            });

                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = w_weak.upgrade() {
                                    FILE_APP_CTX.with(|cell| {
                                        if let Some(ctx_ref) = cell.borrow().as_ref() {
                                            let cur_p = ctx_ref.local_current_path.borrow().clone();
                                            refresh_local_path(ctx_ref, &cur_p);
                                            sync_file_explorer_ui(&w, ctx_ref);
                                        }
                                    });
                                }
                            });
                        });
                    } else {
                        let parent = if is_dir {
                            p_str.clone()
                        } else if let Some(pos) = p_str.rfind('/') {
                            if pos == 0 { "/".to_string() } else { p_str[..pos].to_string() }
                        } else {
                            ctx_file_act.remote_current_path.borrow().clone()
                        };
                        let target_file = format!("{}/新建文本文档.txt", parent.trim_end_matches('/'));
                        let host_id = get_active_remote_host_id(&ctx_file_act);
                        if let Some(launch_cfg) = resolve_host_launch_config(&ctx_file_act, &host_id) {
                            let events = ctx_file_act.core_state.events().clone();
                            let w_weak = w.as_weak();
                            let target_clone = target_file.clone();
                            crate::async_util::spawn_async(async move {
                                let ok = crate::sftp::create_remote_file(launch_cfg, target_clone.clone()).await.is_ok();
                                events.dispatch(&FileOperationCompletedEvent {
                                    action: "create_file".into(),
                                    is_remote: true,
                                    path: target_clone,
                                    success: ok,
                                });

                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(w) = w_weak.upgrade() {
                                        FILE_APP_CTX.with(|cell| {
                                            if let Some(ctx_ref) = cell.borrow().as_ref() {
                                                if ok {
                                                    ctx_ref.notify_info("已创建", "远程文件创建成功");
                                                } else {
                                                    ctx_ref.notify_error("创建失败", "远程文件创建失败");
                                                }
                                                let cur_p = ctx_ref.remote_current_path.borrow().clone();
                                                let _ = try_refresh_remote_path(ctx_ref, &cur_p, false);
                                                sync_file_explorer_ui(&w, ctx_ref);
                                            }
                                        });
                                    }
                                });
                            });
                        }
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
                        let events = ctx_file_act.core_state.events().clone();
                        let p_str_clone = p_str.clone();
                        let n_str_clone = n_str.clone();
                        let w_weak = w.as_weak();
                        crate::async_util::spawn_async(async move {
                            let ok = tokio::task::spawn_blocking({
                                let p_str = p_str_clone.clone();
                                move || {
                                    if is_dir {
                                        std::fs::remove_dir_all(&p_str).is_ok()
                                    } else {
                                        std::fs::remove_file(&p_str).is_ok()
                                    }
                                }
                            }).await.unwrap_or(false);

                            events.dispatch(&FileOperationCompletedEvent {
                                action: "delete".into(),
                                is_remote: false,
                                path: p_str_clone,
                                success: ok,
                            });

                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = w_weak.upgrade() {
                                    FILE_APP_CTX.with(|cell| {
                                        if let Some(ctx_ref) = cell.borrow().as_ref() {
                                            if ok {
                                                ctx_ref.notify_info("已删除", format!("已删除: {}", n_str_clone));
                                            }
                                            let cur_p = ctx_ref.local_current_path.borrow().clone();
                                            refresh_local_path(ctx_ref, &cur_p);
                                            sync_file_explorer_ui(&w, ctx_ref);
                                        }
                                    });
                                }
                            });
                        });
                    } else {
                        let host_id = get_active_remote_host_id(&ctx_file_act);
                        if let Some(launch_cfg) = resolve_host_launch_config(&ctx_file_act, &host_id) {
                            let events = ctx_file_act.core_state.events().clone();
                            let p_str_clone = p_str.clone();
                            let n_str_clone = n_str.clone();
                            let w_weak = w.as_weak();
                            crate::async_util::spawn_async(async move {
                                let ok = crate::sftp::remove_remote_path(launch_cfg, p_str_clone.clone()).await.is_ok();
                                events.dispatch(&FileOperationCompletedEvent {
                                    action: "delete".into(),
                                    is_remote: true,
                                    path: p_str_clone,
                                    success: ok,
                                });

                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(w) = w_weak.upgrade() {
                                        FILE_APP_CTX.with(|cell| {
                                            if let Some(ctx_ref) = cell.borrow().as_ref() {
                                                if ok {
                                                    ctx_ref.notify_info("已删除", format!("已删除远程项: {}", n_str_clone));
                                                } else {
                                                    ctx_ref.notify_error("删除失败", format!("无法删除远程项: {}", n_str_clone));
                                                }
                                                let cur_p = ctx_ref.remote_current_path.borrow().clone();
                                                let _ = try_refresh_remote_path(ctx_ref, &cur_p, false);
                                                sync_file_explorer_ui(&w, ctx_ref);
                                            }
                                        });
                                    }
                                });
                            });
                        }
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
    fb.on_copy_to_clipboard(move |text| {
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
    fb.on_filter_file_launcher(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let hosts = build_file_launcher_hosts(&ctx_filter_file, query.as_str());
            w.global::<FilesBridge>().set_file_launcher_host_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(hosts))));
        }
    });

    // 5.2 选取主机并自动连接 SFTP 会话 (或在右栏打开本地目录会话)
    let window_weak = window.as_weak();
    let ctx_open_fhost = ctx.clone();
    fb.on_open_file_host(move |host_id| {
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
            let tree = ctx_open_fhost.master_tree.read().unwrap();
            let target_node = tree.iter().find(|n| n.id == hid && !n.is_group).cloned();
            drop(tree);

            let (h_name, h_addr) = if let Some(n) = target_node {
                (n.name, if n.port > 0 { format!("{}:{}", n.address, n.port) } else { n.address })
            } else {
                (format!("Host ({})", hid), "127.0.0.1:22".into())
            };

            let tab_id = format!("rtab-{}", tabs.len() + 1);
            let mut session = RemoteFileTabSession::new(
                tab_id.clone(),
                hid.clone(),
                h_name,
                h_addr,
                "/root",
            );
            session.status = "connecting".into();
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
    fb.on_reorder_local_tab(move |from_idx: i32, to_idx: i32| {
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
    fb.on_reorder_remote_tab(move |from_idx: i32, to_idx: i32| {
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

    // -------------------------------------------------------------------------
    
    // 6. 目录树折叠展开
    fb.on_toggle_local_expand(|_| {});
    fb.on_toggle_remote_expand(|_| {});
}
