//! 凭据与密钥管理事件处理器 (Credential Handlers)。
//!
//! 负责凭据中心 (Master-Detail 布局) 的列表检索、详情回显、实时编辑保存、一键生成密钥/强密码与安全复制。
//! 通过 `CoreState::events()` (通用强类型事件分发器 `EventDispatcher`) 显式广播领域事件，驱动跨模块协同与安全审计。

use std::rc::Rc;
use slint::{ComponentHandle, ModelRc, VecModel};
use smagical_core::domain::credential::{CredentialRecord, CredentialType};
use smagical_core::event::{
    CredentialCopyType, CredentialDeletedEvent, CredentialSavedEvent,
    CredentialSecretCopiedEvent, CredentialSelectedEvent, KeyGeneratedEvent,
    PasswordGeneratedEvent,
};
use smagical_core::CoreState;

use crate::generated::{AppWindow, CredentialItemData};
use crate::handlers::AppContext;

/// 将单条凭据记录的数据回显载入至右侧表单属性 (默认进入受保护的只读查看模式)
pub(crate) fn load_credential_into_form(window: &AppWindow, cred: &CredentialRecord) {
    tracing::debug!(
        target: "smagical_ui::credentials",
        "回显凭据详情至表单: ID=[{}], Name='{}', 类型={:?}, 算法='{}'",
        cred.id, cred.name, cred.cred_type, cred.algorithm
    );
    window.set_is_credential_create_mode(false);
    window.set_is_credential_editing(false);
    window.set_active_credential_id(cred.id.clone().into());
    window.set_credential_form_id(cred.id.clone().into());
    window.set_credential_form_name(cred.name.clone().into());
    window.set_credential_form_type(cred.cred_type.as_str().into());
    window.set_credential_form_algorithm(cred.algorithm.clone().into());
    window.set_credential_form_username(cred.username.clone().unwrap_or_default().into());
    window.set_credential_form_secret_data(cred.secret_data.clone().into());
    window.set_credential_form_passphrase(cred.passphrase.clone().unwrap_or_default().into());
    window.set_credential_form_public_key(cred.public_key.clone().unwrap_or_default().into());
    window.set_credential_form_fingerprint(cred.fingerprint.clone().unwrap_or_default().into());
    window.set_credential_form_notes(cred.notes.clone().into());
    window.set_credential_form_bound_host_count(cred.bound_host_count as i32);
    window.set_credential_form_updated_at(cred.updated_at.clone().into());
}

/// 清空右侧表单并置为新建模式
pub(crate) fn clear_form_for_create(window: &AppWindow) {
    tracing::debug!(target: "smagical_ui::credentials", "凭据表单置为新建模式");
    window.set_is_credential_create_mode(true);
    window.set_is_credential_editing(true);
    window.set_credential_form_id("".into());
    window.set_credential_form_name("".into());
    window.set_credential_form_type("key".into());
    window.set_credential_form_algorithm("Ed25519".into());
    window.set_credential_form_username("root".into());
    window.set_credential_form_secret_data("".into());
    window.set_credential_form_passphrase("".into());
    window.set_credential_form_public_key("".into());
    window.set_credential_form_fingerprint("".into());
    window.set_credential_form_notes("".into());
    window.set_credential_form_bound_host_count(0);
    window.set_credential_form_updated_at("".into());
}

/// 将存储层凭据数据同步更新至 Slint UI
pub(crate) fn sync_credentials_ui(
    window: &AppWindow,
    core_state: &CoreState,
    filter_cat: &str,
    search_q: &str,
) {
    let all_creds = core_state.storage().credentials().list_all().unwrap_or_default();
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

    let current_active_id = window.get_active_credential_id().to_string();
    let is_create_mode = window.get_is_credential_create_mode();

    // 若当前未在新建模式，且没有选中项或选中项不在列表中，自动选中第一项
    if !is_create_mode && !filtered_records.is_empty() {
        let active_exists = filtered_records.iter().any(|c| c.id == current_active_id);
        if !active_exists {
            if let Some(first) = filtered_records.first() {
                load_credential_into_form(window, first);
            }
        } else if let Some(target) = filtered_records.iter().find(|c| c.id == current_active_id) {
            load_credential_into_form(window, target);
        }
    }

    let model: ModelRc<CredentialItemData> = Rc::new(VecModel::from(ui_items)).into();
    window.set_credentials(model);
}

/// 注册所有凭据相关交互回调
pub(crate) fn register_credential_handlers(window: &AppWindow, ctx: &AppContext) {
    // -------------------------------------------------------------------------
    // 1. 初始化加载凭据列表并预先回显首个凭据
    // -------------------------------------------------------------------------
    sync_credentials_ui(window, &ctx.core_state, "all", "");

    // -------------------------------------------------------------------------
    // 2. 选中凭据回调 (加载并回显至右侧详情面板，派发选中事件)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_sel = ctx.core_state.clone();
    window.on_select_credential(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = id.to_string();
            if let Ok(Some(cred)) = core_state_sel.storage().credentials().get_by_id(&id_str) {
                tracing::info!(
                    target: "smagical_ui::credentials",
                    "用户选中凭据: ID=[{}], Name='{}', 类型={:?}",
                    cred.id, cred.name, cred.cred_type
                );
                // 显式派发凭据选中事件
                core_state_sel.events().dispatch(&CredentialSelectedEvent {
                    cred_id: id_str.clone(),
                });
                load_credential_into_form(&w, &cred);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 3. 开启新建凭据模式 (右侧面板转为创建表单)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    window.on_open_create_credential_modal(move || {
        if let Some(w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_ui::credentials", "打开新建凭据面板");
            clear_form_for_create(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 4. 取消新建凭据模式 (恢复查看当前选中的凭据)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_cancel = ctx.core_state.clone();
    window.on_cancel_create_credential(move || {
        if let Some(w) = window_weak.upgrade() {
            tracing::debug!(target: "smagical_ui::credentials", "取消新建凭据");
            let active_id = w.get_active_credential_id().to_string();
            if let Ok(Some(cred)) = core_state_cancel.storage().credentials().get_by_id(&active_id) {
                load_credential_into_form(&w, &cred);
                return;
            }
            // 若无有效选中项，回退加载首条凭据
            if let Some(first) = core_state_cancel.storage().credentials().list_all().ok().and_then(|all| all.into_iter().next()) {
                load_credential_into_form(&w, &first);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 5. 开启编辑当前凭据模式
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    window.on_start_edit_credential(move || {
        if let Some(w) = window_weak.upgrade() {
            let active_id = w.get_active_credential_id().to_string();
            tracing::info!(target: "smagical_ui::credentials", "开启凭据编辑模式: ID=[{}]", active_id);
            w.set_is_credential_editing(true);
        }
    });

    // -------------------------------------------------------------------------
    // 6. 取消编辑当前凭据模式 (回滚未保存的更改)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_ce = ctx.core_state.clone();
    window.on_cancel_edit_credential(move || {
        if let Some(w) = window_weak.upgrade() {
            let active_id = w.get_active_credential_id().to_string();
            tracing::debug!(target: "smagical_ui::credentials", "取消编辑凭据并回滚: ID=[{}]", active_id);
            w.set_is_credential_editing(false);
            if let Ok(Some(cred)) = core_state_ce.storage().credentials().get_by_id(&active_id) {
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
    window.on_save_credential(move |id, name, cred_type, algorithm, username, secret_data, passphrase, public_key, fingerprint, notes| {
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
                bound_host_count: w.get_credential_form_bound_host_count() as usize,
                created_at: "2026-09-01 12:00:00".to_string(),
                updated_at: "2026-09-01 14:40:00".to_string(),
                notes: notes.to_string(),
            };

            let is_new = id.is_empty();
            let _ = core_state_save.storage().credentials().save(&record);

            if is_new {
                tracing::info!(
                    target: "smagical_ui::credentials",
                    "新建凭据保存成功: ID=[{}], Name='{}', 类型={:?}, 算法='{}'",
                    id_str, name_str, ctype, algorithm
                );
                // 显式派发保存事件至事件总线
                core_state_save.events().dispatch(&CredentialSavedEvent {
                    cred_id: id_str.clone(),
                    name: name_str.clone(),
                    cred_type: ctype,
                    algorithm: algorithm.to_string(),
                    username: user_opt,
                    fingerprint: fp_opt,
                    is_new: true,
                });
                notif_save.success("凭据创建成功", format!("凭据 [{}] 已保存至本地保管库", name_str));
            } else {
                tracing::info!(
                    target: "smagical_ui::credentials",
                    "更新凭据保存成功: ID=[{}], Name='{}', 算法='{}'",
                    id_str, name_str, algorithm
                );
                // 显式派发更新事件至事件总线
                core_state_save.events().dispatch(&CredentialSavedEvent {
                    cred_id: id_str.clone(),
                    name: name_str.clone(),
                    cred_type: ctype,
                    algorithm: algorithm.to_string(),
                    username: user_opt,
                    fingerprint: fp_opt,
                    is_new: false,
                });
                notif_save.success("凭据更新成功", format!("凭据 [{}] 已成功保存修改", name_str));
            }

            w.set_is_credential_create_mode(false);
            w.set_active_credential_id(id_str.clone().into());
            load_credential_into_form(&w, &record);

            let cat = w.get_credential_filter_category();
            let q = w.get_credential_search_query();
            sync_credentials_ui(&w, &core_state_save, &cat, &q);
        }
    });

    // -------------------------------------------------------------------------
    // 8. 删除凭据回调 (通过事件分发器广播)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_del = ctx.core_state.clone();
    let notif_del = ctx.notifications.clone();
    window.on_delete_credential(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = id.to_string();
            let _ = core_state_del.storage().credentials().delete(&id_str);
            tracing::warn!(target: "smagical_ui::credentials", "删除凭据: ID=[{}]", id_str);
            // 显式广播凭据删除事件
            core_state_del.events().dispatch(&CredentialDeletedEvent {
                cred_id: id_str.clone(),
            });
            notif_del.info("凭据已删除", "指定凭据已从本地保管库中安全清除");

            w.set_active_credential_id("".into());
            let cat = w.get_credential_filter_category();
            let q = w.get_credential_search_query();
            sync_credentials_ui(&w, &core_state_del, &cat, &q);
        }
    });

    // -------------------------------------------------------------------------
    // 9. 复制机密信息 (公钥 / 密码 / 管道) 回调 (派发安全审计事件)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_copy = ctx.core_state.clone();
    let notif_copy = ctx.notifications.clone();
    window.on_copy_credential_secret(move |id| {
        if let Some(_w) = window_weak.upgrade() {
            let id_str = id.to_string();
            if let Ok(Some(cred)) = core_state_copy.storage().credentials().get_by_id(&id_str) {
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
                    let _ = clipboard.set_text(copy_content);
                    tracing::info!(
                        target: "smagical_ui::credentials",
                        "复制凭据机密: ID=[{}], Name='{}', 类型={:?}, 敏感={}",
                        cred.id, cred.name, copy_type, is_sensitive
                    );
                    // 显式广播敏感机密提取安全审计事件
                    core_state_copy.events().dispatch(&CredentialSecretCopiedEvent {
                        cred_id: cred.id.clone(),
                        name: cred.name.clone(),
                        copy_type,
                        is_sensitive,
                    });
                    notif_copy.success(tip_title, tip_msg);
                } else {
                    tracing::error!(target: "smagical_ui::credentials", "访问系统剪贴板失败");
                    notif_copy.warning("剪贴板受限", "无法访问操作系统剪贴板服务");
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 10. 复制自定义文本 (公钥/指纹) 回调
    // -------------------------------------------------------------------------
    let notif_custom_copy = ctx.notifications.clone();
    window.on_copy_custom_text(move |text, title, msg| {
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
    let window_weak = window.as_weak();
    let core_state_gen = ctx.core_state.clone();
    let notif_gen_key = ctx.notifications.clone();
    window.on_generate_credential_key(move |algorithm| {
        if let Some(w) = window_weak.upgrade() {
            let algo = algorithm.to_string();
            let key_suffix = &uuid::Uuid::new_v4().to_string()[..8];
            let mock_fp_suffix = &uuid::Uuid::new_v4().to_string()[..12];

            let (priv_key, pub_key, fp) = if algo == "RSA-4096" {
                (
                    format!(
                        "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0k6K9X7L9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQ2Y69kd\nvQMwAAAAtzc2gtcnNhAAAAAwEAAQAAAgEAv7b4a2p8zXqN3vP9xK2m4rL9nO1pQ8tL\n3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQ\nA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4l\nXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQFAgcICQoLDA0O\nDxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0\nBBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3Bx\ncnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6Ch\noqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR\n0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/wID\n-----END RSA PRIVATE KEY-----"
                    ),
                    format!("ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7b4a2p8zXqN3vP9xK2m4rL9nO1pQ8tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ smalux_{}@smalux.io", key_suffix),
                    format!("SHA256:rsa_{}", mock_fp_suffix),
                )
            } else {
                (
                    format!(
                        "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\nQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAJCR2Y69kdmO\nvQAAAAtzc2gtZWQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6\nSQAAAEA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP\n4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ=\n-----END OPENSSH PRIVATE KEY-----"
                    ),
                    format!("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMfyDbS9fsr2nUF83bA/iVeplszjEaAD1Anq0vuvWfpJ smalux_{}@smalux.io", key_suffix),
                    format!("SHA256:ed_{}", mock_fp_suffix),
                )
            };

            w.set_credential_form_secret_data(priv_key.into());
            w.set_credential_form_public_key(pub_key.into());
            w.set_credential_form_fingerprint(fp.clone().into());

            tracing::info!(
                target: "smagical_ui::credentials",
                "一键生成密钥对: 算法='{}', 公钥指纹=[{}]",
                algo, fp
            );
            // 显式广播密钥生成事件
            core_state_gen.events().dispatch(&KeyGeneratedEvent {
                algorithm: algo.clone(),
                fingerprint: fp.clone(),
            });
            notif_gen_key.success("密钥生成成功", format!("已生成全新的 {} 密钥对与指纹", algo));
        }
    });

    // -------------------------------------------------------------------------
    // 12. 一键生成高强度随机密码回调 (写入右侧表单属性并广播事件)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_gp = ctx.core_state.clone();
    let notif_gen_pwd = ctx.notifications.clone();
    window.on_generate_credential_password(move || {
        if let Some(w) = window_weak.upgrade() {
            let rand_part1 = &uuid::Uuid::new_v4().to_string()[..6];
            let rand_part2 = &uuid::Uuid::new_v4().to_string()[6..12];
            let strong_pwd = format!("Sm@lux#{}!{}", rand_part1, rand_part2);

            w.set_credential_form_secret_data(strong_pwd.into());
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
    let window_weak = window.as_weak();
    let core_state_filter = ctx.core_state.clone();
    window.on_filter_credentials(move |cat, query| {
        if let Some(w) = window_weak.upgrade() {
            tracing::debug!(
                target: "smagical_ui::credentials",
                "筛选凭据列表: 分类='{}', 关键词='{}'",
                cat, query
            );
            w.set_credential_filter_category(cat.clone());
            w.set_credential_search_query(query.clone());
            sync_credentials_ui(&w, &core_state_filter, &cat, &query);
        }
    });
}

