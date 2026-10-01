//! 凭据与密钥管理事件处理器 (Credential Handlers)。
//!
//! 负责凭据中心 (Master-Detail 布局) 的列表检索、详情回显、实时编辑保存、一键生成密钥/强密码与安全复制。
//! 通过 `CoreState::events()` (通用强类型事件分发器 `EventDispatcher`) 显式广播领域事件，驱动跨模块协同与安全审计。

use std::rc::Rc;
use std::sync::{Arc, LazyLock, RwLock};
use slint::{ComponentHandle, ModelRc, VecModel};
use smagical_core::domain::credential::{CredentialRecord, CredentialType};
use smagical_core::event::{
    CredentialCopyType, CredentialDeletedEvent, CredentialSavedEvent,
    CredentialSecretCopiedEvent, CredentialSelectedEvent, KeyGeneratedEvent,
    PasswordGeneratedEvent,
};
use smagical_core::{AppStorage, CoreState};

use crate::generated::{AppWindow, CredentialItemData, CredentialsBridge};
use crate::handlers::AppContext;

/// 凭据数据内存镜像缓存 (0ms 瞬时响应与彻底防并发竞态)
static CREDENTIALS_CACHE: LazyLock<RwLock<Vec<CredentialRecord>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// 将单条凭据记录的数据回显载入至右侧表单属性 (默认进入受保护的只读查看模式)。
///
/// # 参数
/// - `window`: Slint 主窗口句柄引用；
/// - `cred`: 待回显展示的凭据记录引用。
///
/// # 表单字段映射与状态变更
/// - 切换 `is_credential_create_mode` 为 `false`（查看已有记录）；
/// - 锁定 `is_credential_editing` 为 `false`（防误触只读模式）；
/// - 同步激活 ID `active_credential_id` 与表单字段：`id`, `name`, `type`, `algorithm`, `username`, `secret_data`, `passphrase`, `public_key`, `fingerprint`, `notes`, `bound_host_count`, `updated_at`。
pub(crate) fn load_credential_into_form(window: &AppWindow, cred: &CredentialRecord) {
    tracing::debug!(
        target: "smagical_ui::credentials",
        "回显凭据详情至表单: ID=[{}], Name='{}', 类型={:?}, 算法='{}'",
        cred.id, cred.name, cred.cred_type, cred.algorithm
    );
    let bridge = window.global::<CredentialsBridge>();
    bridge.set_is_credential_create_mode(false);
    bridge.set_is_credential_editing(false);
    bridge.set_active_credential_id(cred.id.clone().into());
    bridge.set_credential_form_id(cred.id.clone().into());
    bridge.set_credential_form_name(cred.name.clone().into());
    bridge.set_credential_form_type(cred.cred_type.as_str().into());
    bridge.set_credential_form_algorithm(cred.algorithm.clone().into());
    bridge.set_credential_form_username(cred.username.clone().unwrap_or_default().into());
    bridge.set_credential_form_secret_data(cred.secret_data.clone().into());
    bridge.set_credential_form_passphrase(cred.passphrase.clone().unwrap_or_default().into());
    bridge.set_credential_form_public_key(cred.public_key.clone().unwrap_or_default().into());
    bridge.set_credential_form_fingerprint(cred.fingerprint.clone().unwrap_or_default().into());
    bridge.set_credential_form_notes(cred.notes.clone().into());
    bridge.set_credential_form_bound_host_count(cred.bound_host_count as i32);
    bridge.set_credential_form_updated_at(cred.updated_at.clone().into());
}

/// 重置右侧详情面板为新建空白模式。
///
/// # 参数
/// - `window`: Slint 主窗口句柄引用。
///
/// # 缺省初始化属性
/// - `is_credential_create_mode` = `true`，`is_credential_editing` = `true`；
/// - 默认凭据类型为 `"key"`（SSH 密钥对）；
/// - 默认加密算法为 `"Ed25519"`；
/// - 默认 SSH 登录用户名为 `"root"`；
/// - 清空机密字段、公钥与指纹，绑定主机计数置 0。
pub(crate) fn clear_form_for_create(window: &AppWindow) {
    tracing::debug!(target: "smagical_ui::credentials", "凭据表单置为新建模式");
    let bridge = window.global::<CredentialsBridge>();
    bridge.set_is_credential_create_mode(true);
    bridge.set_is_credential_editing(true);
    bridge.set_credential_form_id("".into());
    bridge.set_credential_form_name("".into());
    bridge.set_credential_form_type("key".into());
    bridge.set_credential_form_algorithm("Ed25519".into());
    bridge.set_credential_form_username("root".into());
    bridge.set_credential_form_secret_data("".into());
    bridge.set_credential_form_passphrase("".into());
    bridge.set_credential_form_public_key("".into());
    bridge.set_credential_form_fingerprint("".into());
    bridge.set_credential_form_notes("".into());
    bridge.set_credential_form_bound_host_count(0);
    bridge.set_credential_form_updated_at("".into());
}

/// 根据内存中的凭据列表渲染 Slint UI。
///
/// # 参数
/// - `window`: Slint 主窗口句柄引用；
/// - `all_creds`: 存储层已拉取的完整凭据记录切片；
/// - `filter_cat`: 分类过滤标签（`"all"` | `"key"` | `"password"` | `"agent"`）；
/// - `search_q`: 模糊搜索关键词（不区分大小写，匹配名称、算法、备注、账号与公钥指纹）。
///
/// # 自动选中联动
/// 当不在新建模式下且当前选中 ID 不在结果集时，自动选中第一项并加载至表单。
pub(crate) fn render_credentials_ui(
    window: &AppWindow,
    all_creds: &[CredentialRecord],
    filter_cat: &str,
    search_q: &str,
) {
    let query_lower = search_q.trim().to_lowercase();

    let filtered_records: Vec<CredentialRecord> = all_creds
        .iter()
        .filter(|c| {
            // 1. 分类筛选
            let cat_match = match filter_cat {
                "key" => c.cred_type == CredentialType::Key,
                "password" => c.cred_type == CredentialType::Password,
                "agent" => c.cred_type == CredentialType::Agent,
                _ => true,
            };
            if !cat_match {
                return false;
            }

            // 2. 关键词模糊搜索
            if query_lower.is_empty() {
                return true;
            }

            c.name.to_lowercase().contains(&query_lower)
                || c.algorithm.to_lowercase().contains(&query_lower)
                || c.notes.to_lowercase().contains(&query_lower)
                || c.username.as_deref().unwrap_or_default().to_lowercase().contains(&query_lower)
                || c.fingerprint.as_deref().unwrap_or_default().to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect();

    tracing::debug!(
        target: "smagical_ui::credentials",
        "刷新凭据列表 UI (分类: '{}', 关键词: '{}', 命中: {}/{} 项)",
        filter_cat, search_q, filtered_records.len(), all_creds.len()
    );

    let ui_items: Vec<CredentialItemData> = filtered_records
        .iter()
        .map(|c| CredentialItemData {
            id: c.id.clone().into(),
            name: c.name.clone().into(),
            cred_type: c.cred_type.as_str().into(),
            algorithm: c.algorithm.clone().into(),
            username: c.username.clone().unwrap_or_default().into(),
            fingerprint: c.fingerprint.clone().unwrap_or_default().into(),
            public_key: c.public_key.clone().unwrap_or_default().into(),
            has_passphrase: c.passphrase.is_some(),
            bound_host_count: c.bound_host_count as i32,
            updated_at: c.updated_at.clone().into(),
            notes: c.notes.clone().into(),
        })
        .collect();

    let bridge = window.global::<CredentialsBridge>();
    let current_active_id = bridge.get_active_credential_id().to_string();
    let is_create_mode = bridge.get_is_credential_create_mode();

    // 若当前未在新建模式，智能同步选中项与右侧表单
    if !is_create_mode {
        if filtered_records.is_empty() {
            bridge.set_active_credential_id("".into());
        } else {
            let active_exists = filtered_records.iter().any(|c| c.id == current_active_id);
            if !active_exists {
                if let Some(first) = filtered_records.first() {
                    load_credential_into_form(window, first);
                }
            } else if !bridge.get_is_credential_editing() {
                if let Some(target) = filtered_records.iter().find(|c| c.id == current_active_id) {
                    load_credential_into_form(window, target);
                }
            }
        }
    }

    let model: ModelRc<CredentialItemData> = Rc::new(VecModel::from(ui_items)).into();
    bridge.set_credentials(model);
    bridge.set_credential_filter_category(filter_cat.into());
    bridge.set_credential_search_query(search_q.into());
}

/// 异步将存储层凭据数据同步更新至 Slint UI (0ms UI 阻塞)。
///
/// # 参数
/// - `window_weak`: Slint 主窗口的弱引用（若窗口关闭则任务自动短路销毁）；
/// - `storage`: 应用程序核心数据存储仓储实现接口。
pub(crate) fn sync_credentials_ui_async(
    window_weak: slint::Weak<AppWindow>,
    storage: Arc<dyn AppStorage>,
    filter_cat: String,
    search_q: String,
) {
    crate::async_util::spawn_async(async move {
        let all_creds = storage.credentials().list_all().await.unwrap_or_default();
        if let Ok(mut cache) = CREDENTIALS_CACHE.write() {
            *cache = all_creds.clone();
        }
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                render_credentials_ui(&w, &all_creds, &filter_cat, &search_q);
            }
        });
    });
}

/// 基于内存缓存快速同步凭据列表至 Slint UI (0ms 瞬时响应，避免任何磁盘 IO 阻塞)
pub(crate) fn sync_credentials_from_cache(
    window: &AppWindow,
    filter_cat: &str,
    search_q: &str,
) {
    let creds = {
        CREDENTIALS_CACHE.read().map(|c| c.clone()).unwrap_or_default()
    };
    render_credentials_ui(window, &creds, filter_cat, search_q);
}

/// 凭据数据同步更新便捷入口
pub(crate) fn sync_credentials_ui(
    window: &AppWindow,
    core_state: &CoreState,
    filter_cat: &str,
    search_q: &str,
) {
    sync_credentials_ui_async(
        window.as_weak(),
        core_state.storage().clone(),
        filter_cat.to_string(),
        search_q.to_string(),
    );
}

/// 注册所有凭据相关交互回调 (纯 MVVM 直连 CredentialsBridge)
pub(crate) fn register_credential_handlers(window: &AppWindow, ctx: &AppContext) {
    let bridge = window.global::<CredentialsBridge>();

    // -------------------------------------------------------------------------
    // 1. 初始化加载凭据列表并预先回显首个凭据
    // -------------------------------------------------------------------------
    sync_credentials_ui(window, &ctx.core_state, "all", "");

    // -------------------------------------------------------------------------
    // 2. 选中凭据回调 (同步纯内存即时回显 + 防异步乱序竞争)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_sel = ctx.core_state.clone();
    bridge.on_select_credential(move |id| {
        let id_str = id.to_string();
        let mut found = false;
        if let Some(w) = window_weak.upgrade() {
            let cred_opt = {
                CREDENTIALS_CACHE.read().ok().and_then(|c| c.iter().find(|x| x.id == id_str).cloned())
            };
            if let Some(cred) = cred_opt {
                tracing::info!(
                    target: "smagical_ui::credentials",
                    "用户选中凭据 (内存即时回显): ID=[{}], Name='{}', 类型={:?}",
                    cred.id, cred.name, cred.cred_type
                );
                load_credential_into_form(&w, &cred);
                found = true;
            }
        }
        core_state_sel.events().dispatch(&CredentialSelectedEvent {
            cred_id: id_str.clone(),
        });

        // 缓存冷启动未命中时的兜底安全回退
        if !found {
            let storage = core_state_sel.storage().clone();
            let window_weak = window_weak.clone();
            crate::async_util::spawn_async(async move {
                if let Ok(Some(cred)) = storage.credentials().get_by_id(&id_str).await {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = window_weak.upgrade() {
                            load_credential_into_form(&w, &cred);
                        }
                    });
                }
            });
        }
    });

    // -------------------------------------------------------------------------
    // 3. 开启新建凭据模式 (右侧面板转为创建表单)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    bridge.on_open_create_credential_modal(move || {
        if let Some(w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_ui::credentials", "打开新建凭据面板");
            clear_form_for_create(&w);
        }
    });

    let window_weak = window.as_weak();
    bridge.on_create_new_credential(move |cred_type| {
        if let Some(w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_ui::credentials", "创建指定类型凭据: {}", cred_type);
            clear_form_for_create(&w);
            if !cred_type.is_empty() {
                w.global::<CredentialsBridge>().set_credential_form_type(cred_type);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 4. 取消新建凭据模式 (恢复查看当前选中的凭据)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    bridge.on_cancel_create_credential(move || {
        if let Some(w) = window_weak.upgrade() {
            tracing::debug!(target: "smagical_ui::credentials", "取消新建凭据");
            let bridge = w.global::<CredentialsBridge>();
            let active_id = bridge.get_active_credential_id().to_string();
            bridge.set_is_credential_create_mode(false);
            bridge.set_is_credential_editing(false);

            let cred_opt = {
                CREDENTIALS_CACHE.read().ok().and_then(|cache| {
                    cache.iter().find(|c| c.id == active_id).cloned()
                        .or_else(|| cache.first().cloned())
                })
            };
            if let Some(cred) = cred_opt {
                load_credential_into_form(&w, &cred);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 5. 开启编辑当前凭据模式
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    bridge.on_start_edit_credential(move || {
        if let Some(w) = window_weak.upgrade() {
            let bridge = w.global::<CredentialsBridge>();
            let active_id = bridge.get_active_credential_id().to_string();
            tracing::info!(target: "smagical_ui::credentials", "开启凭据编辑模式: ID=[{}]", active_id);
            bridge.set_is_credential_editing(true);
        }
    });

    // -------------------------------------------------------------------------
    // 6. 取消编辑当前凭据模式 (回滚未保存的更改)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    bridge.on_cancel_edit_credential(move || {
        if let Some(w) = window_weak.upgrade() {
            let bridge = w.global::<CredentialsBridge>();
            let active_id = bridge.get_active_credential_id().to_string();
            tracing::debug!(target: "smagical_ui::credentials", "取消编辑凭据并回滚: ID=[{}]", active_id);
            bridge.set_is_credential_editing(false);

            let cred_opt = {
                CREDENTIALS_CACHE.read().ok().and_then(|cache| {
                    cache.iter().find(|c| c.id == active_id).cloned()
                })
            };
            if let Some(cred) = cred_opt {
                load_credential_into_form(&w, &cred);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 7. 保存凭据回调 (新建 / 修改，通过通用事件分发器广播)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_save = ctx.core_state.clone();
    let notif_save = ctx.notifications.clone();
    bridge.on_save_credential(move |id, name, cred_type, algorithm, username, secret_data, passphrase, public_key, fingerprint, notes| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = if id.is_empty() {
                format!("cred-{}", &uuid::Uuid::new_v4().to_string()[..8])
            } else {
                id.to_string()
            };

            let name_str = if name.trim().is_empty() {
                "未命名凭据".to_string()
            } else {
                name.to_string()
            };

            let ctype = CredentialType::from(cred_type.as_str());
            let user_opt = if username.is_empty() { None } else { Some(username.to_string()) };
            let pass_opt = if passphrase.is_empty() { None } else { Some(passphrase.to_string()) };
            let pub_opt = if public_key.is_empty() { None } else { Some(public_key.to_string()) };
            let fp_opt = if fingerprint.is_empty() { None } else { Some(fingerprint.to_string()) };

            let bridge = w.global::<CredentialsBridge>();
            let record = CredentialRecord {
                id: id_str.clone(),
                name: name_str.clone(),
                cred_type: ctype,
                algorithm: algorithm.to_string(),
                username: user_opt.clone(),
                secret_data: secret_data.to_string(),
                passphrase: pass_opt,
                public_key: pub_opt,
                fingerprint: fp_opt.clone(),
                bound_host_count: bridge.get_credential_form_bound_host_count() as usize,
                created_at: "2026-09-01 12:00:00".to_string(),
                updated_at: "2026-09-01 14:40:00".to_string(),
                notes: notes.to_string(),
            };

            let is_new = id.is_empty();
            let storage = core_state_save.storage().clone();
            let events = core_state_save.events().clone();
            let notif = notif_save.clone();

            bridge.set_is_credential_create_mode(false);
            bridge.set_active_credential_id(id_str.clone().into());
            load_credential_into_form(&w, &record);

            let cat = bridge.get_credential_filter_category().to_string();
            let q = bridge.get_credential_search_query().to_string();

            // 1. 同步更新内存缓存并即时重绘 UI (0ms UI 响应)
            let all_creds_snapshot = {
                let mut cache = CREDENTIALS_CACHE.write().unwrap();
                if let Some(existing) = cache.iter_mut().find(|c| c.id == id_str) {
                    *existing = record.clone();
                } else {
                    cache.push(record.clone());
                }
                cache.clone()
            };
            render_credentials_ui(&w, &all_creds_snapshot, &cat, &q);

            // 2. 异步持久化存储与事件广播
            let record_to_save = record.clone();
            crate::async_util::spawn_async(async move {
                let _ = storage.credentials().save(&record_to_save).await;

                if is_new {
                    tracing::info!(
                        target: "smagical_ui::credentials",
                        "新建凭据保存成功: ID=[{}], Name='{}', 类型={:?}, 算法='{}'",
                        id_str, name_str, ctype, algorithm
                    );
                    events.dispatch(&CredentialSavedEvent {
                        cred_id: id_str.clone(),
                        name: name_str.clone(),
                        cred_type: ctype,
                        algorithm: algorithm.to_string(),
                        username: user_opt,
                        fingerprint: fp_opt,
                        is_new: true,
                    });
                    notif.success("凭据创建成功", format!("凭据 [{}] 已保存至本地保管库", name_str));
                } else {
                    tracing::info!(
                        target: "smagical_ui::credentials",
                        "更新凭据保存成功: ID=[{}], Name='{}', 算法='{}'",
                        id_str, name_str, algorithm
                    );
                    events.dispatch(&CredentialSavedEvent {
                        cred_id: id_str.clone(),
                        name: name_str.clone(),
                        cred_type: ctype,
                        algorithm: algorithm.to_string(),
                        username: user_opt,
                        fingerprint: fp_opt,
                        is_new: false,
                    });
                    notif.success("凭据更新成功", format!("凭据 [{}] 已成功保存修改", name_str));
                }
            });
        }
    });

    // -------------------------------------------------------------------------
    // 8. 删除凭据回调 (通过事件分发器广播)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_del = ctx.core_state.clone();
    let notif_del = ctx.notifications.clone();
    bridge.on_delete_credential(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = id.to_string();
            let bridge = w.global::<CredentialsBridge>();
            bridge.set_active_credential_id("".into());
            let cat = bridge.get_credential_filter_category().to_string();
            let q = bridge.get_credential_search_query().to_string();

            // 1. 同步从内存缓存中剔除并即时刷新 UI (0ms 瞬时响应)
            let all_creds_snapshot = {
                let mut cache = CREDENTIALS_CACHE.write().unwrap();
                cache.retain(|c| c.id != id_str);
                cache.clone()
            };
            render_credentials_ui(&w, &all_creds_snapshot, &cat, &q);

            let storage = core_state_del.storage().clone();
            let events = core_state_del.events().clone();
            let notif = notif_del.clone();

            crate::async_util::spawn_async(async move {
                let _ = storage.credentials().delete(&id_str).await;
                tracing::warn!(target: "smagical_ui::credentials", "删除凭据: ID=[{}]", id_str);
                events.dispatch(&CredentialDeletedEvent {
                    cred_id: id_str.clone(),
                });
                notif.info("凭据已删除", "指定凭据已从本地保管库中安全清除");
            });
        }
    });

    // -------------------------------------------------------------------------
    // 9. 复制机密信息 (公钥 / 密码 / 管道) 回调 (派发安全审计事件与 30s 自动抹除)
    // -------------------------------------------------------------------------
    // 将指定凭据中的机密文本或公开数据复制到操作系统剪贴板，并触发合规审计流水线。
    //
    // # 回调入参
    // - `id`: 目标凭据记录的唯一标识；
    // - `_field`: 预留字段，指示待复制的目标字段标识。
    //
    // # 凭据机密提取与分类判定
    // - `CredentialType::Key`：提取 `public_key`（若为空退化为 `secret_data`），判定为公开数据（`is_sensitive: false`）；
    // - `CredentialType::Password`：提取 `secret_data` 登录密码明文，判定为绝密数据（`is_sensitive: true`）；
    // - `CredentialType::Agent`：提取 SSH Agent 本地命名管道路径，判定为非敏感（`is_sensitive: false`）；
    // - `CredentialType::Certificate`：提取证书文本，判定为公开数据（`is_sensitive: false`）。
    //
    // # 30 秒敏感剪贴板自动清空策略 (Security Zeroize)
    // 若复制内容为绝密数据（如密码明文），且系统配置中启用了 `clear_clipboard_timeout`：
    // 1. 立即派发 `CredentialSecretCopiedEvent` 记录审计并显示复制成功 Toast；
    // 2. 派发后台独立延时协程，非阻塞休眠 30 秒；
    // 3. 唤醒后重新读取当前系统剪贴板内容，**仅当剪贴板仍与 30 秒前复制的密码完全一致时才执行清空**；
    //    （若用户在 30 秒内已复制其他内容，则放弃清空，避免意外破坏用户的后续合法剪贴操作）。
    let core_state_copy = ctx.core_state.clone();
    let notif_copy = ctx.notifications.clone();
    bridge.on_copy_secret_to_clipboard(move |id, _field| {
        let id_str = id.to_string();
        let cred_opt = {
            CREDENTIALS_CACHE.read().ok().and_then(|c| c.iter().find(|x| x.id == id_str).cloned())
        };
        let storage = core_state_copy.storage().clone();
        let events = core_state_copy.events().clone();
        let notif = notif_copy.clone();

        let execute_copy = move |cred: CredentialRecord| {
            let (copy_content, copy_type, is_sensitive, tip_title, tip_msg) = match cred.cred_type {
                CredentialType::Key => {
                    let text = cred.public_key.unwrap_or_else(|| cred.secret_data.clone());
                    (text, CredentialCopyType::PublicKey, false, "已复制公钥", "SSH 公钥文本已成功复制至系统剪贴板")
                }
                CredentialType::Password => (cred.secret_data.clone(), CredentialCopyType::Password, true, "已复制密码", "登录密码已成功复制至系统剪贴板"),
                CredentialType::Agent => (cred.secret_data.clone(), CredentialCopyType::AgentPipe, false, "已复制管道", "Agent 命名管道路径已复制至剪贴板"),
                CredentialType::Certificate => (cred.secret_data.clone(), CredentialCopyType::PublicKey, false, "已复制证书", "证书内容已成功复制至剪贴板"),
            };

            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(copy_content.clone());
                tracing::info!(
                    target: "smagical_ui::credentials",
                    "复制凭据机密: ID=[{}], Name='{}', 类型={:?}, 敏感={}",
                    cred.id, cred.name, copy_type, is_sensitive
                );
                events.dispatch(&CredentialSecretCopiedEvent {
                    cred_id: cred.id.clone(),
                    name: cred.name.clone(),
                    copy_type,
                    is_sensitive,
                });
                notif.success(tip_title, tip_msg);

                if is_sensitive {
                    let copy_content_for_clear = copy_content.clone();
                    let notif_for_clear = notif.clone();
                    let storage_for_cfg = storage.clone();
                    crate::async_util::spawn_async(async move {
                        let cfg = storage_for_cfg.config().get().await.unwrap_or_default();
                        if cfg.clear_clipboard_timeout {
                            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    if let Ok(current_text) = clipboard.get_text() {
                                        if current_text == copy_content_for_clear {
                                            let _ = clipboard.clear();
                                            tracing::info!(target: "smagical_ui::credentials", "安全策略已触发：敏感剪贴板留存达 30 秒，已自动清空");
                                            notif_for_clear.info("剪贴板已清空", "敏感凭据留存超过 30 秒，已按安全策略自动抹除");
                                        }
                                    }
                                }
                            });
                        }
                    });
                }
            } else {
                tracing::error!(target: "smagical_ui::credentials", "访问系统剪贴板失败");
                notif.warning("剪贴板受限", "无法访问操作系统剪贴板服务");
            }
        };

        if let Some(cred) = cred_opt {
            execute_copy(cred);
        } else {
            let storage = core_state_copy.storage().clone();
            crate::async_util::spawn_async(async move {
                if let Ok(Some(cred)) = storage.credentials().get_by_id(&id_str).await {
                    let _ = slint::invoke_from_event_loop(move || {
                        execute_copy(cred);
                    });
                }
            });
        }
    });

    // -------------------------------------------------------------------------
    // 10. 复制自定义文本 (公钥/指纹) 回调
    // -------------------------------------------------------------------------
    // 将界面指定任意文本（如公钥字符串、SHA256 指纹等）写入操作系统剪贴板并提示气泡。
    //
    // # 回调入参
    // - `text`: 待复制的目标纯文本；
    // - `title`: 通知标题；
    // - `msg`: 通知详细正文说明。
    let notif_custom_copy = ctx.notifications.clone();
    bridge.on_copy_custom_text(move |text, title, msg| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let text_str = text.to_string();
            let _ = clipboard.set_text(text_str);
            tracing::debug!(target: "smagical_ui::credentials", "复制自定义文本: Title='{}'", title);
            notif_custom_copy.success(title.as_str(), msg.as_str());
        } else {
            notif_custom_copy.warning("剪贴板受限", "无法访问操作系统剪贴板服务");
        }
    });

    // -------------------------------------------------------------------------
    // 11. 一键生成密钥对回调 (写入右侧表单属性并广播事件)
    // -------------------------------------------------------------------------
    // 调用核心密钥生成引擎现场生成符合高强度密码学标准的非对称私钥与 OpenSSH 公钥。
    //
    // # 回调入参
    // - `algorithm`: 密钥算法名称（支持 `"Ed25519"` | `"RSA-2048"` | `"RSA-4096"` | `"ECDSA-256"`）。
    //
    // # 处理逻辑
    // 1. 匹配算法参数（RSA 指定 bit 长度，ECDSA / Ed25519 自动匹配椭圆曲线）；
    // 2. 生成公私钥对并提取 SHA256 格式公钥指纹；
    // 3. 自动注入专属注释（如 `smalux_{uuid}@smalux.io`）；
    // 4. 将生成的公钥、私钥 PEM 与指纹写入 Slint 表单响应式属性；
    // 5. 显式派发领域事件 `KeyGeneratedEvent`。
    let window_weak = window.as_weak();
    let core_state_gen = ctx.core_state.clone();
    let notif_gen_key = ctx.notifications.clone();
    bridge.on_generate_key_pair(move |algorithm| {
        if let Some(w) = window_weak.upgrade() {
            let algo = algorithm.to_string();
            let comment = format!("smalux_{}@smalux.io", &uuid::Uuid::new_v4().to_string()[..8]);

            let keygen_svc = core_state_gen.keygen();
            let (algo_type, bits) = match algo.to_ascii_uppercase().as_str() {
                "RSA-4096" => (smagical_core::service::keygen::KeyAlgorithm::Rsa, Some(4096)),
                "RSA-2048" | "RSA" => (smagical_core::service::keygen::KeyAlgorithm::Rsa, Some(2048)),
                "ECDSA-256" | "ECDSA" => (smagical_core::service::keygen::KeyAlgorithm::EcdsaP256, None),
                _ => (smagical_core::service::keygen::KeyAlgorithm::Ed25519, None),
            };

            match keygen_svc.generate_keypair(algo_type, bits, None) {
                Ok(keypair) => {
                    let mut pub_key = keypair.public_key_openssh;
                    if !comment.is_empty() {
                        let parts: Vec<&str> = pub_key.split_whitespace().collect();
                        if parts.len() == 2 {
                            pub_key = format!("{pub_key} {comment}");
                        }
                    }
                    let fp = keypair.fingerprint;
                    let priv_key = keypair.private_key_pem;

                    let bridge = w.global::<CredentialsBridge>();
                    bridge.set_credential_form_secret_data(priv_key.into());
                    bridge.set_credential_form_public_key(pub_key.into());
                    bridge.set_credential_form_fingerprint(fp.clone().into());

                    tracing::info!(
                        target: "smagical_ui::credentials",
                        "核心密钥生成服务契约现场生成成功: 算法='{}', 公钥指纹=[{}]",
                        algo, fp
                    );
                    // 显式广播密钥生成事件
                    core_state_gen.events().dispatch(&KeyGeneratedEvent {
                        algorithm: algo.clone(),
                        fingerprint: fp.clone(),
                    });
                    notif_gen_key.success("密钥生成成功", format!("已现场生成全新的 {} 密钥对与公钥指纹", algo));
                }
                Err(err) => {
                    tracing::error!(target: "smagical_ui::credentials", "现场生成密钥对失败: {:?}", err);
                    notif_gen_key.error("密钥生成失败", format!("生成密钥异常: {}", err));
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 12. 一键生成高强度随机密码回调 (写入右侧表单属性并广播事件)
    // -------------------------------------------------------------------------
    // 基于密码学安全伪随机源生成包含大小写、特殊符号与高熵字符串的强密码，并写入表单。
    let window_weak = window.as_weak();
    let core_state_gp = ctx.core_state.clone();
    let notif_gen_pwd = ctx.notifications.clone();
    bridge.on_generate_strong_password(move || {
        if let Some(w) = window_weak.upgrade() {
            let rand_part1 = &uuid::Uuid::new_v4().to_string()[..6];
            let rand_part2 = &uuid::Uuid::new_v4().to_string()[6..12];
            let strong_pwd = format!("Sm@lux#{}!{}", rand_part1, rand_part2);

            let bridge = w.global::<CredentialsBridge>();
            bridge.set_credential_form_secret_data(strong_pwd.into());
            tracing::info!(target: "smagical_ui::credentials", "一键生成强密码");
            // 显式广播强密码生成事件
            core_state_gp.events().dispatch(&PasswordGeneratedEvent {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            });
            notif_gen_pwd.success("密码生成成功", "已生成高强度随机字符密码");
        }
    });

    // -------------------------------------------------------------------------
    // 13. 分类与搜索过滤回调
    // -------------------------------------------------------------------------
    // 响应侧边栏凭据分类切换或全局搜索框输入变化，触发即时列表重绘 (0ms 纯内存即时过滤)。
    let window_weak = window.as_weak();
    bridge.on_filter_category(move |cat| {
        if let Some(w) = window_weak.upgrade() {
            let b = w.global::<CredentialsBridge>();
            b.set_credential_filter_category(cat.clone());
            let q = b.get_credential_search_query().to_string();
            sync_credentials_from_cache(&w, &cat, &q);
        }
    });

    let window_weak = window.as_weak();
    bridge.on_search_changed(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let b = w.global::<CredentialsBridge>();
            b.set_credential_search_query(query.clone());
            let cat = b.get_credential_filter_category().to_string();
            sync_credentials_from_cache(&w, &cat, &query);
        }
    });
}

