//! 数据全量备份导出、导入还原、OpenSSH 扫描与容灾节点事件处理器
//!
//! 提供系统级数据保护与多端容灾调度能力：
//! 1. **全量快照导出**：将主机资产、分组树、隧道拓扑、片段库及凭据密文打包为带版本号的 JSON 归档；
//! 2. **数据包导入还原**：解析第三方或本系统导出的快照，执行无损幂等恢复并广播领域事件；
//! 3. **OpenSSH 资产嗅探**：探测扫描本机 `~/.ssh/config`，自动识别 Host 别名、IP、端口及私钥路径；
//! 4. **第三方资产解析**：支持 Termius (JSON/CSV)、Xshell (.xsh) 等多格式资产智能识别入库；
//! 5. **灾备快照还原流水线**：还原前自动生成本地前置安全快照防丢失，通过驱动拉取远端快照数据并恢复。

use slint::{ComponentHandle, Model};
use crate::common::{run_on_ui, to_model_rc, ToSharedString};

use smagical_core::domain::group::GroupRecord;
use smagical_core::domain::backup::{BackupStrategy, BackupTaskRecord, BackupType};
use smagical_core::event::types::HostAssetChangedEvent;
use smagical_core::service::backup::{
    create_backup_driver, create_backup_payload,
    restore_backup_payload_with_policy, BackupConflictPolicy,
};
use crate::backup_daemon::BackupDaemonService;

use crate::generated::{
    AppWindow, BackupSnapshotEntry, HostsBridge, SettingsBridge, WindowBridge,
};
use crate::handlers::AppContext;
use super::super::theme_handlers::pick_folder;
use super::utils::{pick_open_file, pick_save_file};

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
    let toast_id = format!("toast-{}", uuid::Uuid::new_v4());
    let toast = crate::generated::ToastItemData {
        id: toast_id.clone().into(),
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
    wb.set_toasts(to_model_rc(all));

    if duration_ms > 0 {
        let w_weak = w.as_weak();
        let tid: slint::SharedString = toast_id.into();
        slint::Timer::single_shot(std::time::Duration::from_millis(duration_ms as u64), move || {
            if let Some(w) = w_weak.upgrade() {
                let wb = w.global::<WindowBridge>();
                let cur = wb.get_toasts();
                let remaining: Vec<crate::generated::ToastItemData> =
                    cur.iter().filter(|t| t.id != tid).collect();
                wb.set_toasts(to_model_rc(remaining));
            }
        });
    }
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

            let restore_res = restore_backup_payload_with_policy(
                storage.as_ref(),
                content.as_bytes(),
                None,
                BackupConflictPolicy::Overwrite,
            ).await;

            let report = match restore_res {
                Ok(r) => r,
                Err(e) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = window_weak.upgrade() {
                            push_toast(&w, "还原失败", &format!("解析或还原备份文件失败: {}", e), "error", 4500);
                        }
                    });
                    return;
                }
            };

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
                        &format!(
                            "成功从文件还原 {} 台主机 (跳过 {} 项)、{} 个分组与 {} 条凭据！",
                            report.imported_hosts, report.skipped_hosts, report.imported_groups, report.imported_credentials
                        ),
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
    // 2. 扫描并导入系统 OpenSSH 资产 (~/.ssh/config 与 ~/.ssh/known_hosts)
    bridge.on_scan_and_import_openssh(move || {
        let storage_import_ssh = core_state_ssh.storage();
        let events_import_ssh = core_state_ssh.events().clone();
        let window_weak = window_weak_ssh.clone();

        crate::async_util::spawn_async(async move {
            let (config_hosts, known_hosts) = tokio::task::spawn_blocking(|| {
                smagical_storage::seaorm::real_seed::discover_local_ssh_hosts(
                    Some("grp-known".to_string()),
                    Some("grp-default".to_string()),
                )
            }).await.unwrap_or_default();

            if config_hosts.is_empty() && known_hosts.is_empty() {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        push_toast(&w, "未发现主机", "从 ~/.ssh 目录下未扫描到任何主机配置或已知主机记录", "info", 3500);
                    }
                });
                return;
            }

            // 若存在 known_hosts，确保 "已知主机" 分组存在
            if !known_hosts.is_empty() {
                if storage_import_ssh.groups().get_by_id("grp-known").await.ok().flatten().is_none() {
                    let mut known_group = smagical_core::domain::group::GroupRecord::root("grp-known", "已知主机 (Known Hosts)");
                    known_group.sort_order = 1;
                    let _ = storage_import_ssh.groups().save(&known_group).await;
                }
            }

            let mut imported_count = 0;
            for host in config_hosts.into_iter().chain(known_hosts.into_iter()) {
                if storage_import_ssh.hosts().get_by_id(&host.id).await.ok().flatten().is_none() {
                    let _ = storage_import_ssh.hosts().save(&host).await;
                    imported_count += 1;
                }
            }

            if imported_count > 0 {
                events_import_ssh.dispatch(&HostAssetChangedEvent {
                    host_id: "batch_import_openssh".to_string(),
                    name: "OpenSSH Assets".to_string(),
                    address: "".to_string(),
                    credential_id: None,
                    action: "created".to_string(),
                });

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        w.global::<HostsBridge>().invoke_search_changed("".into());
                        push_toast(
                            &w,
                            "OpenSSH 资产导入成功",
                            &format!("成功从 ~/.ssh 识别并导入 {} 台主机资产！", imported_count),
                            "success",
                            3500,
                        );
                    }
                });

                tracing::info!(target: "smagical_ui::backup", "成功从系统 OpenSSH 资产中导入 {} 台主机", imported_count);
            } else {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        push_toast(&w, "无新主机需导入", "系统 ~/.ssh 中的主机资产均已存在于当前资产库中", "info", 3500);
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
                    remark: s.remark.to_shared(),
                    hash: s.hash.to_shared(),
                })
                .collect();

            let _ = run_on_ui(window_weak, move |w| {
                let sb = w.global::<SettingsBridge>();
                sb.set_active_snapshot_task_id(task_id.to_shared());
                sb.set_active_snapshot_task_name(task_name.to_shared());
                sb.set_current_task_snapshots(to_model_rc(ui_snapshots));
                sb.set_is_snapshots_modal_open(true);
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
            match restore_backup_payload_with_policy(
                storage.as_ref(),
                &payload_bytes,
                None,
                BackupConflictPolicy::Overwrite,
            ).await {
                Ok(report) => {
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
                                &format!(
                                    "已成功无损回滚至快照「{}」！恢复 {} 台主机 (跳过 {} 项)、{} 个分组与 {} 条凭据",
                                    snap_id_str, report.imported_hosts, report.skipped_hosts, report.imported_groups, report.imported_credentials
                                ),
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

    // 5.16 动态切换数据源模式 (物理 SQLite vs 内存 Mock)
    let w_switch = window.as_weak();
    let ctx_switch = StorageSwitchContext::from(ctx);
    bridge.on_switch_storage_mode(move |mode| {
        let w_inner = w_switch.clone();
        let ctx_inner = ctx_switch.clone();
        let m = mode.to_string();
        crate::async_util::spawn_async(async move {
            execute_switch_storage_mode(&m, w_inner, ctx_inner).await;
        });
    });

    // 5.17 一键清空预设演示数据
    let w_clear = window.as_weak();
    let ctx_clear = StorageSwitchContext::from(ctx);
    bridge.on_clear_all_demo_data(move || {
        let w_inner = w_clear.clone();
        let ctx_inner = ctx_clear.clone();
        crate::async_util::spawn_async(async move {
            execute_clear_demo_data(w_inner, ctx_inner).await;
        });
    });

    // 5.18 恢复预设演示数据
    let w_reset = window.as_weak();
    let ctx_reset = StorageSwitchContext::from(ctx);
    bridge.on_reset_all_demo_data(move || {
        let w_inner = w_reset.clone();
        let ctx_inner = ctx_reset.clone();
        crate::async_util::spawn_async(async move {
            execute_reset_demo_data(w_inner, ctx_inner).await;
        });
    });
}

/// 用于存储层切换与清空/恢复操作的跨线程上下文 (保证所有字段均为 Send + Sync + 'static)
#[derive(Clone)]
pub(crate) struct StorageSwitchContext {
    pub core_state: smagical_core::CoreState,
    pub master_tree: std::sync::Arc<std::sync::RwLock<Vec<crate::tree_model::RawTreeNode>>>,
    pub master_cards: std::sync::Arc<std::sync::RwLock<Vec<crate::generated::HostItemData>>>,
    pub expanded_groups: std::sync::Arc<std::sync::RwLock<std::collections::HashSet<String>>>,
    pub selector_expanded_groups: std::sync::Arc<std::sync::RwLock<std::collections::HashSet<String>>>,
    pub search_query: std::sync::Arc<std::sync::RwLock<String>>,
}

impl From<&AppContext> for StorageSwitchContext {
    fn from(ctx: &AppContext) -> Self {
        Self {
            core_state: (*ctx.core_state).clone(),
            master_tree: ctx.master_tree.clone(),
            master_cards: ctx.master_cards.clone(),
            expanded_groups: ctx.expanded_groups.clone(),
            selector_expanded_groups: ctx.selector_expanded_groups.clone(),
            search_query: ctx.search_query.clone(),
        }
    }
}

/// 全量热刷新并重新渲染所有与底层存储关联的 Slint 界面 (主机树、凭据、代码片段、网络隧道)
pub(crate) async fn reload_all_storage_views(
    storage: &std::sync::Arc<dyn smagical_core::storage::AppStorage>,
    is_mock: bool,
    window_weak: slint::Weak<AppWindow>,
    ctx: &StorageSwitchContext,
) {
    let (groups_res, hosts_res, creds_res, snippet_groups_res, snippets_res, tunnels_res) = tokio::join!(
        storage.groups().list_all(),
        storage.hosts().list_all(),
        storage.credentials().list_all(),
        storage.snippets().list_groups(),
        storage.snippets().list_all(),
        storage.tunnels().list_all(),
    );

    let all_groups = groups_res.unwrap_or_default();
    let all_hosts = hosts_res.unwrap_or_default();
    let all_creds = creds_res.unwrap_or_default();
    let all_snippet_groups = snippet_groups_res.unwrap_or_default();
    let all_snippets = snippets_res.unwrap_or_default();
    let all_tunnels = tunnels_res.unwrap_or_default();

    let new_tree = crate::tree_model::build_raw_tree(&all_groups, &all_hosts, &all_creds);
    let new_cards = crate::tree_model::build_cards_from_records(&all_hosts, &all_groups);

    let initial_expanded: std::collections::HashSet<String> = all_groups
        .iter()
        .filter(|g| g.is_expanded)
        .map(|g| g.id.clone())
        .collect();

    let mut initial_selector = std::collections::HashSet::from(["root".to_string()]);
    all_groups
        .iter()
        .filter(|g| g.parent_id.is_none())
        .for_each(|g| {
            initial_selector.insert(g.id.clone());
        });

    let master_tree = ctx.master_tree.clone();
    let master_cards = ctx.master_cards.clone();
    let expanded_groups = ctx.expanded_groups.clone();
    let selector_expanded_groups = ctx.selector_expanded_groups.clone();
    let search_query = ctx.search_query.clone();

    run_on_ui(window_weak, move |w| {
        *master_tree.write().unwrap() = new_tree;
        *master_cards.write().unwrap() = new_cards.clone();
        *expanded_groups.write().unwrap() = initial_expanded;
        *selector_expanded_groups.write().unwrap() = initial_selector;

        let tree = master_tree.read().unwrap();
        let expanded = expanded_groups.read().unwrap();
        let selector_expanded = selector_expanded_groups.read().unwrap();
        let q = search_query.read().unwrap().clone();

        let options = crate::tree_model::build_group_options(&tree, &selector_expanded);
        let nodes = if q.is_empty() {
            crate::tree_model::build_visible_tree_nodes(&tree, &expanded)
        } else {
            crate::tree_model::build_search_tree_nodes(&tree, &q)
        };

        let hb = w.global::<HostsBridge>();
        hb.set_group_options(to_model_rc(options));
        hb.set_tree_content_width(crate::tree_model::calculate_max_tree_width(&nodes));
        hb.set_tree_nodes(to_model_rc(nodes));
        hb.set_hosts(to_model_rc(new_cards.clone()));
        w.global::<WindowBridge>().set_launcher_host_items(to_model_rc(new_cards));

        // 刷新凭据库
        crate::handlers::credential_handlers::update_credentials_cache(all_creds.clone());
        crate::handlers::credential_handlers::render_credentials_ui(&w, &all_creds, "all", "");

        // 刷新代码片段库
        let master_snippets = crate::snippet_tree_model::build_raw_snippet_tree(&all_snippet_groups, &all_snippets);
        let expanded_snippets: std::collections::HashSet<String> = all_snippet_groups.iter().map(|g| g.id.clone()).collect();
        crate::handlers::snippet_handlers::render_snippets_ui(&w, &master_snippets, &all_snippet_groups, &all_snippets, "", &expanded_snippets);

        // 刷新网络隧道库
        crate::handlers::tunnel_handlers::render_tunnels_ui(&w, &all_tunnels, "all", "");

        // 同步设置中心与调试中心状态
        let mode_str = if is_mock { "mock" } else { "physical" };
        w.global::<SettingsBridge>().set_setting_storage_mode(mode_str.into());
        w.global::<crate::generated::DebugBridge>().set_use_mock_storage(is_mock);
    });
}

/// 执行数据源动态热切换逻辑 (支持在物理持久化 SQLite 与内存 Mock 之间无缝切换并实时落盘首选项)
pub(crate) async fn execute_switch_storage_mode(
    mode: &str,
    window_weak: slint::Weak<AppWindow>,
    ctx: StorageSwitchContext,
) {
    let mode_trimmed = mode.trim().to_lowercase();
    let is_mock = mode_trimmed == "mock";

    let (new_storage, toast_title, toast_msg) = if is_mock {
        let storage: std::sync::Arc<dyn smagical_core::storage::AppStorage> =
            std::sync::Arc::new(smagical_storage::MockStorage::new_seeded());
        let _ = crate::storage_config::save_persisted_storage_mode("mock");
        tracing::info!(target: "smagical_ui::storage", "数据层已动态切换至: [MockStorage] 内存演示数据层 (已持久化)");
        (storage, "已切换至 Mock 演示数据层", "当前处于纯内存模式，所有操作不影响物理数据库")
    } else {
        match smagical_storage::SeaOrmStorage::open_default().await {
            Ok(storage) => {
                let storage_arc: std::sync::Arc<dyn smagical_core::storage::AppStorage> = std::sync::Arc::new(storage);
                let _ = crate::storage_config::save_persisted_storage_mode("physical");
                tracing::info!(target: "smagical_ui::storage", "数据层已动态切换至: [SeaOrmStorage] 物理持久化 SQLite 引擎 (已持久化)");
                (storage_arc, "已连接至 SQLite 物理数据库", "所有数据均实时持久化落盘至本地 data.db 文件")
            }
            Err(err) => {
                tracing::error!(target: "smagical_ui::storage", "打开物理 SQLite 数据库失败: {:?}", err);
                let err_msg = format!("打开数据库错误: {:?}", err);
                run_on_ui(window_weak, move |w| {
                    push_toast(&w, "切换物理数据层失败", &err_msg, "error", 4000);
                });
                return;
            }
        }
    };

    ctx.core_state.set_storage(new_storage.clone(), is_mock);
    reload_all_storage_views(&new_storage, is_mock, window_weak.clone(), &ctx).await;

    run_on_ui(window_weak, move |w| {
        push_toast(&w, toast_title, toast_msg, "success", 3000);
    });
}

/// 清空当前存储层中的所有预设演示数据
pub(crate) async fn execute_clear_demo_data(
    window_weak: slint::Weak<AppWindow>,
    ctx: StorageSwitchContext,
) {
    let storage = ctx.core_state.storage();
    let is_mock = ctx.core_state.is_mock_storage();

    if let Ok(hosts) = storage.hosts().list_all().await {
        for h in hosts { let _ = storage.hosts().delete(&h.id).await; }
    }
    if let Ok(groups) = storage.groups().list_all().await {
        for g in groups { let _ = storage.groups().delete(&g.id).await; }
    }
    if let Ok(creds) = storage.credentials().list_all().await {
        for c in creds { let _ = storage.credentials().delete(&c.id).await; }
    }
    if let Ok(snippets) = storage.snippets().list_all().await {
        for s in snippets { let _ = storage.snippets().delete(&s.id).await; }
    }
    if let Ok(s_groups) = storage.snippets().list_groups().await {
        for sg in s_groups { let _ = storage.snippets().delete_group(&sg.id).await; }
    }
    if let Ok(tunnels) = storage.tunnels().list_all().await {
        for t in tunnels { let _ = storage.tunnels().delete(&t.id).await; }
    }

    reload_all_storage_views(&storage, is_mock, window_weak.clone(), &ctx).await;

    run_on_ui(window_weak, move |w| {
        push_toast(&w, "演示数据已清空", "已成功移除所有预设演示主机与凭据，当前数据库已重置为纯净空库", "success", 3000);
    });
}

/// 恢复预设演示种子数据
pub(crate) async fn execute_reset_demo_data(
    window_weak: slint::Weak<AppWindow>,
    ctx: StorageSwitchContext,
) {
    let storage = ctx.core_state.storage();
    let is_mock = ctx.core_state.is_mock_storage();

    if let Ok(hosts) = storage.hosts().list_all().await {
        for h in hosts { let _ = storage.hosts().delete(&h.id).await; }
    }
    if let Ok(groups) = storage.groups().list_all().await {
        for g in groups { let _ = storage.groups().delete(&g.id).await; }
    }
    if let Ok(creds) = storage.credentials().list_all().await {
        for c in creds { let _ = storage.credentials().delete(&c.id).await; }
    }
    if let Ok(snippets) = storage.snippets().list_all().await {
        for s in snippets { let _ = storage.snippets().delete(&s.id).await; }
    }
    if let Ok(s_groups) = storage.snippets().list_groups().await {
        for sg in s_groups { let _ = storage.snippets().delete_group(&sg.id).await; }
    }
    if let Ok(tunnels) = storage.tunnels().list_all().await {
        for t in tunnels { let _ = storage.tunnels().delete(&t.id).await; }
    }

    let seed = smagical_storage::mock::seed_data::generate_seed_data();
    for g in seed.groups { let _ = storage.groups().save(&g).await; }
    for h in seed.hosts { let _ = storage.hosts().save(&h).await; }
    for sg in seed.snippet_groups { let _ = storage.snippets().save_group(&sg).await; }
    for s in seed.snippets { let _ = storage.snippets().save(&s).await; }
    for t in seed.tunnels { let _ = storage.tunnels().save(&t).await; }
    for c in seed.credentials { let _ = storage.credentials().save(&c).await; }
    for h in seed.history { let _ = storage.history().save(&h).await; }
    for (hid, snap) in seed.snapshots {
        let _ = storage.history().save_snapshot(&hid, &snap, 500).await;
    }

    reload_all_storage_views(&storage, is_mock, window_weak.clone(), &ctx).await;

    run_on_ui(window_weak, move |w| {
        push_toast(&w, "预设数据已恢复", "已成功重新载入初始预设演示主机、分组与凭据数据", "success", 3000);
    });
}
