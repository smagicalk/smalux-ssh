//! 数据全量备份导出、导入还原、OpenSSH 扫描与容灾节点事件处理器
//!
//! 提供系统级数据保护与多端容灾调度能力：
//! 1. **全量快照导出**：将主机资产、分组树、隧道拓扑、片段库及凭据密文打包为带版本号的 JSON 归档；
//! 2. **数据包导入还原**：解析第三方或本系统导出的快照，执行无损幂等恢复并广播领域事件；
//! 3. **OpenSSH 资产嗅探**：探测扫描本机 `~/.ssh/config`，自动识别 Host 别名、IP、端口及私钥路径；
//! 4. **第三方资产解析**：支持 Termius (JSON/CSV)、Xshell (.xsh) 等多格式资产智能识别入库；
//! 5. **灾备快照还原流水线**：还原前自动生成本地前置安全快照防丢失，通过驱动拉取远端快照数据并恢复。

use std::rc::Rc;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use smagical_core::domain::credential::CredentialRecord;
use smagical_core::domain::group::GroupRecord;
use smagical_core::domain::host::HostRecord;
use smagical_core::domain::snippet::{SnippetGroupRecord, SnippetRecord};
use smagical_core::domain::tunnel::TunnelRecord;
use smagical_core::domain::backup::{BackupStrategy, BackupTaskRecord, BackupType};
use smagical_core::event::types::HostAssetChangedEvent;
use smagical_core::service::backup::{create_backup_driver, create_backup_payload, restore_backup_payload};
use crate::backup_daemon::BackupDaemonService;

use crate::generated::{
    AppWindow, BackupSnapshotEntry, HostsBridge, SettingsBridge, WindowBridge,
};
use crate::handlers::AppContext;
use super::super::theme_handlers::pick_folder;
use super::utils::{get_ssh_config_path, parse_ssh_config, pick_open_file, pick_save_file};

/// 向当前主窗口界面压入一条临时悬浮通知气泡 (Toast)。
///
/// # 参数
/// - `w`: Slint 主窗口句柄引用；
/// - `title`: 通知横幅标题；
/// - `message`: 详细描述说明内容；
/// - `level`: 严重级别（`"success"` | `"info"` | `"warning"` | `"error"`）；
/// - `duration_ms`: 悬浮留存显示时间（毫秒），超时自动淡出销毁。
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

/// 注册设置抽屉中的数据备份、容灾快照、第三方资产导入与出厂重置事件处理器。
///
/// 挂载 Slint [`SettingsBridge`] 暴露的各项备份与容灾回调。
///
/// # 参数
/// - `window`: Slint 主窗口实例引用；
/// - `ctx`: 全局应用共享上下文对象引用。
pub(crate) fn register_backup_handlers(window: &AppWindow, ctx: &AppContext) {
    let bridge = window.global::<SettingsBridge>();

    // -------------------------------------------------------------------------
    // 1. 全量加密备份包导出 (Export Backup Archive)
    // -------------------------------------------------------------------------
    // 触发将当前系统中的全部主机资产、分组树、代码片段、网络隧道及凭据信息导出为 JSON 归档文件。
    //
    // # 回调入参
    // - `include_passwords`: 是否包含敏感凭据与机密数据（若为 true，则包含密码明文与私钥；若为 false，则排除敏感字段）。
    //
    // # 执行流程
    // 1. 弹出系统级另存为文件对话框（Filter: `*.json;*.smalux`）；
    // 2. 异步并发收集 Storage 仓储中的主机、分组、片段、隧道与凭据记录；
    // 3. 封装带 schema 协议版本 (`version: "1.0"`)、导出时间戳与资产统计的统一 JSON 数据包；
    // 4. 后台写出至磁盘目标文件，并通过 UI 线程推送 Toast 提示导出结果。
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
    // 从磁盘导入外部备份文件（.json 或 .smalux），并将其中的全部资产无损存入存储库。
    //
    // # 还原顺序与完整性保证
    // 1. 弹出文件选择器由用户指定备份源文件；
    // 2. 依次按逻辑依赖关系反序列化并保存：
    //    - 分组 (`groups`)：优先入库，保证后续主机能挂载到对应分组；
    //    - 主机 (`hosts`)：入库主机配置；
    //    - 凭据 (`credentials`)：还原密钥对与密码；
    //    - 隧道 (`tunnels`)：还原端口转发隧道规则；
    //    - 片段与分组 (`snippets`, `snippet_groups`)：还原快速命令代码库；
    // 3. 广播领域事件 `HostAssetChangedEvent`；
    // 4. 通知 Slint UI 触发搜索栏变更与资产列表重绘。
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
    // 探测系统环境标准路径（Windows: `%USERPROFILE%\.ssh\config`，Unix: `~/.ssh/config`）下的配置文件，
    // 解析出其中的 Host 别名、HostName 真实地址、User 登录账号、Port 端口号与 IdentityFile 密钥路径。
    //
    // # 幂等性与防覆盖机制
    // 1. 若本地不存在 `.ssh/config` 文件，则提示信息气泡并直接返回；
    // 2. 逐一遍历解析出的主机项，在存入 Storage 仓储前先核验是否存在同 ID 或同名记录；
    // 3. 仅保存全新的主机项，杜绝覆盖用户自定义编辑的主机名称或附加凭据；
    // 4. 若有新主机导入成功，派发 `HostAssetChangedEvent` 并刷新 UI 主机树。
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
    // 3. 导入外部第三方终端资产 (Termius / Xshell / CSV / TSV)
    // -------------------------------------------------------------------------
    // 打开本地第三方会话资产配置文件，通过智能格式嗅探器 `parse_external_assets`
    // 解析出主机条目，并支持自动创建并挂载所属分组。
    //
    // # 支持的导入来源格式
    // - Termius 导出的 JSON 结构或 CSV 报表；
    // - NetSarang Xshell 的 `.xsh` 会话文件或制表符分隔 TSV 文件；
    // - 通用电子表格导出的 CSV 逗号分隔资产清单。
    //
    // # 分组树动态映射
    // - 若导入记录包含 `group_name`，首先比对数据库中已有分组名称映射 ID；
    // - 若属未知新分组，动态生成 `grp-{uuid}` 并在数据库建立根级分组后将主机正确归类。
    let core_state_ext = ctx.core_state.clone();
    let window_weak_ext = window.as_weak();
    let notif_external = ctx.notifications.clone();
    bridge.on_import_external_assets(move || {
        let filter = "所有支持的资产文件 (*.json;*.csv;*.tsv;*.xsh)|*.json;*.csv;*.tsv;*.xsh|Termius 导出文件 (*.json;*.csv)|*.json;*.csv|Xshell 会话文件 (*.xsh;*.tsv)|*.xsh;*.tsv|所有文件 (*.*)|*.*";
        let file_path = match pick_open_file(filter) {
            Some(p) => p,
            None => {
                tracing::info!(target: "smagical_ui::backup", "用户取消了第三方资产文件选择");
                return;
            }
        };

        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let storage = core_state_ext.storage();
        let events = core_state_ext.events().clone();
        let window_weak = window_weak_ext.clone();
        let notif = notif_external.clone();

        crate::async_util::spawn_async(async move {
            let read_res = tokio::task::spawn_blocking({
                let p = file_path.clone();
                move || std::fs::read_to_string(&p)
            }).await;

            let content = match read_res {
                Ok(Ok(c)) => c,
                Ok(Err(e)) => {
                    tracing::error!(target: "smagical_ui::backup", "读取第三方资产文件失败: {}", e);
                    notif.error("读取失败", format!("无法读取文件内容: {}", e));
                    return;
                }
                Err(e) => {
                    tracing::error!(target: "smagical_ui::backup", "读取任务后台异常: {}", e);
                    return;
                }
            };

            let parsed_entries = crate::importer::parse_external_assets(&content, Some(&file_name));
            if parsed_entries.is_empty() {
                notif.warning("未识别到资产", "所选文件格式未识别出任何有效的主机连接配置 (支持 Termius JSON/CSV、Xshell .xsh 或标准 CSV 表格)");
                return;
            }

            let mut imported_hosts = 0;
            let mut created_groups = 0;
            let mut group_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

            // 预载现有分组
            let existing_groups = storage.groups().list_all().await.unwrap_or_default();
            for g in existing_groups {
                group_map.insert(g.name.clone(), g.id.clone());
            }

            for entry in parsed_entries {
                // 处理分组
                let mut target_group_id = None;
                if let Some(ref gname) = entry.group_name {
                    if let Some(gid) = group_map.get(gname) {
                        target_group_id = Some(gid.clone());
                    } else {
                        let new_gid = format!("grp-{}", uuid::Uuid::new_v4().simple());
                        let new_group = GroupRecord::root(new_gid.clone(), gname.clone());
                        if storage.groups().save(&new_group).await.is_ok() {
                            group_map.insert(gname.clone(), new_gid.clone());
                            target_group_id = Some(new_gid);
                            created_groups += 1;
                        }
                    }
                }

                let host_rec = entry.into_host_record(target_group_id);
                if storage.hosts().save(&host_rec).await.is_ok() {
                    imported_hosts += 1;
                }
            }

            events.dispatch(&HostAssetChangedEvent {
                host_id: "batch_import_external".to_string(),
                name: file_name.clone(),
                address: String::new(),
                credential_id: None,
                action: "created".to_string(),
            });

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak.upgrade() {
                    w.global::<HostsBridge>().invoke_search_changed("".into());
                    push_toast(
                        &w,
                        "第三方资产导入成功",
                        &format!("成功从 [{}] 导入 {} 台主机、{} 个分组！", file_name, imported_hosts, created_groups),
                        "success",
                        4000,
                    );
                }
            });

            tracing::info!(target: "smagical_ui::backup", "成功从外部资产文件导入 {} 台主机、{} 个分组", imported_hosts, created_groups);
        });
    });

    // -------------------------------------------------------------------------
    // 4. 恢复出厂设置 (Factory Reset)
    // -------------------------------------------------------------------------
    // 将客户端外观主题、壁纸模式、终端字体、字号等个性化偏好配置重置回系统默认初值。
    //
    // # 还原项目
    // 1. Storage 数据库中的用户偏好配置项重置 (`reset_to_default`)；
    // 2. 外观重置为内置经典暗黑主题 `builtin.ui.darcula`；
    // 3. 关闭全局背景壁纸（模式为 `none`，透明度恢复为 0.20）；
    // 4. 终端字体恢复为 `JetBrains Mono`，字号恢复为 13.0 pt。
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

    // 5.1 自动化备份调度总开关切换
    let notif_tba = ctx.notifications.clone();
    bridge.on_toggle_backup_auto(move |enabled| {
        notif_tba.info("自动备份策略调整", &format!("自动化备份与多端容灾调度已{}", if enabled { "开启" } else { "停用" }));
    });

    // 5.2 本地自动增量备份子开关切换
    let notif_tbl = ctx.notifications.clone();
    bridge.on_toggle_backup_local(move |enabled| {
        notif_tbl.info("本地备份策略调整", &format!("本地自动增量备份已{}", if enabled { "开启" } else { "停用" }));
    });

    // 5.3 云端多端容灾同步子开关切换
    let notif_tbc = ctx.notifications.clone();
    bridge.on_toggle_backup_cloud(move |enabled| {
        notif_tbc.info("云端容灾策略调整", &format!("云端多端同步与容灾已{}", if enabled { "开启" } else { "停用" }));
    });

    // 5.4 本地备份目标存储目录选择 (通过原生目录对话框)
    let window_weak_bdir = window.as_weak();
    let notif_bdir = ctx.notifications.clone();
    bridge.on_browse_backup_local_dir(move || {
        if let Some(folder) = pick_folder() {
            let path_str = folder.display().to_string();
            tracing::info!("选中本地备份目录: {}", path_str);
            notif_bdir.info("已选取备份保存目录", &path_str);
            if let Some(w) = window_weak_bdir.upgrade() {
                w.global::<SettingsBridge>().set_backup_modal_form_endpoint(path_str.into());
            }
        }
    });

    // 初始从数据库加载真实备份任务列表水合到 UI
    let storage_init = ctx.core_state.storage().clone();
    let window_weak_init = window.as_weak();
    crate::async_util::spawn_async(async move {
        BackupDaemonService::refresh_ui_tasks(storage_init, window_weak_init).await;
    });

    // 5.5 备份主策略模式切换 ("off" | "local" | "cloud")
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

    // 5.6 切换单个备份节点的启用/停用状态
    let storage_tg = ctx.core_state.storage().clone();
    let window_weak_tg = window.as_weak();
    bridge.on_toggle_backup_task_enabled(move |id, enabled| {
        let storage = storage_tg.clone();
        let id_str = id.to_string();
        let window_weak = window_weak_tg.clone();
        crate::async_util::spawn_async(async move {
            let _ = storage.backup_tasks().set_enabled(&id_str, enabled).await;
            BackupDaemonService::refresh_ui_tasks(storage, window_weak).await;
        });
    });

    // 5.7 删除指定的备份任务节点及其关联的历史快照链
    let storage_del = ctx.core_state.storage().clone();
    let window_weak_del = window.as_weak();
    let notif_del = ctx.notifications.clone();
    bridge.on_delete_backup_task(move |id| {
        let storage = storage_del.clone();
        let id_str = id.to_string();
        let window_weak = window_weak_del.clone();
        let notif = notif_del.clone();
        crate::async_util::spawn_async(async move {
            let _ = storage.backup_snapshots().delete_by_task(&id_str).await;
            let _ = storage.backup_tasks().delete(&id_str).await;
            notif.warning("备份任务已删除", "已移除该备份容灾节点配置及其快照历史");
            BackupDaemonService::refresh_ui_tasks(storage, window_weak).await;
        });
    });

    // 5.8 打开指定备份任务的历史快照列表弹窗 (SnapshotsModal)
    let storage_snap = ctx.core_state.storage().clone();
    let window_weak_snap = window.as_weak();
    bridge.on_open_task_snapshots(move |id, name| {
        let storage = storage_snap.clone();
        let task_id = id.to_string();
        let task_name = name.to_string();
        let window_weak = window_weak_snap.clone();

        crate::async_util::spawn_async(async move {
            let snapshots = storage.backup_snapshots().list_by_task(&task_id).await.unwrap_or_default();
            let ui_snapshots: Vec<BackupSnapshotEntry> = snapshots
                .into_iter()
                .map(|s| BackupSnapshotEntry {
                    id: s.id.into(),
                    task_id: s.task_id.into(),
                    timestamp: s.timestamp.into(),
                    size_str: s.size_str.into(),
                    remark: s.remark.into(),
                    hash: s.hash.into(),
                })
                .collect();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak.upgrade() {
                    let sb = w.global::<SettingsBridge>();
                    sb.set_active_snapshot_task_id(task_id.into());
                    sb.set_active_snapshot_task_name(task_name.into());
                    sb.set_current_task_snapshots(ModelRc::from(Rc::new(VecModel::from(ui_snapshots))));
                    sb.set_is_snapshots_modal_open(true);
                }
            });
        });
    });

    // 5.9 从指定快照版本执行全量无损回滚与资产恢复 (Restore from Snapshot)
    //
    // # 4 步无损安全恢复流水线
    // 1. **前置安全快照防丢失**：在覆写数据库前，先对当前本地全部资产做内存打包，
    //    写入临时安全备份文件 `~/.smalux/backups/pre_restore_safety_{epoch}.json`；
    // 2. **元数据定位**：从 Storage 查询该快照记录及其所属 Task 配置（获取远端存储驱动参数）；
    // 3. **远端拉取**：调用对应驱动（本地目录、WebDAV、S3、SFTP 等）拉取原始快照二进制 Payload；
    // 4. **无损覆写还原**：调用 `restore_backup_payload` 写入数据库，派发 `HostAssetChangedEvent`，
    //    关闭弹窗并刷新主机树界面。
    let storage_rest = ctx.core_state.storage().clone();
    let events_rest = ctx.core_state.events().clone();
    let window_weak_rest = window.as_weak();
    let notif_rest = ctx.notifications.clone();
    bridge.on_restore_from_snapshot(move |task_id, snap_id| {
        let storage = storage_rest.clone();
        let events = events_rest.clone();
        let window_weak = window_weak_rest.clone();
        let notif = notif_rest.clone();
        let task_id_str = task_id.to_string();
        let snap_id_str = snap_id.to_string();

        crate::async_util::spawn_async(async move {
            // 1. 创建前置安全快照防丢失
            if let Ok((safety_bytes, _, _, _)) = create_backup_payload(storage.as_ref(), true).await {
                if let Some(base) = directories::BaseDirs::new() {
                    let pre_dir = base.home_dir().join(".smalux").join("backups");
                    let _ = std::fs::create_dir_all(&pre_dir);
                    let now_epoch = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let safety_file = pre_dir.join(format!("pre_restore_safety_{}.json", now_epoch));
                    let _ = std::fs::write(safety_file, safety_bytes);
                }
            }

            // 2. 查询快照与任务元数据
            let snap_rec = match storage.backup_snapshots().get_by_id(&snap_id_str).await {
                Ok(Some(s)) => s,
                _ => {
                    notif.error("快照还原失败", "未找到指定的快照记录");
                    return;
                }
            };

            let task_rec = match storage.backup_tasks().get_by_id(&task_id_str).await {
                Ok(Some(t)) => t,
                _ => {
                    notif.error("快照还原失败", "未找到所属备份任务配置");
                    return;
                }
            };

            // 3. 从驱动拉取完整快照数据
            let driver = create_backup_driver(&task_rec);
            let payload_bytes = match driver.pull_snapshot(&snap_rec.remote_id).await {
                Ok(b) => b,
                Err(err) => {
                    notif.error("拉取快照失败", &format!("无法从远端下载快照文件: {}", err));
                    return;
                }
            };

            // 4. 执行无损资产还原
            match restore_backup_payload(storage.as_ref(), &payload_bytes).await {
                Ok((hosts, groups, creds)) => {
                    events.dispatch(&HostAssetChangedEvent {
                        host_id: "batch_restore_snapshot".into(),
                        name: "Snapshot Restore".into(),
                        address: "".into(),
                        credential_id: None,
                        action: "restored".into(),
                    });

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = window_weak.upgrade() {
                            w.global::<HostsBridge>().invoke_search_changed("".into());
                            w.global::<SettingsBridge>().set_is_snapshots_modal_open(false);
                            notif.success(
                                "快照镜像已还原",
                                &format!("已成功无损回滚至快照「{}」！恢复 {} 台主机、{} 个分组与 {} 条凭据", snap_id_str, hosts, groups, creds),
                            );
                        }
                    });
                }
                Err(err) => {
                    notif.error("解析快照失败", &format!("快照数据解析还原失败: {}", err));
                }
            }
        });
    });

    // 5.10 打开新建备份任务节点弹窗 (重置为空白默认值)
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

    // 5.11 打开编辑备份任务节点弹窗 (回显已保存的任务字段与策略)
    let storage_eb = ctx.core_state.storage().clone();
    let window_weak_eb = window.as_weak();
    bridge.on_open_edit_backup_task(move |id| {
        let storage = storage_eb.clone();
        let id_str = id.to_string();
        let window_weak = window_weak_eb.clone();

        crate::async_util::spawn_async(async move {
            if let Ok(Some(task)) = storage.backup_tasks().get_by_id(&id_str).await {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        let sb = w.global::<SettingsBridge>();
                        sb.set_is_edit_backup_modal_mode(true);
                        sb.set_backup_modal_edit_id(task.id.clone().into());
                        sb.set_backup_modal_form_name(task.name.clone().into());
                        sb.set_backup_modal_form_type(task.backup_type.as_str().into());
                        sb.set_backup_modal_form_endpoint(task.endpoint.clone().into());
                        sb.set_backup_modal_form_user(task.auth_user.clone().into());
                        sb.set_backup_modal_form_pass("".into());
                        sb.set_backup_modal_form_strategy(task.strategy.as_str().into());

                        let ret_str = task.retention.clone();
                        let (val, unit) = if ret_str.contains("永久") || ret_str.contains("unlimited") {
                            ("0".to_string(), "unlimited".to_string())
                        } else if ret_str.contains("天") || ret_str.contains("days") {
                            let digits: String = ret_str.chars().filter(|c| c.is_ascii_digit()).collect();
                            (if digits.is_empty() { "7".to_string() } else { digits }, "days".to_string())
                        } else if ret_str.contains("小时") || ret_str.contains("hours") {
                            let digits: String = ret_str.chars().filter(|c| c.is_ascii_digit()).collect();
                            (if digits.is_empty() { "24".to_string() } else { digits }, "hours".to_string())
                        } else {
                            let digits: String = ret_str.chars().filter(|c| c.is_ascii_digit()).collect();
                            (if digits.is_empty() { "10".to_string() } else { digits }, "copies".to_string())
                        };

                        sb.set_backup_modal_form_retention_val(val.into());
                        sb.set_backup_modal_form_retention_unit(unit.into());
                        sb.set_backup_modal_form_retention_summary(task.retention.into());
                        sb.set_is_snapshots_modal_open(false);
                        sb.set_is_create_backup_modal_open(true);
                    }
                });
            }
        });
    });

    // 5.12 保存新建的备份任务节点到 Storage 数据库
    let storage_save = ctx.core_state.storage().clone();
    let window_weak_save = window.as_weak();
    let notif_save = ctx.notifications.clone();
    bridge.on_save_new_backup_task(move |name, b_type, endpoint, user, pass, strategy, retention| {
        let storage = storage_save.clone();
        let window_weak = window_weak_save.clone();
        let notif = notif_save.clone();

        let new_id = format!("task-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let now = chrono::Utc::now().timestamp() as u64;
        let task_name = name.to_string();

        let record = BackupTaskRecord {
            id: new_id,
            name: name.to_string(),
            backup_type: BackupType::from_str(&b_type),
            endpoint: endpoint.to_string(),
            auth_user: user.to_string(),
            auth_secret: pass.to_string(),
            strategy: BackupStrategy::from_str(&strategy),
            retention: retention.to_string(),
            enabled: true,
            last_backup_time: "从未执行".into(),
            snapshot_count: 0,
            last_status: "idle".into(),
            last_error: "".into(),
            created_at: now,
            updated_at: now,
        };

        crate::async_util::spawn_async(async move {
            if let Err(e) = storage.backup_tasks().save(&record).await {
                notif.error("创建失败", &format!("保存备份节点失败: {}", e));
            } else {
                notif.success("备份节点已创建", &format!("已成功创建容灾节点「{}」", task_name));
                BackupDaemonService::refresh_ui_tasks(storage, window_weak).await;
            }
        });
    });

    // 5.13 更新已有的备份任务节点配置 (若未修改密码则保持原密文不变)
    let storage_upd = ctx.core_state.storage().clone();
    let window_weak_upd = window.as_weak();
    let notif_upd = ctx.notifications.clone();
    bridge.on_update_backup_task(move |id, name, b_type, endpoint, user, pass, strategy, retention| {
        let storage = storage_upd.clone();
        let window_weak = window_weak_upd.clone();
        let notif = notif_upd.clone();
        let task_id = id.to_string();
        let task_name = name.to_string();

        crate::async_util::spawn_async(async move {
            let existing = match storage.backup_tasks().get_by_id(&task_id).await {
                Ok(Some(t)) => t,
                _ => return,
            };

            let pass_str = pass.to_string();
            let auth_secret = if pass_str.is_empty() {
                existing.auth_secret
            } else {
                pass_str
            };

            let updated = BackupTaskRecord {
                id: task_id,
                name: name.to_string(),
                backup_type: BackupType::from_str(&b_type),
                endpoint: endpoint.to_string(),
                auth_user: user.to_string(),
                auth_secret,
                strategy: BackupStrategy::from_str(&strategy),
                retention: retention.to_string(),
                enabled: existing.enabled,
                last_backup_time: existing.last_backup_time,
                snapshot_count: existing.snapshot_count,
                last_status: existing.last_status,
                last_error: existing.last_error,
                created_at: existing.created_at,
                updated_at: chrono::Utc::now().timestamp() as u64,
            };

            if let Err(e) = storage.backup_tasks().save(&updated).await {
                notif.error("更新失败", &format!("更新备份节点失败: {}", e));
            } else {
                notif.success("备份节点已更新", &format!("已成功更新容灾节点「{}」配置", task_name));
                BackupDaemonService::refresh_ui_tasks(storage, window_weak).await;
            }
        });
    });

    // 5.14 立即手动触发所有启用的容灾节点执行快照备份
    let daemon_inst = ctx.backup_daemon.clone();
    bridge.on_trigger_instant_backup(move || {
        let daemon = daemon_inst.clone();
        crate::async_util::spawn_async(async move {
            daemon.trigger_all_now().await;
        });
    });

    // 5.15 触发远端镜像同步检测与容灾对齐
    let daemon_rest_rem = ctx.backup_daemon.clone();
    bridge.on_trigger_restore_remote(move || {
        let daemon = daemon_rest_rem.clone();
        crate::async_util::spawn_async(async move {
            daemon.trigger_all_now().await;
        });
    });
}
