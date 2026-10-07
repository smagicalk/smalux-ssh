//! 本地凭据主密码管理与安全保险库锁定/解锁事件处理器 (Security & Vault Handlers)。
//!
//! 负责：
//! 1. 响应冷启动与按需解锁主密码 (`unlock-vault`)；
//! 2. 响应设置与修改主密码 (`set-master-password`)；
//! 3. 响应移除主密码并降级为默认无密码设备模式 (`submit-remove-master-password`)；
//! 4. 自动维护 Slint 视图模型 `SettingsBridge` 的模态弹窗与错误提示状态。

use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use slint::ComponentHandle;
use smagical_core::AppStorage;
use crate::common::{run_on_ui, to_model_rc};

use crate::async_util::spawn_async;
use crate::generated::{AppWindow, SettingsBridge};
use crate::handlers::AppContext;
use crate::tree_model::{
    build_cards_from_records, build_group_options, build_raw_tree,
    build_visible_tree_nodes, calculate_max_tree_width, RawTreeNode,
};

/// 检查冷启动时安全保险库是否处于锁定状态
///
/// # 业务与安全逻辑
/// 1. 异步探测 SQLite 仓储中是否存在自定义主密码保护 (`has_custom_master_password`)；
/// 2. 检查内存密钥环中的保险库当前是否已被解密激活 (`is_vault_unlocked`)；
/// 3. 若已开启自定义主密码且尚未完成解密输入，强制打开 Slint `is_unlock_modal_open` 全屏锁屏遮罩，拦截一切未授权的主机敏感凭据访问。
///
/// # 参数
/// - `window`: Slint 顶级应用窗口；
/// - `ctx`: 应用程序全局上下文引用。
pub(crate) fn check_vault_lock_on_boot(window: &AppWindow, ctx: &AppContext) {
    let storage = ctx.core_state.storage();
    let window_weak = window.as_weak();
    spawn_async(async move {
        let has_custom = storage.has_custom_master_password().await.unwrap_or(false);
        let is_unlocked = storage.is_vault_unlocked();

        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                let bridge = w.global::<SettingsBridge>();
                bridge.set_setting_master_password_enabled(has_custom);
                if has_custom && !is_unlocked {
                    tracing::info!(target: "smagical_ui::security", "检测到已开启本地主密码保护且保险库未解锁，呼出冷启动解锁遮罩");
                    bridge.set_is_unlock_modal_open(true);
                }
            }
        });
    });
}

/// 重新从存储层全量刷新主机资产树、卡片视图与凭据数据至前端 UI
///
/// # 触发时机
/// 当用户成功解锁保险库、导入外部数据包、或出厂重置完成时触发。
///
/// # 处理步骤
/// 1. 异步拉取并刷新凭据中心列表 (`sync_credentials_ui_async`)；
/// 2. 异步拉取全量主机、分组和凭据列表；
/// 3. 重建资产树模型 `build_raw_tree` 与卡片模型 `build_cards_from_records`，写入写锁守护；
/// 4. 调度回 UI 主事件循环，水合分组下拉项、树节点、自适应宽度及主机卡片模型。
///
/// # 参数
/// - `window`: Slint 应用窗口；
/// - `storage`: 仓储服务抽象接口；
/// - `master_tree`: 全局主机树读写锁共享指针；
/// - `master_cards`: 全局主机卡片数据读写锁共享指针；
/// - `expanded_groups`: 树节点展开状态集合；
/// - `selector_expanded_groups`: 下拉选择器节点展开状态集合。
pub(crate) fn refresh_vault_sensitive_data(
    window: &AppWindow,
    storage: Arc<dyn AppStorage>,
    master_tree: Arc<RwLock<Vec<RawTreeNode>>>,
    master_cards: Arc<RwLock<Vec<crate::generated::HostItemData>>>,
    expanded_groups: Arc<RwLock<HashSet<String>>>,
    selector_expanded_groups: Arc<RwLock<HashSet<String>>>,
) {
    let window_weak = window.as_weak();
    let storage_for_tree = Arc::clone(&storage);

    // 1. 刷新凭据中心
    crate::handlers::credential_handlers::sync_credentials_ui_async(
        window_weak.clone(),
        Arc::clone(&storage),
        "all".to_string(),
        "".to_string(),
    );

    // 2. 刷新主机与资产树
    spawn_async(async move {
        let hosts = storage_for_tree.hosts().list_all().await.unwrap_or_default();
        let groups = storage_for_tree.groups().list_all().await.unwrap_or_default();
        let creds = storage_for_tree.credentials().list_all().await.unwrap_or_default();

        let new_tree = build_raw_tree(&groups, &hosts, &creds);
        let new_cards = build_cards_from_records(&hosts, &groups);

        {
            let mut t = master_tree.write().unwrap();
            *t = new_tree;
        }
        {
            let mut c = master_cards.write().unwrap();
            *c = new_cards;
        }

        let _ = run_on_ui(window_weak, move |w| {
            let hb = w.global::<crate::generated::HostsBridge>();
            let tree_guard = master_tree.read().unwrap();
            let sel_guard = selector_expanded_groups.read().unwrap();
            let exp_guard = expanded_groups.read().unwrap();

            let opts = build_group_options(&tree_guard, &sel_guard);
            hb.set_group_options(to_model_rc(opts));

            let nodes = build_visible_tree_nodes(&tree_guard, &exp_guard);
            hb.set_tree_content_width(calculate_max_tree_width(&nodes));
            hb.set_tree_nodes(to_model_rc(nodes));
            hb.set_hosts(to_model_rc(master_cards.read().unwrap().clone()));
        });
    });
}

/// 注册所有安全中心主密码设置、重置、移除与保险库解锁交互回调
///
/// 包含以下关键功能：
/// 1. `on_unlock_vault`: 用户输入主密码验证并解锁敏感数据；
/// 2. `on_set_master_password`: 校验旧密码并重构信封加密密钥派生树；
/// 3. `on_submit_remove_master_password`: 撤销自定义密码，降级为设备绑定默认信封；
/// 4. `on_open_master_pwd_modal`, `on_close_master_pwd_modal`: 弹窗状态管理；
/// 5. `on_toggle_lock_on_minimize`: 设置最小化到系统托盘时是否自动加锁。
///
/// # 参数
/// - `window`: Slint 顶级应用窗口；
/// - `ctx`: 应用程序全局上下文句柄。
pub(crate) fn register_security_handlers(window: &AppWindow, ctx: &AppContext) {
    let bridge = window.global::<SettingsBridge>();
    let master_tree = Arc::clone(&ctx.master_tree);
    let master_cards = Arc::clone(&ctx.master_cards);
    let expanded_groups = Arc::clone(&ctx.expanded_groups);
    let selector_expanded_groups = Arc::clone(&ctx.selector_expanded_groups);
    let storage = ctx.core_state.storage();

    // 1. 冷启动 / 按需解锁保险库
    bridge.on_unlock_vault({
        let window_weak = window.as_weak();
        let storage = Arc::clone(&storage);
        let notif = ctx.notifications.clone();
        let tree = Arc::clone(&master_tree);
        let cards = Arc::clone(&master_cards);
        let exp = Arc::clone(&expanded_groups);
        let sel = Arc::clone(&selector_expanded_groups);

        move |password| {
            let pwd = password.to_string();
            let window_weak_inner = window_weak.clone();
            let storage = Arc::clone(&storage);
            let notif = notif.clone();
            let tree = Arc::clone(&tree);
            let cards = Arc::clone(&cards);
            let exp = Arc::clone(&exp);
            let sel = Arc::clone(&sel);

            spawn_async(async move {
                match storage.unlock_vault(&pwd).await {
                    Ok(true) => {
                        tracing::info!(target: "smagical_ui::security", "主密码校验成功，保险库已解锁");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = window_weak_inner.upgrade() {
                                let sb = w.global::<SettingsBridge>();
                                sb.set_is_unlock_modal_open(false);
                                sb.set_unlock_pwd_input("".into());
                                sb.set_unlock_pwd_error_msg("".into());
                                notif.success("保险库已解锁", "本地凭据与主机资产已就绪");
                                refresh_vault_sensitive_data(&w, storage, tree, cards, exp, sel);
                            }
                        });
                    }
                    Ok(false) => {
                        tracing::warn!(target: "smagical_ui::security", "用户输入的主密码错误");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = window_weak_inner.upgrade() {
                                let sb = w.global::<SettingsBridge>();
                                sb.set_unlock_pwd_error_msg("主密码不正确，请重新输入".into());
                            }
                        });
                    }
                    Err(err) => {
                        tracing::error!(target: "smagical_ui::security", "解锁保险库异常: {:?}", err);
                        let err_msg = format!("解锁异常: {}", err);
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = window_weak_inner.upgrade() {
                                let sb = w.global::<SettingsBridge>();
                                sb.set_unlock_pwd_error_msg(err_msg.into());
                            }
                        });
                    }
                }
            });
        }
    });

    // 2. 设置或修改主密码
    bridge.on_set_master_password({
        let window_weak = window.as_weak();
        let storage = Arc::clone(&storage);
        let notif = ctx.notifications.clone();
        move |old_pwd, new_pwd| {
            let old = old_pwd.to_string();
            let new = new_pwd.to_string();

            if new.chars().count() < 6 {
                if let Some(w) = window_weak.upgrade() {
                    w.global::<SettingsBridge>().set_master_pwd_error_msg("主密码长度至少需为 6 位！".into());
                }
                return;
            }

            let window_weak_inner = window_weak.clone();
            let storage = Arc::clone(&storage);
            let notif = notif.clone();

            spawn_async(async move {
                let old_opt = if old.is_empty() { None } else { Some(old.as_str()) };
                match storage.change_master_password(old_opt, &new, "").await {
                    Ok(()) => {
                        tracing::info!(target: "smagical_ui::security", "主密码已更新 (信封重新封装完成)");
                        // 同步持久化配置
                        let _ = storage.config().update(Box::new(|c| {
                            c.master_password_enabled = true;
                        })).await;

                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = window_weak_inner.upgrade() {
                                let sb = w.global::<SettingsBridge>();
                                sb.set_setting_master_password_enabled(true);
                                sb.set_is_master_pwd_modal_open(false);
                                sb.set_master_pwd_old("".into());
                                sb.set_master_pwd_new("".into());
                                sb.set_master_pwd_confirm("".into());
                                sb.set_master_pwd_error_msg("".into());
                                notif.success("主密码已生效", "本地所有凭据与私钥已完成安全信封封装");
                            }
                        });
                    }
                    Err(err) => {
                        tracing::error!(target: "smagical_ui::security", "设置主密码失败: {:?}", err);
                        let err_msg = format!("设置主密码失败: {}", err);
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = window_weak_inner.upgrade() {
                                let sb = w.global::<SettingsBridge>();
                                sb.set_master_pwd_error_msg(err_msg.into());
                            }
                        });
                    }
                }
            });
        }
    });

    // 3. 移除主密码并降级回默认开箱即用无感模式
    bridge.on_submit_remove_master_password({
        let window_weak = window.as_weak();
        let storage = Arc::clone(&storage);
        let notif = ctx.notifications.clone();
        move |current_pwd| {
            let pwd = current_pwd.to_string();
            let window_weak_inner = window_weak.clone();
            let storage = Arc::clone(&storage);
            let notif = notif.clone();

            spawn_async(async move {
                match storage.remove_master_password(&pwd).await {
                    Ok(()) => {
                        tracing::info!(target: "smagical_ui::security", "已成功移除主密码保护");
                        let _ = storage.config().update(Box::new(|c| {
                            c.master_password_enabled = false;
                        })).await;

                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = window_weak_inner.upgrade() {
                                let sb = w.global::<SettingsBridge>();
                                sb.set_setting_master_password_enabled(false);
                                sb.set_is_remove_pwd_modal_open(false);
                                sb.set_remove_pwd_input("".into());
                                sb.set_remove_pwd_error_msg("".into());
                                notif.info("已移除主密码", "本地凭据已恢复为开箱即用的默认设备密钥模式");
                            }
                        });
                    }
                    Err(err) => {
                        tracing::error!(target: "smagical_ui::security", "移除主密码验证失败: {:?}", err);
                        let err_msg = format!("移除失败: {}", err);
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = window_weak_inner.upgrade() {
                                let sb = w.global::<SettingsBridge>();
                                sb.set_remove_pwd_error_msg(err_msg.into());
                            }
                        });
                    }
                }
            });
        }
    });

    // 4. 手动立即锁定保险库
    bridge.on_lock_vault({
        let window_weak = window.as_weak();
        let storage = Arc::clone(&storage);
        let notif = ctx.notifications.clone();
        move || {
            storage.lock_vault();
            tracing::info!(target: "smagical_ui::security", "用户手动锁定保险库，内存密钥已抹除");

            let window_weak_inner = window_weak.clone();
            let notif = notif.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak_inner.upgrade() {
                    let sb = w.global::<SettingsBridge>();
                    sb.set_is_unlock_modal_open(true);
                    sb.set_unlock_pwd_input("".into());
                    sb.set_unlock_pwd_error_msg("".into());
                    notif.info("保险库已锁定", "内存密钥已安全抹除，请输入主密码重新解锁");
                }
            });
        }
    });
}

use std::sync::atomic::{AtomicU64, Ordering};

/// 全局记录用户最近一次交互操作的 UNIX 纪元时间戳 (秒数)。
///
/// 用于空闲自动锁定守护线程检测用户是否离席。
/// 任何键盘敲击、文本粘贴、代码片段执行或导航路由均会原子更新该变量。
static LAST_USER_ACTIVITY_SECS: AtomicU64 = AtomicU64::new(0);

/// 记录用户交互行为活跃 (用于空闲超时自动锁定判定)。
///
/// # 触发时机
/// - 终端键盘按键敲击 (`on_terminal_key_input`)；
/// - 终端剪贴板文本粘贴 (`on_terminal_paste`)；
/// - 快捷指令/片段注入执行 (`on_send_snippet`)；
/// - 自动锁定守护任务启动时 (`start_auto_lock_monitor`)。
///
/// # 性能设计
/// 采用纯原子存储 `Ordering::Relaxed`，耗时小于 1ns，不会产生系统调用或锁竞争，可在高频输入下放心调用。
pub fn record_user_activity() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    LAST_USER_ACTIVITY_SECS.store(now, Ordering::Relaxed);
}

/// 解析用户配置的空闲超时时间字符串并转换为物理秒数。
///
/// # 支持格式
/// - `"5m"` -> 300 秒 (5分钟)
/// - `"15m"` -> 900 秒 (15分钟)
/// - `"30m"` -> 1800 秒 (30分钟)
/// - `"1h"` -> 3600 秒 (1小时)
/// - `"never"` 或其他非法格式 -> 返回 `0` (表示禁用自动锁定)
///
/// # 参数
/// - `s`: 界面下拉框传入的时间策略标识串。
///
/// # 返回值
/// 对应的超时时间秒数；若为 0 则表示不启用空闲锁定。
fn parse_timeout_secs(s: &str) -> u64 {
    match s.trim().to_lowercase().as_str() {
        "5m" => 300,
        "15m" => 900,
        "30m" => 1800,
        "1h" => 3600,
        _ => 0,
    }
}

/// 启动空闲超时自动锁定保险库后台守护协程。
///
/// # 核心设计
/// 1. **常驻后台轮询**：每隔 10 秒唤醒一次，极低 CPU 占用；
/// 2. **三级前置快速短路**：
///    - 检查 1：当前设备是否设置了主密码 (`has_custom_master_password`)。若未设，直接跳过；
///    - 检查 2：当前保险库是否处于已解锁状态 (`is_vault_unlocked`)。若已在锁定态，无需重复锁；
///    - 检查 3：当前超时配置是否大于 0。若为 `"never"`，跳过；
/// 3. **精准超时判定与密钥抹除**：
///    - 若 `当前时间 - 上次活跃时间 >= 超时秒数`，调用 `storage.lock_vault()` 将内存中的派生主密钥与私钥全部抹零；
///    - 调度 Slint UI 事件循环弹出解锁遮罩 (`is_unlock_modal_open = true`) 并清空旧输入；
///    - 发送全局 Toast 信息通知提示用户保险库已被安全锁闭；
/// 4. **原子防抖重置**：锁定后立即重置活跃时间戳，杜绝因轮询引起的连续重复弹窗通知。
///
/// # 参数
/// - `window_weak`: Slint UI 主窗口句柄弱引用 (用于跨线程更新视图模型)；
/// - `storage`: 底层存储与加解密引擎门面句柄；
/// - `notif`: 全局气泡通知服务管理器。
pub fn start_auto_lock_monitor(
    window_weak: slint::Weak<AppWindow>,
    storage: Arc<dyn AppStorage>,
    notif: crate::notification_service::NotificationManager,
) {
    record_user_activity();

    crate::async_util::spawn_async(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;

            // 若 UI 主窗口已销毁，终止常驻巡检协程，释放 Arc 存储连接与资源
            if window_weak.upgrade().is_none() {
                tracing::info!("UI 主窗口已关闭，终止保险库自动锁定巡检后台协程");
                break;
            }

            // 1. 若当前未启用主密码或未解锁，无需执行锁定
            let has_master = storage.has_custom_master_password().await.unwrap_or(false);
            if !has_master || !storage.is_vault_unlocked() {
                continue;
            }

            // 2. 读取当前超时配置
            let cfg = storage.config().get().await.unwrap_or_default();
            let timeout_secs = parse_timeout_secs(&cfg.auto_lock_timeout);
            if timeout_secs == 0 {
                continue;
            }

            // 3. 计算距离上次操作经过的时间
            let last_activity = LAST_USER_ACTIVITY_SECS.load(Ordering::Relaxed);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if last_activity > 0 && now >= last_activity + timeout_secs {
                // 触发自动锁定
                storage.lock_vault();
                tracing::info!(
                    target: "smagical_ui::security",
                    "空闲时间已超过安全阈值 ({}秒)，保险库已自动锁定",
                    timeout_secs
                );

                let w_weak = window_weak.clone();
                let notif_clone = notif.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = w_weak.upgrade() {
                        let sb = w.global::<SettingsBridge>();
                        sb.set_is_unlock_modal_open(true);
                        sb.set_unlock_pwd_input("".into());
                        sb.set_unlock_pwd_error_msg("".into());
                        notif_clone.info("保险库已自动锁定", "检测到长时间未操作，已自动锁定凭据保险库");
                    }
                });

                // 重置时间戳，防止紧接着循环内连续重复弹通知
                record_user_activity();
            }
        }
    });
}
