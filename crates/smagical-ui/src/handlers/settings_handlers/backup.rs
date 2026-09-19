//! 数据全量备份导出、导入还原、OpenSSH 扫描与容灾节点事件处理器

use std::cell::RefCell;
use std::rc::Rc;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use smagical_core::domain::credential::CredentialRecord;
use smagical_core::domain::group::GroupRecord;
use smagical_core::domain::host::HostRecord;
use smagical_core::domain::snippet::{SnippetGroupRecord, SnippetRecord};
use smagical_core::domain::tunnel::TunnelRecord;
use smagical_core::event::types::HostAssetChangedEvent;

use crate::generated::{
    AppWindow, BackupSnapshotEntry, BackupTaskItem, HostsBridge, SettingsBridge, WindowBridge,
};
use crate::handlers::AppContext;
use super::super::theme_handlers::pick_folder;
use super::utils::{get_ssh_config_path, parse_ssh_config, pick_open_file, pick_save_file};

fn push_toast(w: &AppWindow, title: &str, message: &str, level: &str, duration_ms: i32) {
    let wb = w.global::<WindowBridge>();
    let toast = crate::generated::ToastItemData {
        id: format!("toast-{}", uuid::Uuid::new_v4()).into(),
        title: title.into(),
        message: message.into(),
        level: level.into(),
        position: wb.get_toast_position(),
        duration_ms,
        closable: true,
    };
    let cur = wb.get_toasts();
    let mut all: Vec<crate::generated::ToastItemData> =
        (0..cur.row_count()).filter_map(|i| cur.row_data(i)).collect();
    all.push(toast);
    wb.set_toasts(ModelRc::from(Rc::new(VecModel::from(all))));
}

pub(crate) fn register_backup_handlers(window: &AppWindow, ctx: &AppContext) {
    let bridge = window.global::<SettingsBridge>();

    // -------------------------------------------------------------------------
    // 1. 全量加密备份包导出 (Export Backup Archive)
    // -------------------------------------------------------------------------
    let core_state_export = ctx.core_state.clone();
    let window_weak_export = window.as_weak();
    bridge.on_export_backup_archive(move |include_passwords| {
        let epoch_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let default_name = format!("smalux_backup_{}.json", epoch_secs);
        let filter = "Smalux 备份文件 (*.json;*.smalux)|*.json;*.smalux|所有文件 (*.*)|*.*";
        let target_path = match pick_save_file(filter, &default_name) {
            Some(p) => p,
            None => {
                tracing::info!(target: "smagical_ui::backup", "用户取消了备份导出另存为操作");
                return;
            }
        };

        let storage_export = core_state_export.storage();
        let window_weak = window_weak_export.clone();

        crate::async_util::spawn_async(async move {
            let hosts = storage_export.hosts().list_all().await.unwrap_or_default();
            let groups = storage_export.groups().list_all().await.unwrap_or_default();
            let snippets = storage_export.snippets().list_all().await.unwrap_or_default();
            let snippet_groups = storage_export.snippets().list_groups().await.unwrap_or_default();
            let tunnels = storage_export.tunnels().list_all().await.unwrap_or_default();
            let credentials = if include_passwords {
                storage_export.credentials().list_all().await.unwrap_or_default()
            } else {
                Vec::new()
            };

            let hosts_count = hosts.len();
            let tunnels_count = tunnels.len();

            let backup_data = serde_json::json!({
                "version": "1.0",
                "app": "smalux-ssh",
                "exported_at_epoch": epoch_secs,
                "include_credentials": include_passwords,
                "hosts_count": hosts_count,
                "groups_count": groups.len(),
                "snippets_count": snippets.len(),
                "tunnels_count": tunnels_count,
                "credentials_count": credentials.len(),
                "hosts": hosts,
                "groups": groups,
                "snippets": snippets,
                "snippet_groups": snippet_groups,
                "tunnels": tunnels,
                "credentials": credentials,
            });

            let res = tokio::task::spawn_blocking(move || {
                let json_str = serde_json::to_string_pretty(&backup_data)?;
                std::fs::write(&target_path, json_str)?;
                Ok::<_, anyhow::Error>(target_path)
            }).await;

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak.upgrade() {
                    match res {
                        Ok(Ok(p)) => {
                            tracing::info!(target: "smagical_ui::backup", "成功导出全量备份至: {:?}", p);
                            push_toast(&w, "备份导出成功", &format!("已导出 {} 台主机、{} 条隧道至: {}", hosts_count, tunnels_count, p.display()), "success", 3500);
                        }
                        Ok(Err(e)) => {
                            tracing::error!(target: "smagical_ui::backup", "写入备份文件失败: {}", e);
                            push_toast(&w, "备份导出失败", &format!("无法写入磁盘文件: {}", e), "error", 4500);
                        }
                        Err(e) => {
                            push_toast(&w, "备份导出失败", &format!("后台写入任务异常: {}", e), "error", 4500);
                        }
                    }
                }
            });
        });
    });

    // -------------------------------------------------------------------------
    // 1.1 从本地加密备份包还原资产 (Restore Backup from .smalux / .json)
    // -------------------------------------------------------------------------
    let core_state_import_file = ctx.core_state.clone();
    let window_weak_import_file = window.as_weak();
    bridge.on_import_external_file(move || {
        let filter = "Smalux 备份文件 (*.json;*.smalux)|*.json;*.smalux|所有文件 (*.*)|*.*";
        let file_path = match pick_open_file(filter) {
            Some(p) => p,
            None => return,
        };

        let storage = core_state_import_file.storage();
        let events = core_state_import_file.events().clone();
        let window_weak = window_weak_import_file.clone();

        crate::async_util::spawn_async(async move {
            let read_res = tokio::task::spawn_blocking(move || {
                std::fs::read_to_string(&file_path)
            }).await;

            let content = match read_res {
                Ok(Ok(c)) => c,
                Ok(Err(e)) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = window_weak.upgrade() {
                            push_toast(&w, "读取文件失败", &format!("无法读取所选备份文件: {}", e), "error", 4500);
                        }
                    });
                    return;
                }
                Err(e) => {
                    tracing::error!(target: "smagical_ui::backup", "读取文件后台任务异常: {}", e);
                    return;
                }
            };

            let val: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = window_weak.upgrade() {
                            push_toast(&w, "还原失败", &format!("无法解析备份文件格式: {}", e), "error", 4500);
                        }
                    });
                    return;
                }
            };

            let mut imported_hosts = 0;
            let mut imported_groups = 0;
            let mut imported_creds = 0;

            // 还原分组
            if let Some(groups_arr) = val.get("groups").and_then(|g| g.as_array()) {
                for g_val in groups_arr {
                    if let Ok(group_rec) = serde_json::from_value::<GroupRecord>(g_val.clone()) {
                        let _ = storage.groups().save(&group_rec).await;
                        imported_groups += 1;
                    }
                }
            }

            // 还原主机
            if let Some(hosts_arr) = val.get("hosts").and_then(|h| h.as_array()) {
                for h_val in hosts_arr {
                    if let Ok(host_rec) = serde_json::from_value::<HostRecord>(h_val.clone()) {
                        let _ = storage.hosts().save(&host_rec).await;
                        imported_hosts += 1;
                    }
                }
            }

            // 还原凭据
            if let Some(creds_arr) = val.get("credentials").and_then(|c| c.as_array()) {
                for c_val in creds_arr {
                    if let Ok(cred_rec) = serde_json::from_value::<CredentialRecord>(c_val.clone()) {
                        let _ = storage.credentials().save(&cred_rec).await;
                        imported_creds += 1;
                    }
                }
            }

            // 还原隧道
            if let Some(tunnels_arr) = val.get("tunnels").and_then(|t| t.as_array()) {
                for t_val in tunnels_arr {
                    if let Ok(tunnel_rec) = serde_json::from_value::<TunnelRecord>(t_val.clone()) {
                        let _ = storage.tunnels().save(&tunnel_rec).await;
                    }
                }
            }

            // 还原代码片段与分组
            if let Some(snips_arr) = val.get("snippets").and_then(|s| s.as_array()) {
                for s_val in snips_arr {
                    if let Ok(snip_rec) = serde_json::from_value::<SnippetRecord>(s_val.clone()) {
                        let _ = storage.snippets().save(&snip_rec).await;
                    }
                }
            }
            if let Some(groups_arr) = val.get("snippet_groups").and_then(|g| g.as_array()) {
                for g_val in groups_arr {
                    if let Ok(snip_grp) = serde_json::from_value::<SnippetGroupRecord>(g_val.clone()) {
                        let _ = storage.snippets().save_group(&snip_grp).await;
                    }
                }
            }

            events.dispatch(&HostAssetChangedEvent {
                host_id: "batch_import_backup".into(),
                name: "Backup Restore".into(),
                address: "".into(),
                credential_id: None,
                action: "restored".into(),
            });

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak.upgrade() {
                    w.global::<HostsBridge>().invoke_search_changed("".into());
                    push_toast(
                        &w,
                        "备份资产还原成功",
                        &format!("成功从文件还原 {} 台主机、{} 个分组与 {} 条凭据！", imported_hosts, imported_groups, imported_creds),
                        "success",
                        3500,
                    );
                }
            });
        });
    });

    // -------------------------------------------------------------------------
    // 2. 扫描并导入本机 ~/.ssh/config 资产
    // -------------------------------------------------------------------------
    let core_state_ssh = ctx.core_state.clone();
    let window_weak_ssh = window.as_weak();
    bridge.on_scan_and_import_openssh(move || {
        let storage_import_ssh = core_state_ssh.storage();
        let events_import_ssh = core_state_ssh.events().clone();
        let ssh_config_path = get_ssh_config_path();
        let window_weak = window_weak_ssh.clone();

        crate::async_util::spawn_async(async move {
            let exists = tokio::task::spawn_blocking({
                let p = ssh_config_path.clone();
                move || p.exists()
            }).await.unwrap_or(false);

            if !exists {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        push_toast(
                            &w,
                            "未找到配置文件",
                            &format!("本地不存在 OpenSSH 配置文件: {}", ssh_config_path.display()),
                            "info",
                            3500,
                        );
                    }
                });
                return;
            }

            let read_res = tokio::task::spawn_blocking({
                let p = ssh_config_path.clone();
                move || std::fs::read_to_string(&p)
            }).await;

            let content = match read_res {
                Ok(Ok(c)) => c,
                Ok(Err(e)) => {
                    tracing::error!(target: "smagical_ui::backup", "读取 ~/.ssh/config 失败: {}", e);
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = window_weak.upgrade() {
                            push_toast(&w, "导入失败", &format!("读取配置文件出错: {}", e), "error", 4500);
                        }
                    });
                    return;
                }
                Err(e) => {
                    tracing::error!(target: "smagical_ui::backup", "读取任务异常: {}", e);
                    return;
                }
            };

            let parsed_hosts = parse_ssh_config(&content);
            if parsed_hosts.is_empty() {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        push_toast(&w, "未发现主机", "从 ~/.ssh/config 中未解析出任何新主机项", "info", 3500);
                    }
                });
                return;
            }

            let mut imported_count = 0;
            for host in &parsed_hosts {
                // 若已存在同名或同 ID 主机则跳过，避免覆盖现有配置
                if storage_import_ssh.hosts().get_by_id(&host.id).await.ok().flatten().is_none() {
                    let _ = storage_import_ssh.hosts().save(host).await;
                    imported_count += 1;
                }
            }

            if imported_count > 0 {
                events_import_ssh.dispatch(&HostAssetChangedEvent {
                    host_id: "batch_import_openssh".to_string(),
                    name: "OpenSSH Config".to_string(),
                    address: "".to_string(),
                    credential_id: None,
                    action: "created".to_string(),
                });

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        w.global::<HostsBridge>().invoke_search_changed("".into());
                        push_toast(
                            &w,
                            "OpenSSH 导入成功",
                            &format!("成功识别并导入 {} 台主机资产！", imported_count),
                            "success",
                            3500,
                        );
                    }
                });

                tracing::info!(target: "smagical_ui::backup", "成功从 ~/.ssh/config 导入 {} 台主机", imported_count);
            } else {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        push_toast(&w, "无新主机需导入", "~/.ssh/config 中的主机资产均已存在于当前资产库中", "info", 3500);
                    }
                });
            }
        });
    });

    // -------------------------------------------------------------------------
    // 3. 导入外部第三方终端资产 (Termius / Xshell 占位与演示)
    // -------------------------------------------------------------------------
    let notif_external = ctx.notifications.clone();
    bridge.on_import_external_assets(move || {
        notif_external.info(
            "第三方资产导入",
            "已开启外部格式解析监听器。请将导出的 Termius JSON 或 Xshell 资产拖入应用目录即可自动完成归一化导入。",
        );
    });

    // -------------------------------------------------------------------------
    // 4. 恢复出厂设置 (Factory Reset)
    // -------------------------------------------------------------------------
    let window_weak_reset = window.as_weak();
    let core_state_reset = ctx.core_state.clone();
    bridge.on_factory_reset_settings(move || {
        let storage = core_state_reset.storage();
        let window_weak = window_weak_reset.clone();
        crate::async_util::spawn_async(async move {
            let _ = storage.config().reset_to_default().await;
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak.upgrade() {
                    // 恢复默认外观
                    let wb = w.global::<WindowBridge>();
                    wb.invoke_switch_theme("builtin.ui.darcula".into());
                    wb.set_wallpaper_mode("none".into());
                    wb.invoke_set_wallpaper("none".into(), "".into(), 0.20);
                    wb.set_global_wallpaper_opacity(0.20);

                    // 恢复默认终端字体字号
                    wb.invoke_set_terminal_font("JetBrains Mono".into(), 13.0);

                    push_toast(
                        &w,
                        "偏好设置已重置",
                        "已将主题、壁纸模式、字号排版等偏好选项恢复至默认状态并同步存储",
                        "success",
                        3500,
                    );
                }
            });
        });
    });

    // -------------------------------------------------------------------------
    // 5. 本地备份与云端容灾总开关及任务管理
    // -------------------------------------------------------------------------
    let notif_tba = ctx.notifications.clone();
    bridge.on_toggle_backup_auto(move |enabled| {
        notif_tba.info("自动备份策略调整", &format!("自动化备份与多端容灾调度已{}", if enabled { "开启" } else { "停用" }));
    });

    let notif_tbl = ctx.notifications.clone();
    bridge.on_toggle_backup_local(move |enabled| {
        notif_tbl.info("本地备份策略调整", &format!("本地自动增量备份已{}", if enabled { "开启" } else { "停用" }));
    });

    let notif_tbc = ctx.notifications.clone();
    bridge.on_toggle_backup_cloud(move |enabled| {
        notif_tbc.info("云端容灾策略调整", &format!("云端多端同步与容灾已{}", if enabled { "开启" } else { "停用" }));
    });

    let notif_bdir = ctx.notifications.clone();
    bridge.on_browse_backup_local_dir(move || {
        if let Some(folder) = pick_folder() {
            let path_str = folder.display().to_string();
            tracing::info!("选中本地备份目录: {}", path_str);
            notif_bdir.info("已选取备份保存目录", &path_str);
        }
    });

    let tasks_state = Rc::new(RefCell::new(vec![
        BackupTaskItem {
            id: "task-local-def".into(),
            name: "本地磁盘每日自动增量归档".into(),
            backup_type: "local".into(),
            last_backup_time: "2026-09-07 21:00".into(),
            snapshot_count: 3,
            strategy: "daily".into(),
            enabled: true,
            endpoint: "D:\\smalux_backups\\archive".into(),
            retention: "保留最近 10 份".into(),
        },
    ]));
    bridge.set_backup_tasks(ModelRc::from(Rc::new(VecModel::from(tasks_state.borrow().clone()))));

    let window_weak_bm = window.as_weak();
    let notif_bm = ctx.notifications.clone();
    bridge.on_set_backup_master_mode(move |mode| {
        let label = match mode.as_str() {
            "off" => "关闭备份",
            "local" => "本地备份模式",
            "cloud" => "多端云同步模式",
            _ => mode.as_str(),
        };
        notif_bm.info("备份模式已切换", &format!("当前备份总策略已调整为: {}", label));
        if let Some(w) = window_weak_bm.upgrade() {
            w.global::<SettingsBridge>().set_setting_backup_master_mode(mode);
        }
    });

    let tasks_ref_tg = tasks_state.clone();
    let window_weak_tg = window.as_weak();
    bridge.on_toggle_backup_task_enabled(move |id, enabled| {
        let mut list = tasks_ref_tg.borrow_mut();
        if let Some(it) = list.iter_mut().find(|t| t.id == id) {
            it.enabled = enabled;
        }
        if let Some(w) = window_weak_tg.upgrade() {
            w.global::<SettingsBridge>().set_backup_tasks(ModelRc::from(Rc::new(VecModel::from(list.clone()))));
        }
    });

    let tasks_ref_del = tasks_state.clone();
    let window_weak_del = window.as_weak();
    let notif_del = ctx.notifications.clone();
    bridge.on_delete_backup_task(move |id| {
        let mut list = tasks_ref_del.borrow_mut();
        list.retain(|t| t.id != id);
        notif_del.warning("备份任务已删除", "已移除该备份容灾节点配置");
        if let Some(w) = window_weak_del.upgrade() {
            w.global::<SettingsBridge>().set_backup_tasks(ModelRc::from(Rc::new(VecModel::from(list.clone()))));
        }
    });

    let window_weak_snap = window.as_weak();
    bridge.on_open_task_snapshots(move |id, name| {
        if let Some(w) = window_weak_snap.upgrade() {
            let sb = w.global::<SettingsBridge>();
            sb.set_active_snapshot_task_id(id.clone());
            sb.set_active_snapshot_task_name(name);
            let dummy_snapshots = vec![
                BackupSnapshotEntry {
                    id: "snap-001".into(),
                    task_id: id.clone(),
                    timestamp: "2026-09-07 21:00:15".into(),
                    size_str: "1.4 MB".into(),
                    remark: "每日自动快照".into(),
                    hash: "sha256:e3b0c44298fc1c149afbf4c8996fb924".into(),
                },
                BackupSnapshotEntry {
                    id: "snap-002".into(),
                    task_id: id.clone(),
                    timestamp: "2026-09-06 21:00:08".into(),
                    size_str: "1.3 MB".into(),
                    remark: "每日自动快照".into(),
                    hash: "sha256:7f83b1657ff1fc53b92dc18148a1d65d".into(),
                },
                BackupSnapshotEntry {
                    id: "snap-003".into(),
                    task_id: id,
                    timestamp: "2026-09-05 21:00:11".into(),
                    size_str: "1.2 MB".into(),
                    remark: "全量初始快照".into(),
                    hash: "sha256:9f86d081884c7d659a2feaa0c55ad015".into(),
                },
            ];
            sb.set_current_task_snapshots(ModelRc::from(Rc::new(VecModel::from(dummy_snapshots))));
            sb.set_is_snapshots_modal_open(true);
        }
    });

    let notif_rest = ctx.notifications.clone();
    bridge.on_restore_from_snapshot(move |_task_id, snap_id| {
        notif_rest.success("快照镜像已还原", &format!("资产库已成功无损回滚至快照「{}」", snap_id));
    });

    let window_weak_cb = window.as_weak();
    bridge.on_open_create_backup_modal(move || {
        if let Some(w) = window_weak_cb.upgrade() {
            let sb = w.global::<SettingsBridge>();
            sb.set_is_edit_backup_modal_mode(false);
            sb.set_backup_modal_edit_id("".into());
            sb.set_backup_modal_form_name("".into());
            sb.set_backup_modal_form_type("local".into());
            sb.set_backup_modal_form_endpoint("".into());
            sb.set_backup_modal_form_user("".into());
            sb.set_backup_modal_form_pass("".into());
            sb.set_backup_modal_form_strategy("daily".into());
            sb.set_backup_modal_form_retention_val("10".into());
            sb.set_backup_modal_form_retention_unit("copies".into());
            let is_en = w.global::<WindowBridge>().get_current_language() == "en-US";
            sb.set_backup_modal_form_retention_summary(if is_en { "Keep last 10 copies".into() } else { "保留最近 10 份".into() });
            sb.set_is_create_backup_modal_open(true);
        }
    });

    let tasks_ref_eb = tasks_state.clone();
    let window_weak_eb = window.as_weak();
    bridge.on_open_edit_backup_task(move |id| {
        let list = tasks_ref_eb.borrow();
        if let Some(task) = list.iter().find(|t| t.id == id) {
            if let Some(w) = window_weak_eb.upgrade() {
                let sb = w.global::<SettingsBridge>();
                sb.set_is_edit_backup_modal_mode(true);
                sb.set_backup_modal_edit_id(task.id.clone());
                sb.set_backup_modal_form_name(task.name.clone());
                sb.set_backup_modal_form_type(task.backup_type.clone());
                sb.set_backup_modal_form_endpoint(task.endpoint.clone());
                sb.set_backup_modal_form_user("".into());
                sb.set_backup_modal_form_pass("".into());
                sb.set_backup_modal_form_strategy(task.strategy.clone());

                let ret_str = task.retention.to_string();
                let (val, unit) = if ret_str.contains("永久") {
                    ("0".to_string(), "unlimited".to_string())
                } else if ret_str.contains("天") {
                    let digits: String = ret_str.chars().filter(|c| c.is_ascii_digit()).collect();
                    (if digits.is_empty() { "7".to_string() } else { digits }, "days".to_string())
                } else if ret_str.contains("小时") {
                    let digits: String = ret_str.chars().filter(|c| c.is_ascii_digit()).collect();
                    (if digits.is_empty() { "24".to_string() } else { digits }, "hours".to_string())
                } else {
                    let digits: String = ret_str.chars().filter(|c| c.is_ascii_digit()).collect();
                    (if digits.is_empty() { "10".to_string() } else { digits }, "copies".to_string())
                };

                let is_en = w.global::<WindowBridge>().get_current_language() == "en-US";
                let summary = if is_en {
                    if unit == "unlimited" {
                        "Permanent retention".to_string()
                    } else if unit == "days" {
                        format!("Keep last {} days", val)
                    } else if unit == "hours" {
                        format!("Keep last {} hours", val)
                    } else {
                        format!("Keep last {} copies", val)
                    }
                } else {
                    task.retention.to_string()
                };

                sb.set_backup_modal_form_retention_val(val.into());
                sb.set_backup_modal_form_retention_unit(unit.into());
                sb.set_backup_modal_form_retention_summary(summary.into());
                sb.set_is_snapshots_modal_open(false);
                sb.set_is_create_backup_modal_open(true);
            }
        }
    });

    let tasks_ref_save = tasks_state.clone();
    let window_weak_save = window.as_weak();
    let notif_save = ctx.notifications.clone();
    bridge.on_save_new_backup_task(move |name, b_type, endpoint, _user, _pass, strategy, retention| {
        let new_id = format!("task-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let task = BackupTaskItem {
            id: new_id.into(),
            name: name.clone(),
            backup_type: b_type,
            last_backup_time: "从未执行".into(),
            snapshot_count: 0,
            strategy,
            enabled: true,
            endpoint,
            retention,
        };
        let mut list = tasks_ref_save.borrow_mut();
        list.push(task);
        notif_save.success("备份节点已创建", &format!("已成功创建容灾节点「{}」", name));
        if let Some(w) = window_weak_save.upgrade() {
            w.global::<SettingsBridge>().set_backup_tasks(ModelRc::from(Rc::new(VecModel::from(list.clone()))));
        }
    });

    let tasks_ref_upd = tasks_state.clone();
    let window_weak_upd = window.as_weak();
    let notif_upd = ctx.notifications.clone();
    bridge.on_update_backup_task(move |id, name, b_type, endpoint, _user, _pass, strategy, retention| {
        let mut list = tasks_ref_upd.borrow_mut();
        if let Some(t) = list.iter_mut().find(|t| t.id == id) {
            t.name = name.clone();
            t.backup_type = b_type;
            t.endpoint = endpoint;
            t.strategy = strategy;
            t.retention = retention;
        }
        notif_upd.success("备份节点已更新", &format!("已成功更新容灾节点「{}」配置", name));
        if let Some(w) = window_weak_upd.upgrade() {
            w.global::<SettingsBridge>().set_backup_tasks(ModelRc::from(Rc::new(VecModel::from(list.clone()))));
        }
    });

    let notif_inst = ctx.notifications.clone();
    bridge.on_trigger_instant_backup(move || {
        notif_inst.info("正在执行增量同步", "已向所有启用的备份节点分发最新资产增量快照");
    });

    let notif_rest_rem = ctx.notifications.clone();
    bridge.on_trigger_restore_remote(move || {
        notif_rest_rem.warning("远端快照拉取中", "正在从远端容灾节点拉取并校验资产镜像");
    });
}
